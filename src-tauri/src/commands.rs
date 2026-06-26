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
pub async fn send_files(state: State<'_, AppState>, peer_device_id: String, files: Vec<String>, secure: bool, verify: bool) -> Result<Uuid, String> {
    let peers = state.discovery.peers().await;
    let n = peers.len();
    let Some(p) = peers.into_iter().find(|x| x.device_id == peer_device_id) else {
        tracing::warn!("send_files: peer {peer_device_id} not found ({n} peers known)");
        return Err("peer not found".into());
    };
    tracing::info!("send_files → '{}' addrs={:?} port={} files={}", p.name, p.addrs, p.port, files.len());
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
