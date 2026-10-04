//! Android 原生：发现由 Kotlin `NsdManager` 驱动，这里只维护 peer 注册表。
//! Kotlin 解析到服务后调用 `nativeOnService`，丢失时调用 `nativeOnServiceLost`。

use crate::discovery::{platform_from_str, Discovery, Peer, PeerEvent, PeerRegistry, Platform};
use crate::store::Identity;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub struct AndroidNativeDiscovery {
    registry: Arc<Mutex<PeerRegistry>>,
    /// serviceName -> device_id（用于 on_service_lost 反查）
    by_name: Arc<Mutex<HashMap<String, String>>>,
    our_id: String,
    tx: mpsc::UnboundedSender<PeerEvent>,
}

impl AndroidNativeDiscovery {
    pub fn new(identity: Identity, _port: u16, tx: mpsc::UnboundedSender<PeerEvent>) -> Self {
        Self {
            registry: Arc::new(Mutex::new(PeerRegistry::new())),
            by_name: Arc::new(Mutex::new(HashMap::new())),
            our_id: identity.device_id,
            tx,
        }
    }
}

#[async_trait]
impl Discovery for AndroidNativeDiscovery {
    async fn start(&self) -> Result<()> {
        Ok(()) // Kotlin 驱动
    }

    async fn peers(&self) -> Vec<Peer> {
        self.registry.lock().expect("registry lock").list()
    }

    async fn set_display_name(&self, _name: &str) -> Result<()> {
        Ok(())
    }

    async fn add_manual_peer(&self, addr: SocketAddr) -> Result<()> {
        let id = format!("manual-{}", uuid::Uuid::new_v4());
        let p = Peer {
            device_id: id,
            name: format!("{}:{}", addr.ip(), addr.port()),
            platform: Platform::Unknown,
            proto_version: 1,
            addrs: vec![addr],
            port: addr.port(),
            last_seen_ms: 0,
        };
        if let Some(ev) = self.registry.lock().expect("registry lock").upsert(p) {
            let _ = self.tx.send(ev);
        }
        Ok(())
    }

    async fn on_service(&self, sname: &str, host: &str, port: u16, txt: &HashMap<String, String>) {
        let id = txt.get("id").cloned().unwrap_or_else(|| sname.to_string());
        if id == self.our_id {
            return; // 忽略自己
        }
        let name = txt.get("name").cloned().unwrap_or_else(|| sname.to_string());
        let platform = txt.get("plat").map(|p| platform_from_str(p)).unwrap_or(Platform::Unknown);
        let addr: SocketAddr = match format!("{host}:{port}").parse() {
            Ok(a) => a,
            Err(_) => return,
        };
        let p = Peer {
            device_id: id.clone(),
            name,
            platform,
            proto_version: 1,
            addrs: vec![addr],
            port,
            last_seen_ms: 0,
        };
        self.by_name.lock().expect("by_name lock").insert(sname.to_string(), id);
        if let Some(ev) = self.registry.lock().expect("registry lock").upsert(p) {
            let _ = self.tx.send(ev);
        }
    }

    async fn on_service_lost(&self, sname: &str) {
        let id = self.by_name.lock().expect("by_name lock").remove(sname);
        if let Some(id) = id
            && let Some(ev) = self.registry.lock().expect("registry lock").remove(&id)
        {
            let _ = self.tx.send(ev);
        }
    }
}
