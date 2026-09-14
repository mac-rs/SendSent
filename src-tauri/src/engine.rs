//! 平台无关的核心引擎：全局 Core + 各操作 + 事件队列/回调。
//! iOS (`ffi.rs`) 与 Android (`jni_bridge.rs`) 都是它的薄包装。
#![allow(dead_code)]

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use tokio::sync::mpsc;
use tokio::sync::Mutex as AsyncMutex;

use crate::discovery::{Discovery, PeerEvent};
use crate::events::TransferEvent;
use crate::history::HistoryStore;
use crate::store::{self, Identity, TransferConfig};
use crate::transfer::manager::SessionManager;
use crate::transfer::tls::TlsConfig;

/// 事件投递：iOS 用 C 回调；Android 无 sink → 入队由 Kotlin 轮询。
pub type EventSink = Box<dyn Fn(String) + Send + Sync>;

pub struct Core {
    pub identity: Mutex<Identity>,
    pub data_dir: PathBuf,
    pub save_dir: PathBuf,
    pub port: u16,
    pub discovery: Arc<dyn Discovery>,
    pub sessions: Arc<SessionManager>,
    pub history: Arc<AsyncMutex<HistoryStore>>,
    pub config: Mutex<TransferConfig>,
    pub runtime: tokio::runtime::Runtime,
    pub sink: Mutex<Option<EventSink>>,
    pub queue: Mutex<VecDeque<String>>,
}

static CORE: OnceLock<Core> = OnceLock::new();
const QUEUE_MAX: usize = 512;

pub fn core() -> Option<&'static Core> {
    CORE.get()
}

pub fn push_event(core: &Core, json: String) {
    {
        let sink = core.sink.lock().unwrap();
        if let Some(f) = sink.as_ref() {
            f(json);
            return;
        }
    }
    let mut q = core.queue.lock().unwrap();
    if q.len() >= QUEUE_MAX {
        q.pop_front();
    }
    q.push_back(json);
}

/// Kotlin 轮询：取出并清空事件，返回 JSON 字符串数组。
pub fn poll_events() -> String {
    let Some(c) = core() else { return "[]".into() };
    let mut q = c.queue.lock().unwrap();
    let v: Vec<String> = q.drain(..).collect();
    serde_json::to_string(&v).unwrap_or_else(|_| "[]".into())
}

pub fn init(data_dir: PathBuf, save_dir: PathBuf, port: u16, sink: Option<EventSink>) -> i32 {
    if CORE.get().is_some() {
        return 0;
    }
    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("sendsent"),
        );
        std::panic::set_hook(Box::new(|info| {
            log::error!("RUST PANIC: {info}");
        }));
    }
    if std::fs::create_dir_all(&data_dir).is_err() {
        return 3;
    }
    let _ = std::fs::create_dir_all(&save_dir);
    let _ = rustls::crypto::ring::default_provider().install_default();

    let identity = match store::load_or_create(&data_dir, store::current_platform(), "device") {
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
    let discovery: Arc<dyn Discovery> = make_discovery(identity.clone(), port, ptx);
    let (ttx, trx) = mpsc::unbounded_channel::<TransferEvent>();
    let sessions = SessionManager::new(identity.clone(), ttx, tls);
    let history = Arc::new(AsyncMutex::new(HistoryStore::load(&data_dir)));

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
        sink: Mutex::new(sink),
        queue: Mutex::new(VecDeque::new()),
    };
    if CORE.set(core).is_err() {
        return 0;
    }
    let core = CORE.get().unwrap();
    spawn_tasks(core, discovery, sessions, prx, trx, port);
    0
}

#[cfg(target_os = "ios")]
fn make_discovery(id: Identity, port: u16, tx: mpsc::UnboundedSender<PeerEvent>) -> Arc<dyn Discovery> {
    Arc::new(crate::discovery::ios_bonjour::BonjourDiscovery::new(id, port, tx))
}
#[cfg(target_os = "android")]
fn make_discovery(id: Identity, port: u16, tx: mpsc::UnboundedSender<PeerEvent>) -> Arc<dyn Discovery> {
    Arc::new(crate::discovery::android_native::AndroidNativeDiscovery::new(id, port, tx))
}
#[cfg(all(not(target_os = "ios"), not(target_os = "android")))]
fn make_discovery(id: Identity, port: u16, tx: mpsc::UnboundedSender<PeerEvent>) -> Arc<dyn Discovery> {
    Arc::new(crate::discovery::mdns::MdnsDiscovery::new(id, port, tx))
}

