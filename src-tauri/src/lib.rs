pub mod proto;
pub mod discovery;
pub mod transfer;
pub mod store;
pub mod events;
pub mod state;
pub mod commands;

use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;

use discovery::{mdns::MdnsDiscovery, Discovery, PeerEvent};
use events::{name, TransferEvent};
use state::AppState;
use store::{default_save_dir, load_or_create};
use transfer::manager::SessionManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();
    rustls::crypto::ring::default_provider().install_default().expect("ring provider");
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir().expect("app_data_dir");
            let platform = if cfg!(target_os = "macos") { "macos" }
                else if cfg!(target_os = "windows") { "windows" }
                else if cfg!(target_os = "linux") { "linux" }
                else { "unknown" };
            let host = hostname().unwrap_or_else(|| "device".into());
            let identity = load_or_create(&data_dir, platform, &host).expect("identity");
            // Default 52225; the iOS SIMULATOR gets 52226 automatically (it shares the Mac's
            // network stack, so it must not collide with a Mac app also binding 52225).
            // Overridable in all cases via SENDSENT_PORT.
            let default_port: u16 = if cfg!(target_abi = "sim") { 52226 } else { 52225 };
            let port: u16 = std::env::var("SENDSENT_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(default_port);

            let handle = app.handle().clone();

            // Resolve a writable save directory. On iOS the sandbox $HOME is read-only, so use the
            // app's Documents container (writable + visible in the Files app). Desktop uses ~/Downloads.
            let save_dir = if cfg!(target_os = "ios") {
                app.path().document_dir()
                    .map(|d| d.join("sendsent"))
                    .unwrap_or_else(|_| default_save_dir().unwrap_or_default())
            } else {
                default_save_dir().unwrap_or_else(|_| app.path().document_dir().unwrap_or_default().join("sendsent"))
            };
            let _ = std::fs::create_dir_all(&save_dir);
            tracing::info!("save_dir = {}", save_dir.display());
            let transfer_config = store::load_or_create_transfer_config(&data_dir);
            tracing::info!("transfer_config: conns={} chunk={} split={}",
                transfer_config.conns, transfer_config.chunk_size, transfer_config.split_threshold);
            let tls_config = crate::transfer::tls::load_or_generate_tls_config(&data_dir)
                .expect("tls config");

            let (ptx, mut prx) = mpsc::unbounded_channel::<PeerEvent>();
            let discovery: Arc<dyn Discovery> = Arc::new(MdnsDiscovery::new(identity.clone(), port, ptx));
            {
                let d = discovery.clone();
                tauri::async_runtime::spawn(async move { let _ = d.start().await; });
            }
            {
                let h = handle.clone();
                tauri::async_runtime::spawn(async move {
                    while let Some(ev) = prx.recv().await {
                        match ev {
                            PeerEvent::Found(p) => {
                                let _ = h.emit(name::PEER_FOUND, events::PeerFoundPayload { peer: p });
                            }
                            PeerEvent::Lost(id) => {
                                let _ = h.emit(name::PEER_LOST, events::PeerLostPayload { device_id: id });
                            }
                        }
                    }
                });
            }

            let (ttx, mut trx) = mpsc::unbounded_channel::<TransferEvent>();
            let sessions = SessionManager::new(identity.clone(), ttx, tls_config.clone());
            {
                let s = sessions.clone();
                tauri::async_runtime::spawn(async move { let _ = s.run_listener(port).await; });
            }
            {
                let h = handle.clone();
                let sessions = sessions.clone();
                tauri::async_runtime::spawn(async move {
                    while let Some(ev) = trx.recv().await {
                        let (n, val) = match &ev {
                            TransferEvent::Request { .. } => {
                                (name::TRANSFER_REQUEST, serde_json::to_value(&ev).unwrap())
                            }
                            TransferEvent::Progress { .. } => {
                                (name::TRANSFER_PROGRESS, serde_json::to_value(&ev).unwrap())
                            }
                            TransferEvent::Finished { .. } => {
                                (name::TRANSFER_FINISHED, serde_json::to_value(&ev).unwrap())
                            }
                        };
                        let _ = h.emit(n, val);
                        if let TransferEvent::Finished { session_id, .. } = &ev {
                            sessions.cleanup_session(*session_id).await;
                        }
                    }
                });
            }

            if let Ok(dir) = default_save_dir() {
                let _ = std::fs::create_dir_all(&dir);
            }
            app.manage(AppState { identity, discovery, sessions, save_dir, transfer_config, tls_config });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_identity,
            commands::set_display_name,
            commands::list_peers,
            commands::add_peer,
            commands::send_files,
            commands::send_text,
            commands::respond,
            commands::cancel,
            commands::get_default_save_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn hostname() -> Option<String> {
    std::env::var("HOSTNAME")
        .ok()
        .or_else(|| std::env::var("USER").ok().map(|u| format!("{u}-mac")))
}
