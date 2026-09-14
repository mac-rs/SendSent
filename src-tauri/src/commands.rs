use crate::discovery::Peer;
use crate::state::AppState;
use std::net::SocketAddr;
use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn get_identity(state: State<'_, AppState>) -> crate::store::Identity {
    state.identity.lock().unwrap().clone()
}

#[tauri::command]
pub async fn set_display_name(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("名称不能为空".into());
    }
    {
        let mut id = state.identity.lock().map_err(|e| e.to_string())?;
        id.name = name.clone();
        if let Ok(s) = serde_json::to_string_pretty(&*id) {
            let _ = std::fs::write(state.identity_dir.join("identity.json"), s);
        }
    }
    // 让广播也用新名字,其他设备无需重启即可看到。
    state.discovery.set_display_name(&name).await.map_err(|e| e.to_string())
}

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
#[allow(unused_variables)]
pub async fn send_files(app: tauri::AppHandle, state: State<'_, AppState>, peer_device_id: String, files: Vec<String>, secure: bool, verify: bool) -> Result<Uuid, String> {
    // iOS hands back plain paths; Android may pass content:// URIs (handled below).
    let files: Vec<String> = files;
    // Android: content:// URIs can't be opened with std::fs. Open the underlying
    // file descriptor via the ContentPlugin and expose it as `/proc/self/fd/<fd>`
    // through a per-file symlink that preserves the original file name. This
    // avoids copying (large videos stay in place).
    #[cfg(target_os = "android")]
    let files = {
        use tauri::Manager;
        if files.iter().any(|f| f.starts_with("content://")) {
            let base = std::env::temp_dir().join("sendsent-fds");
            let _ = std::fs::create_dir_all(&base);
            let mut out = Vec::with_capacity(files.len());
            for f in files {
                if f.starts_with("content://") {
                    let content = app.state::<crate::content_plugin::Content<tauri::Wry>>();
                    let display_name = content.display_name(&f).unwrap_or_else(|_| "file".to_string());
                    let fd = content.open_fd(&f)?;
                    let safe: String = display_name
                        .chars()
                        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
                        .collect();
                    let safe = if safe.is_empty() { "file".to_string() } else { safe };
                    let dir = base.join(Uuid::new_v4().to_string());
                    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
                    let link = dir.join(&safe);
                    std::os::unix::fs::symlink(format!("/proc/self/fd/{fd}"), &link)
                        .map_err(|e| format!("symlink: {e}"))?;
                    out.push(link.to_string_lossy().into_owned());
                } else {
                    out.push(f);
                }
            }
            out
        } else {
            files
        }
    };
    let peers = state.discovery.peers().await;
    let n = peers.len();
    let Some(p) = peers.into_iter().find(|x| x.device_id == peer_device_id) else {
        tracing::warn!("send_files: peer {peer_device_id} not found ({n} peers known)");
        return Err("peer not found".into());
    };
    tracing::info!("send_files → '{}' addrs={:?} port={} files={}", p.name, p.addrs, p.port, files.len());
    tracing::debug!("send_files paths: {:?}", files);
    state.sessions.start_send(p, files, state.transfer_config.lock().unwrap().clone(), secure, verify).map_err(|e| e.to_string())
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
    state.sessions.start_send(p, vec![file], state.transfer_config.lock().unwrap().clone(), secure, verify).map_err(|e| e.to_string())
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
pub fn get_transfer_config(state: State<'_, AppState>) -> Result<crate::store::TransferConfig, String> {
    Ok(state.transfer_config.lock().unwrap().clone())
}

#[tauri::command]
pub fn set_transfer_config(state: State<'_, AppState>, conns: u32, chunk_kb: u64, split_mb: u64, zerocopy: bool) -> Result<(), String> {
    // 就地更新内存配置 + 落盘,这样无需重启即可生效。
    let sanitized = {
        let mut cfg = state.transfer_config.lock().map_err(|e| e.to_string())?;
        cfg.conns = conns;
        cfg.chunk_size = chunk_kb * 1024;
        cfg.split_threshold = split_mb * 1024 * 1024;
        cfg.zerocopy = zerocopy;
        let s = cfg.clone().sanitized();
        *cfg = s.clone();
        s
    };
    let p = state.identity_dir.join("transfer.json");
    let s = serde_json::to_string_pretty(&sanitized).map_err(|e| e.to_string())?;
    std::fs::write(&p, s).map_err(|e| format!("write transfer.json: {e}"))?;
    tracing::info!("transfer config updated: conns={} chunk={} split={} zerocopy={}", sanitized.conns, sanitized.chunk_size, sanitized.split_threshold, sanitized.zerocopy);
    Ok(())
}

#[tauri::command]
pub fn get_default_save_dir(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.save_dir.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn list_transfer_history(state: State<'_, AppState>) -> Result<Vec<crate::history::HistoryRecord>, String> {
    Ok(state.history.lock().await.list())
}

#[tauri::command]
pub async fn clear_transfer_history(state: State<'_, AppState>) -> Result<(), String> {
    state.history.lock().await.clear();
    Ok(())
}

// Android 上 dialog.open() 首次不 resolve 的已知问题(tauri plugins-workspace
// #3366):打开系统选择器期间需要周期性调用一个命令,保持前后端 IPC 通道活跃。
#[tauri::command]
pub fn noop() {}

// ── 我的设备信息 · QR / IP 列表 ─────────────────────

/// 返回本机的内网 IP 列表(枚举所有非 loopback、非 link-local 的 IPv4 接口)
#[tauri::command]
pub fn get_my_addresses() -> Result<Vec<crate::misc::MyAddress>, String> {
    crate::misc::get_my_addresses()
}

/// 返回本机 QR PNG(base64 字符串),payload 为 sendsent:// 协议
/// - `size`: PNG 边长像素 (128..=1024)
/// - `ip`: 可选 — 显式指定要写入 payload 的 IP;默认取枚举出来的第一个
#[tauri::command]
pub fn get_my_qr(state: State<'_, AppState>, size: Option<u32>, ip: Option<String>) -> Result<String, String> {
    let name = state.identity.lock().map_err(|e| e.to_string())?.name.clone();
    crate::misc::my_qr_base64(
        &name,
        state.port,
        &state.save_dir.to_string_lossy(),
        size,
        ip,
    )
}