fn spawn_tasks(
    core: &'static Core,
    discovery: Arc<dyn Discovery>,
    sessions: Arc<SessionManager>,
    mut prx: mpsc::UnboundedReceiver<PeerEvent>,
    mut trx: mpsc::UnboundedReceiver<TransferEvent>,
    port: u16,
) {
    core.runtime.spawn(async move {
        if let Err(e) = discovery.start().await {
            tracing::error!("discovery start failed: {e}");
        }
    });

    core.runtime.spawn(async move {
        while let Some(ev) = prx.recv().await {
            let json = match ev {
                PeerEvent::Found(p) => serde_json::json!({"kind":"peer_found","peer":p}).to_string(),
                PeerEvent::Lost(id) => serde_json::json!({"kind":"peer_lost","device_id":id}).to_string(),
            };
            push_event(core, json);
        }
    });

    let listener_sessions = sessions.clone();
    core.runtime.spawn(async move {
        if let Err(e) = listener_sessions.run_listener(port).await {
            tracing::error!("listener failed: {e}");
        }
    });

    core.runtime.spawn(async move {
        let mut throttle = crate::misc::Throttle::new(std::time::Duration::from_millis(100));
        while let Some(ev) = trx.recv().await {
            if let TransferEvent::Progress { session_id, .. } = &ev
                && !throttle.allow_at(*session_id, std::time::Instant::now())
            {
                continue;
            }
            if let TransferEvent::Finished { session_id, .. } = &ev {
                sessions.cleanup_session(*session_id).await;
            }
            // 落库：桌面 Tauri 的 run() 会做这件事；native 引擎必须自己做，
            // 否则 nativeHistory() 永远为空（Finished 时的 reloadHistory 会覆盖内存列表）。
            if let TransferEvent::Recorded(record) = &ev {
                core.history.lock().await.append(record.clone());
            }
            let json = serde_json::to_string(&ev).unwrap_or_default();
            push_event(core, json);
        }
    });
}

// ── ops ────────────────────────────────────────────────────

pub fn identity_json() -> String {
    serde_json::to_string(&*core().unwrap().identity.lock().unwrap()).unwrap()
}

pub fn peers_json() -> String {
    let c = core().unwrap();
    serde_json::to_string(&c.runtime.block_on(c.discovery.peers())).unwrap()
}

pub fn addresses_json() -> Result<String, String> {
    crate::misc::get_my_addresses().map(|a| serde_json::to_string(&a).unwrap())
}

pub fn qr_json(size: u32, ip: Option<String>) -> Result<String, String> {
    let c = core().ok_or("not initialized")?;
    let name = c.identity.lock().unwrap().name.clone();
    let save = c.save_dir.to_string_lossy().to_string();
    crate::misc::my_qr_base64(&name, c.port, &save, Some(size), ip).map(|b| serde_json::json!(b).to_string())
}

pub fn add_peer(addr: &str) -> Result<(), String> {
    let c = core().ok_or("not initialized")?;
    let sock: std::net::SocketAddr = addr.parse().map_err(|e| format!("invalid address: {e}"))?;
    c.runtime.block_on(c.discovery.add_manual_peer(sock)).map_err(|e| e.to_string())
}

pub fn send(peer_id: &str, files_json: &str, secure: bool, verify: bool) -> Result<String, String> {
    let c = core().ok_or("not initialized")?;
    let files = resolve_files(files_json)?;
    let peers = c.runtime.block_on(c.discovery.peers());
    let peer = peers.into_iter().find(|p| p.device_id == peer_id).ok_or("peer not found")?;
    let cfg = c.config.lock().unwrap().clone();
    // start_send 内部会 tokio::spawn,必须在 runtime 上下文里调用。
    let _guard = c.runtime.enter();
    c.sessions
        .start_send(peer, files, cfg, secure, verify)
        .map(|id| serde_json::json!({ "session_id": id.to_string() }).to_string())
        .map_err(|e| e.to_string())
}

