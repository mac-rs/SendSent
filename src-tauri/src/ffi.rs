//! iOS 原生 FFI：SwiftUI 通过 C ABI 调用 Rust 核心。
#![cfg(target_os = "ios")]

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::mpsc;

use crate::discovery::ios_bonjour::BonjourDiscovery;
use crate::discovery::{Discovery, PeerEvent};
use crate::events::TransferEvent;
use crate::history::HistoryStore;
use crate::store::{self, Identity, TransferConfig};
use crate::transfer::manager::SessionManager;
use crate::transfer::tls::TlsConfig;

type EventCb = extern "C" fn(*const c_char);

pub struct Core {
    pub identity: Mutex<Identity>,
    pub data_dir: PathBuf,
    pub save_dir: PathBuf,
    pub port: u16,
    pub discovery: std::sync::Arc<dyn Discovery>,
    pub sessions: std::sync::Arc<SessionManager>,
    pub history: std::sync::Arc<AsyncMutex<HistoryStore>>,
    pub config: Mutex<TransferConfig>,
    pub runtime: tokio::runtime::Runtime,
    pub event_cb: Mutex<Option<EventCb>>,
}

static CORE: OnceLock<Core> = OnceLock::new();

fn core() -> Option<&'static Core> {
    CORE.get()
}

fn emit(core: &Core, json: String) {
    let cb = *core.event_cb.lock().unwrap();
    if let Some(cb) = cb {
        if let Ok(c) = CString::new(json) {
            cb(c.as_ptr());
        }
    }
}

unsafe fn cstr(ptr: *const c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Err("null pointer".into());
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(|s| s.to_owned())
        .map_err(|e| e.to_string())
}

fn to_c(s: String) -> *mut c_char {
    CString::new(s).map(|c| c.into_raw()).unwrap_or(std::ptr::null_mut())
}

fn err(msg: impl Into<String>) -> *mut c_char {
    to_c(serde_json::json!({ "error": msg.into() }).to_string())
}

