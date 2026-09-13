#![cfg(feature = "tauri-shell")]

use crate::discovery::Discovery;
use crate::history::HistoryStore;
use crate::store::{Identity, TransferConfig};
use crate::transfer::manager::SessionManager;
use crate::transfer::tls::TlsConfig;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub identity: Identity,
    pub identity_dir: PathBuf,
    pub discovery: Arc<dyn Discovery>,
    pub sessions: Arc<SessionManager>,
    pub save_dir: PathBuf,
    pub transfer_config: TransferConfig,
    pub tls_config: TlsConfig,
    pub history: Arc<Mutex<HistoryStore>>,
    pub port: u16,
}
