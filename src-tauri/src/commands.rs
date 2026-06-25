use crate::discovery::Peer;
use crate::state::AppState;
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
pub async fn send_files(state: State<'_, AppState>, peer_device_id: String, files: Vec<String>) -> Result<Uuid, String> {
    let peers = state.discovery.peers().await;
    let n = peers.len();
    let Some(p) = peers.into_iter().find(|x| x.device_id == peer_device_id) else {
        tracing::warn!("send_files: peer {peer_device_id} not found ({n} peers known)");
        return Err("peer not found".into());
    };
    tracing::info!("send_files → '{}' addrs={:?} port={} files={}", p.name, p.addrs, p.port, files.len());
    state.sessions.start_send(p.addrs, files).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn respond(state: State<'_, AppState>, session_id: Uuid, accept: bool, save_dir: Option<String>) -> Result<(), String> {
    let dir = match save_dir {
        Some(d) => PathBuf::from(d),
        None => crate::store::default_save_dir().map_err(|e| e.to_string())?,
    };
    state.sessions.respond(session_id, accept, dir).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel(_state: State<'_, AppState>, _session_id: Uuid) -> Result<(), String> { Ok(()) }

#[tauri::command]
pub fn get_default_save_dir() -> Result<String, String> {
    crate::store::default_save_dir().map(|p| p.to_string_lossy().into_owned()).map_err(|e| e.to_string())
}
