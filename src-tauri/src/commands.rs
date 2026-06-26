use crate::discovery::Peer;
use crate::state::AppState;
use std::net::SocketAddr;
use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn get_identity(state: State<'_, AppState>) -> crate::store::Identity { state.identity.clone() }

#[tauri::command]
pub async fn set_display_name(_state: State<'_, AppState>, _name: String) -> Result<(), String> { Ok(()) }

#[tauri::command]
pub async fn list_peers(state: State<'_, AppState>) -> Result<Vec<Peer>, String> {
    Ok(state.discovery.peers().await)
}

#[tauri::command]
pub async fn add_peer(state: State<'_, AppState>, address: String) -> Result<(), String> {
    let addr: SocketAddr = address.parse().map_err(|e| format!("invalid address: {e}"))?;
    state.discovery.add_manual_peer(addr).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_files(app: tauri::AppHandle, state: State<'_, AppState>, peer_device_id: String, files: Vec<String>, secure: bool, verify: bool) -> Result<Uuid, String> {
    // Android: 文件选择器返回 content:// URI,Rust std::fs 无法直接打开。
    // 用 tauri-plugin-fs 的 FsExt(内部走 Kotlin ContentResolver.getFileDescriptor)
    // 把文件流式复制到临时目录,再用临时路径走原传输流程。
    let files = if files.iter().any(|f| f.starts_with("content://")) {
        tauri::async_runtime::spawn_blocking(move || -> Result<Vec<String>, String> {
            use tauri_plugin_fs::{FsExt, FilePath, OpenOptions};
            #[cfg(target_os = "android")]
            use tauri::Manager;
            let mut out = Vec::with_capacity(files.len());
            for f in files {
                if f.starts_with("content://") {
                    let fp: FilePath = f.parse().map_err(|_: std::convert::Infallible| "parse uri".to_string())?;
                    let mut opts = OpenOptions::new();
                    opts.read(true);
                    let mut src = app.fs().open(fp, opts).map_err(|e| format!("open {f}: {e}"))?;
                    // 查询原始文件名(Android),用它命名临时文件,保证接收端文件名正确
                    #[cfg(target_os = "android")]
                    let display_name = app
                        .try_state::<crate::content_plugin::Content<tauri::Wry>>()
                        .map(|c| c.display_name(&f).unwrap_or_else(|_| "file".to_string()))
                        .unwrap_or_else(|| "file".to_string());
                    #[cfg(not(target_os = "android"))]
                    let display_name = "file".to_string();
                    let safe = display_name.chars().map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' }).collect::<String>();
                    let dir = std::env::temp_dir().join(format!("sendsent-{}", Uuid::new_v4()));
                    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
                    let cache = dir.join(if safe.is_empty() { "file".to_string() } else { safe });
                    let mut dst = std::fs::File::create(&cache).map_err(|e| format!("create temp: {e}"))?;
                    std::io::copy(&mut src, &mut dst).map_err(|e| format!("copy: {e}"))?;
                    tracing::info!("cached content uri {f} -> {}", cache.display());
                    out.push(cache.to_string_lossy().into_owned());
                } else {
                    out.push(f);
                }
            }
            Ok(out)
        }).await.map_err(|e| e.to_string())??
    } else {
        files
    };
    let peers = state.discovery.peers().await;
    let n = peers.len();
    let Some(p) = peers.into_iter().find(|x| x.device_id == peer_device_id) else {
        tracing::warn!("send_files: peer {peer_device_id} not found ({n} peers known)");
        return Err("peer not found".into());
    };
    tracing::info!("send_files → '{}' addrs={:?} port={} files={}", p.name, p.addrs, p.port, files.len());
    tracing::debug!("send_files paths: {:?}", files);
    state.sessions.start_send(p.addrs, files, state.transfer_config.clone(), secure, verify).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_text(state: State<'_, AppState>, peer_device_id: String, text: String, secure: bool, verify: bool) -> Result<Uuid, String> {
    let dir = std::env::temp_dir().join("sendsent-text");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{}.txt", Uuid::new_v4()));
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    let file = path.to_string_lossy().into_owned();
    let peers = state.discovery.peers().await;
    let Some(p) = peers.into_iter().find(|x| x.device_id == peer_device_id) else {
        return Err("peer not found".into());
    };
    state.sessions.start_send(p.addrs, vec![file], state.transfer_config.clone(), secure, verify).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn respond(state: State<'_, AppState>, session_id: Uuid, accept: bool, save_dir: Option<String>, pin: Option<String>) -> Result<(), String> {
    let dir = match save_dir {
        Some(d) => PathBuf::from(d),
        None => state.save_dir.clone(),
    };
    state.sessions.respond(session_id, accept, dir, pin).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel(_state: State<'_, AppState>, _session_id: Uuid) -> Result<(), String> { Ok(()) }

#[tauri::command]
pub fn get_default_save_dir(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.save_dir.to_string_lossy().into_owned())
}

// ── iOS 原生文档选择器 ──

#[cfg(target_os = "ios")]
pub(crate) mod ios_picker {
    use std::os::raw::c_char;
    use std::sync::Mutex;
    use tokio::sync::oneshot;

    static PICKER_STATE: Mutex<Option<oneshot::Sender<Vec<String>>>> = Mutex::new(None);

    unsafe extern "C" {
        fn sendsent_pick_files(cb: extern "C" fn(*const c_char));
    }

    extern "C" fn picker_result(ptr: *const c_char) {
        let mut guard = PICKER_STATE.lock().unwrap();
        if let Some(tx) = guard.take() {
            let files = if ptr.is_null() {
                vec![]
            } else {
                let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
                let s = c_str.to_string_lossy();
                serde_json::from_str(&s).unwrap_or_default()
            };
            let _ = tx.send(files);
        }
    }

    #[tauri::command]
    pub async fn pick_files_ios() -> Result<Vec<String>, String> {
        let (tx, rx) = oneshot::channel::<Vec<String>>();
        *PICKER_STATE.lock().unwrap() = Some(tx);
        unsafe { sendsent_pick_files(picker_result); }
        rx.await.map_err(|e| e.to_string())
    }
}

#[cfg(not(target_os = "ios"))]
pub(crate) mod ios_picker {
    #[tauri::command]
    pub async fn pick_files_ios() -> Result<Vec<String>, String> {
        Err("iOS only".into())
    }
}
