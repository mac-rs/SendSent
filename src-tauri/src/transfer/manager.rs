use crate::events::TransferEvent;
use crate::proto::frame::read_control;
use crate::proto::messages::*;
use crate::store::Identity;
use crate::transfer::receiver::{Decision, run_receiver};
use crate::transfer::sender::run_sender;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, Mutex};
use uuid::Uuid;

struct SessionChannels {
    decision_tx: Option<oneshot::Sender<Decision>>,
    data_tx: Option<oneshot::Sender<tokio::net::TcpStream>>,
}

pub struct SessionManager {
    our: Identity,
    events_tx: mpsc::UnboundedSender<TransferEvent>,
    pending: Mutex<HashMap<Uuid, SessionChannels>>,
}

impl SessionManager {
    pub fn new(our: Identity, events_tx: mpsc::UnboundedSender<TransferEvent>) -> Arc<Self> {
        Arc::new(Self { our, events_tx, pending: Mutex::new(HashMap::new()) })
    }

    pub async fn run_listener(self: Arc<Self>, port: u16) -> anyhow::Result<()> {
        let listener = match TcpListener::bind(("0.0.0.0", port)).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("listener bind failed on port {port}: {e}");
                return Err(e.into());
            }
        };
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let me = self.clone();
                    tokio::spawn(async move { let _ = me.handle_incoming(stream).await; });
                }
                Err(e) => { tracing::warn!("accept error: {e}"); continue; }
            }
        }
    }

    async fn handle_incoming(self: Arc<Self>, mut stream: tokio::net::TcpStream) -> anyhow::Result<()> {
        let peer = stream.peer_addr().ok();
        let (ty, buf) = read_control(&mut stream).await?;
        tracing::info!("incoming connection from {peer:?}: first frame {ty:?}");
        match ty {
            MsgType::Hello => {
                let hello: Hello = bincode::deserialize(&buf)?;
                tracing::info!("Hello from '{}' session {}", hello.name, hello.session_id);
                let session_id = hello.session_id;
                let (dtx, drx) = oneshot::channel::<Decision>();
                let (xtx, xrx) = oneshot::channel::<tokio::net::TcpStream>();
                self.pending.lock().await.insert(session_id, SessionChannels {
                    decision_tx: Some(dtx), data_tx: Some(xtx),
                });
                let events = self.events_tx.clone();
                let our = self.our.clone();
                tokio::spawn(async move {
                    let _ = run_receiver(stream, hello, events, drx, xrx, our).await;
                });
                Ok(())
            }
            MsgType::DataOpen => {
                let d: DataOpen = bincode::deserialize(&buf)?;
                let mut guard = self.pending.lock().await;
                if let Some(ch) = guard.get_mut(&d.session_id)
                    && let Some(xtx) = ch.data_tx.take() {
                        let _ = xtx.send(stream);
                    }
                Ok(())
            }
            other => Err(anyhow::anyhow!("unexpected first frame {other:?}")),
        }
    }

    pub async fn respond(&self, session_id: Uuid, accept: bool, save_dir: PathBuf) -> anyhow::Result<()> {
        let mut guard = self.pending.lock().await;
        if let Some(ch) = guard.get_mut(&session_id)
            && let Some(dtx) = ch.decision_tx.take() {
                let _ = dtx.send(Decision { accept, save_dir });
            }
        Ok(())
    }

    pub async fn cleanup_session(&self, session_id: Uuid) {
        self.pending.lock().await.remove(&session_id);
    }

    pub fn start_send(&self, peer_addrs: Vec<SocketAddr>, files: Vec<String>) -> anyhow::Result<Uuid> {
        let session_id = Uuid::new_v4();
        let our = self.our.clone();
        let events = self.events_tx.clone();
        let id = session_id;
        tokio::spawn(async move {
            let _ = run_sender(id, peer_addrs, files, our, events).await;
        });
        Ok(session_id)
    }
}
