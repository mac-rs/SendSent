//! Android discovery via the platform NSD service (NsdManager), bridged through
//! `crate::nsd_plugin`. mDNS on the wire, so it interoperates with the
//! mdns-sd/Bonjour peers on desktop/iOS.

use crate::discovery::{platform_from_str, Discovery, Peer, PeerEvent, PeerRegistry};
use crate::nsd_plugin::{Nsd, NsdService};
use crate::store::Identity;
use anyhow::Result;
use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tauri::Wry;
use tokio::sync::{mpsc, Mutex};

const POLL_INTERVAL: Duration = Duration::from_secs(2);

pub struct NsdDiscovery {
    nsd: Nsd<Wry>,
    registry: Arc<Mutex<PeerRegistry>>,
    tx: mpsc::UnboundedSender<PeerEvent>,
    identity: Identity,
    port: u16,
}

impl NsdDiscovery {
    pub fn new(
        nsd: Nsd<Wry>,
        identity: Identity,
        port: u16,
        tx: mpsc::UnboundedSender<PeerEvent>,
    ) -> Self {
        Self {
            nsd,
            registry: Arc::new(Mutex::new(PeerRegistry::new())),
            tx,
            identity,
            port,
        }
    }
}

#[async_trait]
impl Discovery for NsdDiscovery {
    async fn start(&self) -> Result<()> {
        let id = self.identity.clone();
        let ip = crate::discovery::primary_ipv4().map(|i| i.to_string()).unwrap_or_default();
        if let Err(e) = self.nsd.register(&id.name, &id.device_id, &id.platform, self.port, &ip).await {
            tracing::error!("nsd register failed: {e}");
        }
        if let Err(e) = self.nsd.browse().await {
            tracing::error!("nsd browse failed: {e}");
        }

        let nsd = self.nsd.clone();
        let registry = self.registry.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(POLL_INTERVAL);
            loop {
                tick.tick().await;
                match nsd.poll().await {
                    Ok(services) => {
                        let mut reg = registry.lock().await;
                        for svc in &services {
                            if let Some(peer) = peer_from_service(svc)
                                && let Some(ev) = reg.upsert(peer)
                            {
                                let _ = tx.send(ev);
                            }
                        }
                        for ev in reg.sweep() {
                            let _ = tx.send(ev);
                        }
                    }
                    Err(e) => tracing::warn!("nsd poll failed: {e}"),
                }
            }
        });
        Ok(())
    }

    async fn peers(&self) -> Vec<Peer> {
        self.registry.lock().await.list()
    }

    async fn set_display_name(&self, _name: &str) -> Result<()> { Ok(()) }

    async fn add_manual_peer(&self, addr: SocketAddr) -> Result<()> {
        let id = format!("manual-{}", uuid::Uuid::new_v4());
        let p = Peer {
            device_id: id, name: format!("{}:{}", addr.ip(), addr.port()),
            platform: crate::discovery::Platform::Unknown, proto_version: 1,
            addrs: vec![addr], port: addr.port(), last_seen_ms: 0,
        };
        if let Some(ev) = self.registry.lock().await.upsert(p) {
            let _ = self.tx.send(ev);
        }
        Ok(())
    }
}

fn peer_from_service(svc: &NsdService) -> Option<Peer> {
    let device_id = svc.txt.get("id")?.clone();
    if device_id.is_empty() {
        return None;
    }
    let name = svc.txt.get("name").cloned().unwrap_or_else(|| svc.name.clone());
    let platform = platform_from_str(svc.txt.get("plat").map(String::as_str).unwrap_or(""));
    let port = svc
        .txt
        .get("port")
        .and_then(|p| p.parse().ok())
        .unwrap_or(svc.port);
    let ip: std::net::IpAddr = svc
        .txt
        .get("ip")
        .and_then(|s| s.parse().ok())
        .or_else(|| svc.host.parse().ok())?;
    Some(Peer {
        device_id,
        name,
        platform,
        proto_version: 1,
        addrs: vec![SocketAddr::new(ip, port)],
        port,
        last_seen_ms: 0,
    })
}
