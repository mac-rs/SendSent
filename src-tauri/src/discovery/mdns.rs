use crate::discovery::{platform_from_str, Discovery, Peer, PeerEvent, PeerRegistry, Platform};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};

const SERVICE_TYPE: &str = "_sendsent._tcp.local.";

pub struct MdnsDiscovery {
    daemon: Mutex<Option<ServiceDaemon>>,
    registry: Arc<Mutex<PeerRegistry>>,
    tx: mpsc::UnboundedSender<PeerEvent>,
    identity: Mutex<crate::store::Identity>,
    last_fullname: Mutex<Option<String>>,
    port: u16,
}

impl MdnsDiscovery {
    pub fn new(
        identity: crate::store::Identity,
        port: u16,
        tx: mpsc::UnboundedSender<PeerEvent>,
    ) -> Self {
        Self {
            daemon: Mutex::new(None),
            registry: Arc::new(Mutex::new(PeerRegistry::new())),
            tx,
            identity: Mutex::new(identity),
            last_fullname: Mutex::new(None),
            port,
        }
    }
}

fn build_info(id: &crate::store::Identity, port: u16) -> Result<ServiceInfo> {
    let host_name = format!("{}.local.", id.name.replace(' ', "-"));
    let my_ip = pick_primary_ip().ok_or_else(|| anyhow!("no usable ipv4"))?;
    let mut props = HashMap::<String, String>::new();
    props.insert("v".into(), "1".into());
    props.insert("id".into(), id.device_id.clone());
    props.insert("name".into(), id.name.clone());
    props.insert("plat".into(), id.platform.clone());
    props.insert("port".into(), port.to_string());
    props.insert("ip".into(), my_ip.to_string());
    ServiceInfo::new(SERVICE_TYPE, &id.name, &host_name, my_ip, port, props)
        .map_err(|e| anyhow!("mdns info: {e}"))
}

#[async_trait]
impl Discovery for MdnsDiscovery {
    async fn start(&self) -> Result<()> {
        let daemon = ServiceDaemon::new().map_err(|e| anyhow!("mdns daemon: {e}"))?;
        let id = self.identity.lock().await.clone();
        let info = build_info(&id, self.port)?;
        let fullname = info.get_fullname().to_string();
        daemon.register(info).map_err(|e| anyhow!("mdns register: {e}"))?;
        *self.last_fullname.lock().await = Some(fullname);

        let recv = daemon.browse(SERVICE_TYPE).map_err(|e| anyhow!("mdns browse: {e}"))?;
        let registry = self.registry.clone();
        let tx = self.tx.clone();
        let our_id = id.device_id.clone();
        tokio::spawn(async move {
            loop {
                match recv.recv_async().await {
                    Ok(ServiceEvent::ServiceResolved(info)) => {
                        if let Some(ev) = handle_resolved(&registry, &info, &our_id).await {
                            let _ = tx.send(ev);
                        }
                    }
                    Ok(ServiceEvent::ServiceRemoved(instance, _fullname)) => {
                        if let Some(ev) = handle_removed(&registry, &instance).await {
                            let _ = tx.send(ev);
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        });

        let registry = self.registry.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut t = tokio::time::interval(Duration::from_secs(20));
            loop {
                t.tick().await;
                let mut r = registry.lock().await;
                for ev in r.sweep() {
                    let _ = tx.send(ev);
                }
            }
        });

        *self.daemon.lock().await = Some(daemon);
        Ok(())
    }

    async fn peers(&self) -> Vec<Peer> { self.registry.lock().await.list() }

    async fn set_display_name(&self, name: &str) -> Result<()> {
        {
            let mut id = self.identity.lock().await;
            id.name = name.to_string();
        }
        let daemon_guard = self.daemon.lock().await;
        let Some(daemon) = daemon_guard.as_ref() else { return Ok(()) };
        if let Some(old) = self.last_fullname.lock().await.take() {
            let _ = daemon.unregister(&old);
        }
        let id = self.identity.lock().await.clone();
        let info = build_info(&id, self.port)?;
        let fullname = info.get_fullname().to_string();
        daemon.register(info).map_err(|e| anyhow!("mdns register: {e}"))?;
        *self.last_fullname.lock().await = Some(fullname);
        Ok(())
    }

    async fn add_manual_peer(&self, addr: std::net::SocketAddr) -> Result<()> {
        let id = format!("manual-{}", uuid::Uuid::new_v4());
        let p = Peer {
            device_id: id, name: format!("{}:{}", addr.ip(), addr.port()),
            platform: Platform::Unknown, proto_version: 1,
            addrs: vec![addr], port: addr.port(), last_seen_ms: 0,
        };
        if let Some(ev) = self.registry.lock().await.upsert(p) {
            let _ = self.tx.send(ev);
        }
        Ok(())
    }
}

async fn handle_resolved(reg: &Arc<Mutex<PeerRegistry>>, info: &ResolvedService, our_id: &str) -> Option<PeerEvent> {
    let device_id = info.get_property_val_str("id")?.to_string();
    if device_id.is_empty() || device_id == our_id { return None; }
    let name = info.get_property_val_str("name").unwrap_or("?").to_string();
    let platform = platform_from_str(info.get_property_val_str("plat").unwrap_or(""));
    let port: u16 = info.get_property_val_str("port")
        .and_then(|s| s.parse().ok()).unwrap_or(52225);
    let proto_version: u16 = info.get_property_val_str("v")
        .and_then(|s| s.parse().ok()).unwrap_or(1);
    let addrs: Vec<std::net::SocketAddr> = info.get_addresses().iter()
        .map(|ip| std::net::SocketAddr::new(ip.to_ip_addr(), port))
        .collect();
    let peer = Peer { device_id, name, platform, proto_version, addrs, port, last_seen_ms: 0 };
    reg.lock().await.upsert(peer)
}

async fn handle_removed(reg: &Arc<Mutex<PeerRegistry>>, instance: &str) -> Option<PeerEvent> {
    let mut r = reg.lock().await;
    // mDNS 实例名 = 设备显示名(与 TXT `name` 一致)。之前用 fullname 首段和
    // `name.replace(' ','-')` 比对,空格/转义对不上,导致下线设备删不掉。
    let hit = r.list().into_iter()
        .find(|p| p.name == instance || p.name.replace(' ', "-") == instance);
    if let Some(p) = hit { r.remove(&p.device_id) } else { None }
}

fn pick_primary_ip() -> Option<IpAddr> { local_ip_iter().into_iter().next() }

fn local_ip_iter() -> Vec<IpAddr> {
    use std::net::UdpSocket;
    let mut out = Vec::new();
    if let Ok(s) = UdpSocket::bind("0.0.0.0:0")
        && s.connect("8.8.8.8:80").is_ok()
        && let Ok(addr) = s.local_addr() { out.push(addr.ip()); }
    out
}
