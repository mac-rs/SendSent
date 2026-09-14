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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
    name: Arc<Mutex<String>>,
    generation: Arc<AtomicU64>,
    port: u16,
}

impl BonjourDiscovery {
    pub fn new(identity: Identity, port: u16, tx: mpsc::UnboundedSender<PeerEvent>) -> Self {
        let name = Arc::new(Mutex::new(identity.name.clone()));
        Self {
            registry: Arc::new(Mutex::new(PeerRegistry::new())),
            tx,
            identity,
            name,
            generation: Arc::new(AtomicU64::new(0)),
            port,
        }
    }
}

#[async_trait]
impl Discovery for BonjourDiscovery {
    async fn start(&self) -> Result<()> {
        let identity = self.identity.clone();
        let port = self.port;
        let name = self.name.clone();
        let generation = self.generation.clone();
        std::thread::spawn(move || {
            if let Err(e) = run_service(identity, port, name, generation) {
                tracing::error!("ios bonjour register stopped: {e}");
            }
        });

        let registry = self.registry.clone();
        let tx = self.tx.clone();
        let our_id = self.identity.device_id.clone();
        std::thread::spawn(move || {
            if let Err(e) = run_browser(registry, tx, our_id) {
                tracing::error!("ios bonjour browse stopped: {e}");
            }
        });
        Ok(())
    }

    async fn peers(&self) -> Vec<Peer> {
        self.registry.lock().expect("registry lock").list()
    }

    async fn set_display_name(&self, name: &str) -> Result<()> {
        *self.name.lock().expect("name lock") = name.to_string();
        self.generation.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn reannounce(&self) -> Result<()> {
        // 回前台:Bonjour 注册在后台会被系统停掉,递增 generation 触发重注册。
        self.generation.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn add_manual_peer(&self, addr: SocketAddr) -> Result<()> {
        let id = format!("manual-{}", uuid::Uuid::new_v4());
        let p = Peer {
            device_id: id, name: format!("{}:{}", addr.ip(), addr.port()),
            platform: Platform::Unknown, proto_version: 1,
            addrs: vec![addr], port: addr.port(), last_seen_ms: 0,
        };
        if let Some(ev) = self.registry.lock().expect("registry lock").upsert(p) {
            let _ = self.tx.send(ev);
        }
        Ok(())
    }
}

fn service_type() -> Result<ServiceType> {
    ServiceType::new(SERVICE_NAME, SERVICE_PROTOCOL).map_err(|e| anyhow!("service type: {e}"))
}

/// Registers (advertises) this device, then drives the Bonjour event loop.
/// Re-registers with a new name when `generation` changes (display-name edit).
fn run_service(
    id: Identity,
    port: u16,
    name: Arc<Mutex<String>>,
    generation: Arc<AtomicU64>,
) -> Result<()> {
    loop {
        let my_gen = generation.load(Ordering::SeqCst);
        let display = name.lock().expect("name lock").clone();

        let mut service = MdnsService::new(service_type()?, port);
        service.set_name(&display);

        let mut txt = TxtRecord::new();
        txt.insert("v", "1").map_err(|e| anyhow!("txt v: {e}"))?;
        txt.insert("id", &id.device_id).map_err(|e| anyhow!("txt id: {e}"))?;
        txt.insert("name", &display).map_err(|e| anyhow!("txt name: {e}"))?;
        txt.insert("plat", &id.platform).map_err(|e| anyhow!("txt plat: {e}"))?;
        let port_s = port.to_string();
        txt.insert("port", &port_s).map_err(|e| anyhow!("txt port: {e}"))?;
        if let Some(ip) = crate::discovery::primary_ipv4() {
            let ip_s = ip.to_string();
            txt.insert("ip", &ip_s).map_err(|e| anyhow!("txt ip: {e}"))?;
        }
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
        // 轮询直到改名;退出内层后 event_loop/service 析构 → 注销旧名字,再以新名字注册。
        while generation.load(Ordering::SeqCst) == my_gen {
            event_loop.poll(POLL_INTERVAL).map_err(|e| anyhow!("register poll: {e}"))?;
        }
        tracing::info!("ios bonjour re-registering with new name");
    }
}

/// Browses for peers and feeds resolved/removed services into the registry.
fn run_browser(
    registry: Arc<Mutex<PeerRegistry>>,
    tx: mpsc::UnboundedSender<PeerEvent>,
    our_id: String,
) -> Result<()> {
    let mut browser = MdnsBrowser::new(service_type()?);
    let reg = registry.clone();
    let txc = tx.clone();
    browser.set_service_callback(Box::new(
        move |result: zeroconf::Result<BrowserEvent>, _ctx: Option<Arc<dyn Any + Send + Sync>>| match result {
            Ok(BrowserEvent::Add(sd)) => {
                if let Some(peer) = peer_from_discovery(&sd) {
                    if peer.device_id == our_id {
                        return;
                    }
                    let mut r = reg.lock().expect("registry lock");
                    if let Some(ev) = r.upsert(peer) {
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
    let proto_version = txt.get("v").and_then(|v| v.parse().ok()).unwrap_or(1);

    // Prefer the IPv4 the peer published in its TXT record. `sd.address()`
    // comes from zeroconf's Bonjour resolver, which parses the callback as
    // `sockaddr_in` unconditionally, so peers that also publish IPv6 (Android
    // via NsdManager) make it return a bogus IPv4.
    let addr = txt
        .get("ip")
        .and_then(|s| s.parse::<std::net::IpAddr>().ok())
        .map(|ip| SocketAddr::new(ip, port))
        .or_else(|| resolve_prefer_ipv4(sd.host_name(), port))
        .or_else(|| sd.address().parse::<std::net::IpAddr>().ok().map(|ip| SocketAddr::new(ip, port)))?;

    Some(Peer {
        device_id, name, platform, proto_version,
        addrs: vec![addr], port, last_seen_ms: 0,
    })
}

/// Resolve a Bonjour host name (e.g. `Android_x.local.`) to a socket address,
/// preferring IPv4 so we never hand the TCP sender an IPv6-only target.
fn resolve_prefer_ipv4(host_name: &str, port: u16) -> Option<SocketAddr> {
    use std::net::ToSocketAddrs;
    let host = host_name.trim_end_matches('.');
    if host.is_empty() {
        return None;
    }
    let mut fallback = None;
    for addr in (host, port).to_socket_addrs().ok()? {
        match addr {
            SocketAddr::V4(_) => return Some(addr),
            SocketAddr::V6(_) => {
                if fallback.is_none() {
                    fallback = Some(addr);
                }
            }
        }
    }
    fallback
}
