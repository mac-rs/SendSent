use crate::discovery::{Discovery, Peer, PeerEvent, PeerRegistry, Platform};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};

const SERVICE_TYPE: &str = "_sendsent._tcp.local.";

pub struct MdnsDiscovery {
    daemon: Mutex<Option<ServiceDaemon>>,
    registry: Arc<Mutex<PeerRegistry>>,
    tx: mpsc::UnboundedSender<PeerEvent>,
    identity: Mutex<crate::store::Identity>,
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
            port,
        }
    }
}

#[async_trait]
impl Discovery for MdnsDiscovery {
    async fn start(&self) -> Result<()> {
        let daemon = ServiceDaemon::new().map_err(|e| anyhow!("mdns daemon: {e}"))?;

        let id = self.identity.lock().await.clone();
        let host_name = format!("{}.local.", id.name.replace(' ', "-"));
        let my_ip = pick_primary_ip().ok_or_else(|| anyhow!("no usable ipv4"))?;

        let mut props = HashMap::<String, String>::new();
        props.insert("v".into(), "1".into());
        props.insert("id".into(), id.device_id.clone());
        props.insert("name".into(), id.name.clone());
        props.insert("plat".into(), id.platform.clone());
        props.insert("port".into(), self.port.to_string());

        let info = ServiceInfo::new(
            SERVICE_TYPE,
            &id.name,
            &host_name,
            &my_ip,
            self.port,
            props,
        )
        .map_err(|e| anyhow!("mdns info: {e}"))?;
        daemon
            .register(info)
            .map_err(|e| anyhow!("mdns register: {e}"))?;

        let recv = daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| anyhow!("mdns browse: {e}"))?;

        let registry = self.registry.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            loop {
                match recv.recv_async().await {
                    Ok(ServiceEvent::ServiceResolved(info)) => {
                        if let Some(ev) = handle_resolved(&registry, &info).await {
                            let _ = tx.send(ev);
                        }
                    }
                    Ok(ServiceEvent::ServiceRemoved(_instance, fullname)) => {
                        if let Some(ev) = handle_removed(&registry, &fullname).await {
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
            let mut t = tokio::time::interval(Duration::from_secs(60));
            loop {
                t.tick().await;
                let mut r = registry.lock().await;
                for ev in r.sweep(Instant::now()) {
                    let _ = tx.send(ev);
                }
            }
        });

        *self.daemon.lock().await = Some(daemon);
        Ok(())
    }

    async fn peers(&self) -> Vec<Peer> {
        self.registry.lock().await.list()
    }

    async fn set_display_name(&self, _name: &str) -> Result<()> {
        Ok(())
    }
}

async fn handle_resolved(
    reg: &Arc<Mutex<PeerRegistry>>,
    info: &ServiceInfo,
) -> Option<PeerEvent> {
    let device_id = info.get_property_val_str("id")?.to_string();
    if device_id.is_empty() {
        return None;
    }
    let name = info.get_property_val_str("name").unwrap_or("?").to_string();
    let platform = match info.get_property_val_str("plat").unwrap_or("") {
        "windows" => Platform::Windows,
        "linux" => Platform::Linux,
        "ios" => Platform::Ios,
        "android" => Platform::Android,
        _ => Platform::Macos,
    };
    let port: u16 = info
        .get_property_val_str("port")
        .and_then(|s| s.parse().ok())
        .unwrap_or(52225);
    let proto_version: u16 = info
        .get_property_val_str("v")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    let addrs: Vec<std::net::SocketAddr> = info
        .get_addresses()
        .iter()
        .map(|ip| std::net::SocketAddr::new(*ip, port))
        .collect();

    let peer = Peer {
        device_id,
        name,
        platform,
        proto_version,
        addrs,
        port,
        last_seen_ms: 0,
    };
    reg.lock().await.upsert(Instant::now(), peer)
}

async fn handle_removed(reg: &Arc<Mutex<PeerRegistry>>, fullname: &str) -> Option<PeerEvent> {
    let instance = fullname.split('.').next().unwrap_or("");
    let mut r = reg.lock().await;
    let hit = r
        .list()
        .into_iter()
        .find(|p| p.name.replace(' ', "-") == instance);
    if let Some(p) = hit {
        r.remove(&p.device_id)
    } else {
        None
    }
}

fn pick_primary_ip() -> Option<IpAddr> {
    local_ip_iter().into_iter().next()
}

fn local_ip_iter() -> Vec<IpAddr> {
    use std::net::UdpSocket;
    let mut out = Vec::new();
    if let Ok(s) = UdpSocket::bind("0.0.0.0:0") {
        if s.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = s.local_addr() {
                out.push(addr.ip());
            }
        }
    }
    out
}
