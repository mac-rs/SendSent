pub mod proto;
pub mod discovery;
pub mod transfer;
pub mod store;
pub mod events;
pub mod history;
pub mod state;
pub mod commands;
#[cfg(target_os = "android")]
pub mod content_plugin;
#[cfg(target_os = "android")]
pub mod nsd_plugin;

use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;

#[cfg(all(not(target_os = "ios"), not(target_os = "android")))]
use discovery::mdns::MdnsDiscovery;
#[cfg(target_os = "ios")]
use discovery::ios_bonjour::BonjourDiscovery;
#[cfg(target_os = "android")]
use discovery::nsd::NsdDiscovery;
use discovery::{Discovery, PeerEvent};
use events::{name, TransferEvent};
use state::AppState;
use store::{default_save_dir, load_or_create};
use transfer::manager::SessionManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Android 上 stdout/stderr 不可见:用 android_logger 把日志输出到 logcat。
    // 启用 tracing 的 "log" feature 后,所有 tracing 事件会转发到 log crate → android_logger → logcat。
    // 桌面端仍用 fmt subscriber(终端彩色输出)。
    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Info)
                .with_tag("sendsent"),
        );
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = tracing_subscriber::fmt::try_init();
    }
    rustls::crypto::ring::default_provider().install_default().expect("ring provider");
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init());
    #[cfg(target_os = "android")]
    let builder = builder.plugin(content_plugin::plugin()).plugin(nsd_plugin::plugin());
    builder
        .setup(|app| {
            let data_dir = app.path().app_data_dir().expect("app_data_dir");
            let platform = store::current_platform();
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

            // Resolve a writable save directory. iOS/Android are sandboxed, so use the
            // app's Documents container (writable + visible in the Files app). Desktop uses ~/Downloads.
            let save_dir = if cfg!(target_os = "ios") || cfg!(target_os = "android") {
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
            // iOS can't open raw multicast sockets without a restricted entitlement, so it
            // uses the system Bonjour responder (`zeroconf`); other platforms keep mdns-sd.
            #[cfg(target_os = "ios")]
            let discovery: Arc<dyn Discovery> = Arc::new(BonjourDiscovery::new(identity.clone(), port, ptx));
            #[cfg(target_os = "android")]
            let discovery: Arc<dyn Discovery> = {
                let nsd = app.state::<crate::nsd_plugin::Nsd<tauri::Wry>>().inner().clone();
                Arc::new(NsdDiscovery::new(nsd, identity.clone(), port, ptx))
            };
            #[cfg(all(not(target_os = "ios"), not(target_os = "android")))]
            let discovery: Arc<dyn Discovery> = Arc::new(MdnsDiscovery::new(identity.clone(), port, ptx));
            {
                let d = discovery.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = d.start().await {
                        tracing::error!("discovery start failed: {e}");
                    }
                });
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

            let history = Arc::new(tokio::sync::Mutex::new(crate::history::HistoryStore::load(&data_dir)));
            let (ttx, mut trx) = mpsc::unbounded_channel::<TransferEvent>();
            let sessions = SessionManager::new(identity.clone(), ttx, tls_config.clone());
            {
                let s = sessions.clone();
                tauri::async_runtime::spawn(async move { let _ = s.run_listener(port).await; });
            }
            {
                let h = handle.clone();
                let sessions = sessions.clone();
                let history = history.clone();
                tauri::async_runtime::spawn(async move {
                    while let Some(ev) = trx.recv().await {
                        if let TransferEvent::Recorded(record) = &ev {
                            history.lock().await.append(record.clone());
                            let _ = h.emit(name::TRANSFER_HISTORY, serde_json::to_value(record).unwrap());
                            continue;
                        }
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
                            TransferEvent::Recorded(_) => unreachable!(),
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
            // macOS 系统红绿灯的位置由 tauri.conf.json 的 trafficLightPosition
            // 在启动期设到屏幕外(-100,-100),此处不再运行时调整。
            app.manage(AppState { identity, identity_dir: data_dir, discovery, sessions, save_dir, transfer_config, tls_config, history, port });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_identity,
            commands::set_display_name,
            commands::list_peers,
            commands::add_peer,
            commands::send_files,
            commands::send_text,
            commands::ios_picker::pick_files_ios,
            commands::respond,
            commands::cancel,
            commands::get_default_save_dir,
            commands::list_transfer_history,
            commands::clear_transfer_history,
            commands::noop,
            commands::get_transfer_config,
            commands::set_transfer_config,
            commands::get_my_qr,
            commands::get_my_addresses,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn hostname() -> Option<String> {
    std::env::var("HOSTNAME")
        .ok()
        .or_else(|| std::env::var("USER").ok().map(|u| format!("{u}-mac")))
}
