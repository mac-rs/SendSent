//! iOS discovery via the system Bonjour responder (DNS-SD C API, wrapped by the
//! `zeroconf` crate).
//!
//! `mdns-sd` opens raw UDP multicast sockets, which iOS blocks unless the app
//! holds the restricted `com.apple.developer.networking.multicast` entitlement
//! (hard to obtain for apps doing their own mDNS). Here we let the OS own the
//! protocol instead: Apple's responder does the multicast, so no entitlement is
//! required — only `NSBonjourServices` + the local-network permission prompt.

use crate::discovery::{platform_from_str, Discovery, Peer, PeerEvent, PeerRegistry, Platform};
use crate::store::Identity;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::any::Any;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use zeroconf::prelude::*;
use zeroconf::{BrowserEvent, MdnsBrowser, MdnsService, ServiceDiscovery, ServiceType, TxtRecord};

const SERVICE_NAME: &str = "sendsent";
const SERVICE_PROTOCOL: &str = "tcp";
const POLL_INTERVAL: Duration = Duration::from_secs(1);

pub struct BonjourDiscovery {
    registry: Arc<Mutex<PeerRegistry>>,
    tx: mpsc::UnboundedSender<PeerEvent>,
    identity: Identity,
    port: u16,
}

impl BonjourDiscovery {
    pub fn new(identity: Identity, port: u16, tx: mpsc::UnboundedSender<PeerEvent>) -> Self {
        Self {
            registry: Arc::new(Mutex::new(PeerRegistry::new())),
            tx,
            identity,
            port,
        }
    }
}

#[async_trait]
impl Discovery for BonjourDiscovery {
    async fn start(&self) -> Result<()> {
        let identity = self.identity.clone();
        let port = self.port;
        std::thread::spawn(move || {
            if let Err(e) = run_service(identity, port) {
                tracing::error!("ios bonjour register stopped: {e}");
            }
        });

        let registry = self.registry.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            if let Err(e) = run_browser(registry, tx) {
                tracing::error!("ios bonjour browse stopped: {e}");
            }
        });
        Ok(())
    }

    async fn peers(&self) -> Vec<Peer> {
        self.registry.lock().expect("registry lock").list()
    }

    async fn set_display_name(&self, _name: &str) -> Result<()> { Ok(()) }

    async fn add_manual_peer(&self, addr: SocketAddr) -> Result<()> {
        let id = format!("manual-{}", uuid::Uuid::new_v4());
        let p = Peer {
            device_id: id, name: format!("{}:{}", addr.ip(), addr.port()),
            platform: Platform::Unknown, proto_version: 1,
            addrs: vec![addr], port: addr.port(), last_seen_ms: 0,
        };
        if let Some(ev) = self.registry.lock().expect("registry lock").upsert(Instant::now(), p) {
            let _ = self.tx.send(ev);
        }
        Ok(())
    }
}

fn service_type() -> Result<ServiceType> {
    ServiceType::new(SERVICE_NAME, SERVICE_PROTOCOL).map_err(|e| anyhow!("service type: {e}"))
}

/// Registers (advertises) this device, then drives the Bonjour event loop.
fn run_service(id: Identity, port: u16) -> Result<()> {
    let mut service = MdnsService::new(service_type()?, port);
    service.set_name(&id.name);

    let mut txt = TxtRecord::new();
    txt.insert("v", "1").map_err(|e| anyhow!("txt v: {e}"))?;
    txt.insert("id", &id.device_id).map_err(|e| anyhow!("txt id: {e}"))?;
    txt.insert("name", &id.name).map_err(|e| anyhow!("txt name: {e}"))?;
    txt.insert("plat", &id.platform).map_err(|e| anyhow!("txt plat: {e}"))?;
    let port_s = port.to_string();
    txt.insert("port", &port_s).map_err(|e| anyhow!("txt port: {e}"))?;
    service.set_txt_record(txt);

    service.set_registered_callback(Box::new(
        |result: zeroconf::Result<zeroconf::ServiceRegistration>, _ctx: Option<Arc<dyn Any + Send + Sync>>| {
            match result {
                Ok(reg) => tracing::info!("ios bonjour registered as {}", reg.name()),
                Err(e) => tracing::error!("ios bonjour register callback: {e}"),
            }
        },
    ));

    let event_loop = service.register().map_err(|e| anyhow!("register: {e}"))?;
    loop {
        event_loop.poll(POLL_INTERVAL).map_err(|e| anyhow!("register poll: {e}"))?;
    }
}

/// Browses for peers and feeds resolved/removed services into the registry.
fn run_browser(registry: Arc<Mutex<PeerRegistry>>, tx: mpsc::UnboundedSender<PeerEvent>) -> Result<()> {
    let mut browser = MdnsBrowser::new(service_type()?);
    let reg = registry.clone();
    let txc = tx.clone();
    browser.set_service_callback(Box::new(
        move |result: zeroconf::Result<BrowserEvent>, _ctx: Option<Arc<dyn Any + Send + Sync>>| match result {
            Ok(BrowserEvent::Add(sd)) => {
                if let Some(peer) = peer_from_discovery(&sd) {
                    let mut r = reg.lock().expect("registry lock");
                    if let Some(ev) = r.upsert(Instant::now(), peer) {
                        let _ = txc.send(ev);
                    }
                }
            }
            Ok(BrowserEvent::Remove(rem)) => {
                let instance = rem.name().clone();
                let mut r = reg.lock().expect("registry lock");
                let hit = r.list().into_iter().find(|p| p.name == instance);
                if let Some(p) = hit && let Some(ev) = r.remove(&p.device_id) {
                    let _ = txc.send(ev);
                }
            }
            Err(e) => tracing::warn!("ios bonjour browse event error: {e}"),
        },
    ));

    let event_loop = browser.browse_services().map_err(|e| anyhow!("browse: {e}"))?;
    loop {
        event_loop.poll(POLL_INTERVAL).map_err(|e| anyhow!("browse poll: {e}"))?;
    }
}

fn peer_from_discovery(sd: &ServiceDiscovery) -> Option<Peer> {
    let txt = sd.txt().as_ref()?;
    let device_id = txt.get("id")?;
    let name = txt.get("name").unwrap_or_else(|| sd.name().clone());
    let platform = platform_from_str(&txt.get("plat").unwrap_or_default());
    let port = txt.get("port").and_then(|p| p.parse().ok()).unwrap_or(*sd.port());
    let ip: std::net::IpAddr = sd.address().parse().ok()?;
    let proto_version = txt.get("v").and_then(|v| v.parse().ok()).unwrap_or(1);
    Some(Peer {
        device_id, name, platform, proto_version,
        addrs: vec![SocketAddr::new(ip, port)], port, last_seen_ms: 0,
    })
}