/// 统一释放 FFI 返回的字符串。
#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(unsafe { CString::from_raw(ptr) });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_init(
    data_dir: *const c_char,
    save_dir: *const c_char,
    port: u16,
    event_cb: Option<EventCb>,
) -> i32 {
    if CORE.get().is_some() {
        return 0;
    }
    let data_dir = match unsafe { cstr(data_dir) } {
        Ok(s) => PathBuf::from(s),
        Err(_) => return 1,
    };
    let save_dir = match unsafe { cstr(save_dir) } {
        Ok(s) => PathBuf::from(s),
        Err(_) => return 2,
    };
    if std::fs::create_dir_all(&data_dir).is_err() {
        return 3;
    }
    let _ = std::fs::create_dir_all(&save_dir);

    let _ = rustls::crypto::ring::default_provider().install_default();
    let platform = store::current_platform();
    let identity = match store::load_or_create(&data_dir, platform, "iPhone") {
        Ok(i) => i,
        Err(_) => return 4,
    };
    let config = store::load_or_create_transfer_config(&data_dir);
    let tls: TlsConfig = match crate::transfer::tls::load_or_generate_tls_config(&data_dir) {
        Ok(t) => t,
        Err(_) => return 5,
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(r) => r,
        Err(_) => return 6,
    };

    let (ptx, prx) = mpsc::unbounded_channel::<PeerEvent>();
    let discovery: std::sync::Arc<dyn Discovery> =
        std::sync::Arc::new(BonjourDiscovery::new(identity.clone(), port, ptx));
    let (ttx, trx) = mpsc::unbounded_channel::<TransferEvent>();
    let sessions = SessionManager::new(identity.clone(), ttx, tls);
    let history = std::sync::Arc::new(AsyncMutex::new(HistoryStore::load(&data_dir)));

    let core = Core {
        identity: Mutex::new(identity),
        data_dir,
        save_dir,
        port,
        discovery: discovery.clone(),
        sessions: sessions.clone(),
        history,
        config: Mutex::new(config),
        runtime,
        event_cb: Mutex::new(event_cb),
    };
    if CORE.set(core).is_err() {
        return 0;
    }
    let core = CORE.get().unwrap();

    spawn_tasks(core, discovery, sessions, prx, trx, port);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_identity() -> *mut c_char {
    match core() {
        Some(c) => to_c(serde_json::to_string(&*c.identity.lock().unwrap()).unwrap()),
        None => err("not initialized"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_peers() -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let peers = c.runtime.block_on(c.discovery.peers());
    to_c(serde_json::to_string(&peers).unwrap())
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_addresses() -> *mut c_char {
    match crate::misc::get_my_addresses() {
        Ok(a) => to_c(serde_json::to_string(&a).unwrap()),
        Err(e) => err(e),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_qr(size: u32, ip: *const c_char) -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let ip = if ip.is_null() { None } else { unsafe { cstr(ip) }.ok() };
    let name = c.identity.lock().unwrap().name.clone();
    let save = c.save_dir.to_string_lossy().to_string();
    match crate::misc::my_qr_base64(&name, c.port, &save, Some(size), ip) {
        Ok(b64) => to_c(serde_json::json!(b64).to_string()),
        Err(e) => err(e),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_add_peer(addr: *const c_char) -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let addr = match unsafe { cstr(addr) } { Ok(s) => s, Err(e) => return err(e) };
    let sock: std::net::SocketAddr = match addr.parse() {
        Ok(s) => s,
        Err(e) => return err(format!("invalid address: {e}")),
    };
    match c.runtime.block_on(c.discovery.add_manual_peer(sock)) {
        Ok(()) => std::ptr::null_mut(),
        Err(e) => err(e.to_string()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_send(
    peer_id: *const c_char,
    files_json: *const c_char,
    secure: bool,
    verify: bool,
) -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let peer_id = match unsafe { cstr(peer_id) } { Ok(s) => s, Err(e) => return err(e) };
    let files_json = match unsafe { cstr(files_json) } { Ok(s) => s, Err(e) => return err(e) };
    let files: Vec<String> = match serde_json::from_str(&files_json) {
        Ok(f) => f,
        Err(e) => return err(format!("bad files json: {e}")),
    };
    let peers = c.runtime.block_on(c.discovery.peers());
    let Some(peer) = peers.into_iter().find(|p| p.device_id == peer_id) else {
        return err("peer not found");
    };
    let cfg = c.config.lock().unwrap().clone();
    match c.sessions.start_send(peer, files, cfg, secure, verify) {
        Ok(id) => to_c(serde_json::json!({ "session_id": id.to_string() }).to_string()),
        Err(e) => err(e.to_string()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_respond(session_id: *const c_char, accept: bool) -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let sid = match unsafe { cstr(session_id) } { Ok(s) => s, Err(e) => return err(e) };
    let uuid = match uuid::Uuid::parse_str(&sid) {
        Ok(u) => u,
        Err(e) => return err(format!("bad session id: {e}")),
    };
    let save = c.save_dir.clone();
    match c.runtime.block_on(c.sessions.respond(uuid, accept, save, None)) {
        Ok(()) => std::ptr::null_mut(),
        Err(e) => err(e.to_string()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_history() -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let items = c.runtime.block_on(async { c.history.lock().await.list() });
    to_c(serde_json::to_string(&items).unwrap())
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_clear_history() -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    c.runtime.block_on(async { c.history.lock().await.clear() });
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_delete_history(session_id: *const c_char) -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let sid = match unsafe { cstr(session_id) } { Ok(s) => s, Err(e) => return err(e) };
    c.runtime.block_on(async { c.history.lock().await.remove(&sid) });
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_get_transfer_config() -> *mut c_char {
    match core() {
        Some(c) => to_c(serde_json::to_string(&*c.config.lock().unwrap()).unwrap()),
        None => err("not initialized"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_set_transfer_config(conns: u32, chunk_kb: u64, split_mb: u64) -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let sanitized = {
        let mut cfg = c.config.lock().unwrap();
        cfg.conns = conns;
        cfg.chunk_size = chunk_kb * 1024;
        cfg.split_threshold = split_mb * 1024 * 1024;
        let s = cfg.clone().sanitized();
        *cfg = s.clone();
        s
    };
    let path = c.data_dir.join("transfer.json");
    match serde_json::to_string_pretty(&sanitized) {
        Ok(s) => {
            if let Err(e) = std::fs::write(&path, s) {
                return err(format!("write transfer.json: {e}"));
            }
            std::ptr::null_mut()
        }
        Err(e) => err(e.to_string()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sendsent_ios_set_display_name(name: *const c_char) -> *mut c_char {
    let Some(c) = core() else { return err("not initialized") };
    let name = match unsafe { cstr(name) } { Ok(s) => s, Err(e) => return err(e) };
    let name = name.trim().to_string();
    if name.is_empty() {
        return err("empty name");
    }
    {
        let mut id = c.identity.lock().unwrap();
        id.name = name.clone();
        let path = c.data_dir.join("identity.json");
        if let Ok(s) = serde_json::to_string_pretty(&*id) {
            let _ = std::fs::write(&path, s);
        }
    }
    match c.runtime.block_on(c.discovery.set_display_name(&name)) {
        Ok(()) => std::ptr::null_mut(),
        Err(e) => err(e.to_string()),
    }
}

pub(crate) fn spawn_tasks(
    core: &'static Core,
    discovery: std::sync::Arc<dyn Discovery>,
    sessions: std::sync::Arc<SessionManager>,
    mut prx: mpsc::UnboundedReceiver<PeerEvent>,
    mut trx: mpsc::UnboundedReceiver<TransferEvent>,
    port: u16,
) {
    // discovery
    core.runtime.spawn(async move {
        if let Err(e) = discovery.start().await {
            tracing::error!("discovery start failed: {e}");
        }
    });

    // peer events → callback
    core.runtime.spawn(async move {
        while let Some(ev) = prx.recv().await {
            let json = match ev {
                PeerEvent::Found(p) => serde_json::json!({ "kind": "peer_found", "peer": p }).to_string(),
                PeerEvent::Lost(id) => serde_json::json!({ "kind": "peer_lost", "device_id": id }).to_string(),
            };
            emit(core, json);
        }
    });

    // listener
    let listener_sessions = sessions.clone();
    core.runtime.spawn(async move {
        if let Err(e) = listener_sessions.run_listener(port).await {
            tracing::error!("listener failed: {e}");
        }
    });

    // transfer events → callback（progress 节流；Finished/Recorded 必发）
    core.runtime.spawn(async move {
        let mut throttle = crate::misc::Throttle::new(std::time::Duration::from_millis(100));
        while let Some(ev) = trx.recv().await {
            if let TransferEvent::Progress { session_id, .. } = &ev {
                if !throttle.allow_at(*session_id, std::time::Instant::now()) {
                    continue;
                }
            }
            if let TransferEvent::Finished { session_id, .. } = &ev {
                sessions.cleanup_session(*session_id).await;
            }
            let json = serde_json::to_string(&ev).unwrap_or_else(|e| {
                serde_json::json!({ "kind": "error", "message": e.to_string() }).to_string()
            });
            emit(core, json);
        }
    });
}
