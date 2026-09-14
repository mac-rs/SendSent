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
            // mdns-sd 会从自己的缓存里反复重发 ServiceResolved,单看 last_seen 感知不到
            // 设备下线(iOS/Android 用系统 mDNS 能秒感知,桌面不能)。这里主动 TCP 探活:
            // 连不上对端接收端口即视为下线,连续 miss 若干次后移除。
            let mut misses: HashMap<String, u8> = HashMap::new();
            let mut t = tokio::time::interval(Duration::from_secs(10));
            loop {
                t.tick().await;
                for p in registry.lock().await.list() {
                    let id = p.device_id.clone();
                    if probe_alive(&p.addrs).await {
                        misses.remove(&id);
                        registry.lock().await.touch(&id);
                    } else {
                        let c = misses.entry(id.clone()).or_insert(0);
                        *c += 1;
                        if *c >= 2 {
                            misses.remove(&id);
                            if let Some(ev) = registry.lock().await.remove(&id) {
                                tracing::info!("peer {id} unreachable, removed");
                                let _ = tx.send(ev);
                            }
                        }
                    }
                }
                // 兜底:长时间没被 touch 的也清掉。
                for ev in registry.lock().await.sweep() {
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

/// 主动探活:能否 TCP 连上对端任一地址(其接收监听端口)。用于感知设备下线。
async fn probe_alive(addrs: &[std::net::SocketAddr]) -> bool {
    for a in addrs {
        if tokio::time::timeout(Duration::from_millis(1200), tokio::net::TcpStream::connect(a))
            .await
            .is_ok_and(|r| r.is_ok())
        {
            return true;
        }
    }
    false
}

fn local_ip_iter() -> Vec<IpAddr> {
    use std::net::UdpSocket;
    let mut out = Vec::new();
    if let Ok(s) = UdpSocket::bind("0.0.0.0:0")
        && s.connect("8.8.8.8:80").is_ok()
        && let Ok(addr) = s.local_addr() { out.push(addr.ip()); }
    out
}
