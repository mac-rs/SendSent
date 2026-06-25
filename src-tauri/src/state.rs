use crate::discovery::Discovery;
use crate::store::Identity;
use crate::transfer::manager::SessionManager;
use std::path::PathBuf;
use std::sync::Arc;

pub struct AppState {
    pub identity: Identity,
    pub discovery: Arc<dyn Discovery>,
    pub sessions: Arc<SessionManager>,
    pub save_dir: PathBuf,
}
