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
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, Mutex};
use uuid::Uuid;

pub enum DataStream { Plain(TcpStream), Tls(Box<tokio_rustls::server::TlsStream<TcpStream>>) }

struct SessionChannels {
    decision_tx: Option<oneshot::Sender<Decision>>,
    data_tx: mpsc::Sender<DataStream>,
    secure: bool,
}

pub struct SessionManager {
    our: Identity,
    events_tx: mpsc::UnboundedSender<TransferEvent>,
    pending: Mutex<HashMap<Uuid, SessionChannels>>,
    tls_config: crate::transfer::tls::TlsConfig,
}

impl SessionManager {
    pub fn new(our: Identity, events_tx: mpsc::UnboundedSender<TransferEvent>, tls_config: crate::transfer::tls::TlsConfig) -> Arc<Self> {
        Arc::new(Self { our, events_tx, pending: Mutex::new(HashMap::new()), tls_config })
    }

    pub async fn run_listener(self: Arc<Self>, port: u16) -> anyhow::Result<()> {
        let listener = match TcpListener::bind(("0.0.0.0", port)).await {
            Ok(l) => l, Err(e) => { tracing::error!("listener bind failed on port {port}: {e}"); return Err(e.into()); }
        };
        loop {
            match listener.accept().await {
                Ok((stream, _)) => { let me = self.clone(); tokio::spawn(async move { let _ = me.handle_incoming(stream).await; }); }
                Err(e) => { tracing::warn!("accept error: {e}"); continue; }
            }
        }
    }

    async fn handle_incoming(self: Arc<Self>, mut stream: TcpStream) -> anyhow::Result<()> {
        let peer = stream.peer_addr().ok();
        let (ty, buf) = read_control(&mut stream).await?;
        tracing::info!("incoming connection from {peer:?}: first frame {ty:?}");
        match ty {
            MsgType::Hello => {
                let hello: Hello = bincode::deserialize(&buf)?;
                tracing::info!("Hello from '{}' session {}", hello.name, hello.session_id);
                let session_id = hello.session_id;
                let is_secure = hello.secure;
                let (dtx, drx) = oneshot::channel::<Decision>();
                let (xtx, xrx) = mpsc::channel::<DataStream>(16);
                self.pending.lock().await.insert(session_id, SessionChannels {
                    decision_tx: Some(dtx), data_tx: xtx, secure: is_secure,
                });
                let events = self.events_tx.clone();
                let our = self.our.clone();
                let tls = self.tls_config.clone();
                tokio::spawn(async move {
                    let _ = run_receiver(stream, hello, events, drx, xrx, our, tls).await;
                });
                Ok(())
            }
            MsgType::DataOpen => {
                let d: DataOpen = bincode::deserialize(&buf)?;
                let (tx, is_secure) = {
                    let guard = self.pending.lock().await;
                    let ch = guard.get(&d.session_id);
                    (ch.map(|c| c.data_tx.clone()), ch.map(|c| c.secure).unwrap_or(false))
                };
                if let Some(tx) = tx {
                    let ds = if is_secure {
                        let acceptor = tokio_rustls::TlsAcceptor::from(self.tls_config.clone());
                        let tls_stream = acceptor.accept(stream).await
                            .map_err(|e| anyhow::anyhow!("data TLS accept: {e}"))?;
                        DataStream::Tls(Box::new(tls_stream))
                    } else {
                        DataStream::Plain(stream)
                    };
                    let _ = tx.send(ds).await;
                }
                Ok(())
            }
            other => Err(anyhow::anyhow!("unexpected first frame {other:?}")),
        }
    }

    pub async fn respond(&self, session_id: Uuid, accept: bool, save_dir: PathBuf, pin: Option<String>) -> anyhow::Result<()> {
        let mut guard = self.pending.lock().await;
        if let Some(ch) = guard.get_mut(&session_id)
            && let Some(dtx) = ch.decision_tx.take() {
                let _ = dtx.send(Decision { accept, save_dir, pin });
            }
        Ok(())
    }

    pub async fn cleanup_session(&self, session_id: Uuid) {
        self.pending.lock().await.remove(&session_id);
    }

    pub fn start_send(&self, peer_addrs: Vec<SocketAddr>, files: Vec<String>, config: crate::store::TransferConfig, secure: bool) -> anyhow::Result<Uuid> {
        let session_id = Uuid::new_v4();
        let our = self.our.clone(); let events = self.events_tx.clone(); let id = session_id;
        tokio::spawn(async move { let _ = run_sender(id, peer_addrs, files, our, events, config, secure).await; });
        Ok(session_id)
    }
}