/// Android SAF: `files_json` 可能是 `[{"fd":N,"name":"..."}]`;其它平台/情形是字符串数组。
#[cfg(target_os = "android")]
fn resolve_files(files_json: &str) -> Result<Vec<String>, String> {
    #[derive(serde::Deserialize)]
    struct FdFile {
        fd: i32,
        name: String,
    }
    let parsed: Vec<serde_json::Value> = serde_json::from_str(files_json).map_err(|e| e.to_string())?;
    if parsed.iter().all(|v| v.is_string()) {
        return Ok(parsed.into_iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());
    }
    let fds: Vec<FdFile> = serde_json::from_str(files_json).map_err(|e| e.to_string())?;
    let base = std::env::temp_dir().join("sendsent-fds");
    let _ = std::fs::create_dir_all(&base);
    let mut out = Vec::new();
    for f in fds {
        let safe: String = f
            .name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
            .collect();
        let dir = base.join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let link = dir.join(if safe.is_empty() { "file".to_string() } else { safe });
        std::os::unix::fs::symlink(format!("/proc/self/fd/{}", f.fd), &link).map_err(|e| e.to_string())?;
        let link_str = link.to_string_lossy().into_owned();
        // SELinux 拒绝按路径 open /proc/self/fd;登记 fd 供 misc::open_source 直读。
        crate::misc::register_fd(link_str.clone(), f.fd);
        out.push(link_str);
    }
    Ok(out)
}

#[cfg(not(target_os = "android"))]
fn resolve_files(files_json: &str) -> Result<Vec<String>, String> {
    serde_json::from_str(files_json).map_err(|e| e.to_string())
}

pub fn respond(session_id: &str, accept: bool) -> Result<(), String> {
    let c = core().ok_or("not initialized")?;
    let id = uuid::Uuid::parse_str(session_id).map_err(|e| e.to_string())?;
    let save = c.save_dir.clone();
    c.runtime.block_on(c.sessions.respond(id, accept, save, None)).map_err(|e| e.to_string())
}

pub fn history_json() -> String {
    let c = core().unwrap();
    serde_json::to_string(&c.runtime.block_on(async { c.history.lock().await.list() })).unwrap()
}

pub fn delete_history(id: &str) {
    if let Some(c) = core() {
        c.runtime.block_on(async { c.history.lock().await.remove(id) });
    }
}

pub fn clear_history() {
    if let Some(c) = core() {
        c.runtime.block_on(async { c.history.lock().await.clear() });
    }
}

pub fn get_config_json() -> String {
    serde_json::to_string(&*core().unwrap().config.lock().unwrap()).unwrap()
}

pub fn set_config(conns: u32, chunk_kb: u64, split_mb: u64, zerocopy: bool) -> Result<(), String> {
    let c = core().ok_or("not initialized")?;
    let sanitized = {
        let mut cfg = c.config.lock().unwrap();
        cfg.conns = conns;
        cfg.chunk_size = chunk_kb * 1024;
        cfg.split_threshold = split_mb * 1024 * 1024;
        cfg.zerocopy = zerocopy;
        let s = cfg.clone().sanitized();
        *cfg = s.clone();
        s
    };
    let json = serde_json::to_string_pretty(&sanitized).map_err(|e| e.to_string())?;
    std::fs::write(c.data_dir.join("transfer.json"), json).map_err(|e| e.to_string())
}

pub fn set_display_name(name: &str) -> Result<(), String> {
    let c = core().ok_or("not initialized")?;
    let name = name.trim();
    if name.is_empty() {
        return Err("empty name".into());
    }
    {
        let mut id = c.identity.lock().unwrap();
        id.name = name.to_string();
        if let Ok(s) = serde_json::to_string_pretty(&*id) {
            let _ = std::fs::write(c.data_dir.join("identity.json"), s);
        }
    }
    c.runtime.block_on(c.discovery.set_display_name(name)).map_err(|e| e.to_string())
}

/// iOS 从后台回前台后重新广播自身(iOS 会停掉后台的 Bonjour 注册)。其它平台 no-op。
pub fn reactivate() -> Result<(), String> {
    let Some(c) = core() else { return Ok(()) };
    c.runtime.block_on(c.discovery.reannounce()).map_err(|e| e.to_string())
}

/// Kotlin NsdManager 推来的服务（Android）。
pub fn on_service(name: &str, host: &str, port: u16, txt_json: &str) {
    tracing::info!("on_service {name} {host}:{port} {txt_json}");
    let Some(c) = core() else { return };
    let txt: std::collections::HashMap<String, String> = serde_json::from_str(txt_json).unwrap_or_default();
    c.runtime.block_on(c.discovery.on_service(name, host, port, &txt));
}

pub fn on_service_lost(name: &str) {
    let Some(c) = core() else { return };
    c.runtime.block_on(c.discovery.on_service_lost(name));
}
