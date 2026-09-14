pub mod mdns;
#[cfg(target_os = "ios")]
pub mod ios_bonjour;
#[cfg(target_os = "android")]
pub mod android_native;
#[cfg(all(target_os = "android", feature = "tauri-shell"))]
pub mod nsd;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

pub const STALE_AFTER: Duration = Duration::from_secs(90);

/// Best-effort primary IPv4 via a UDP "connect" (no packets are sent). Works on
/// Android too, where `getifaddrs()` is blocked by SELinux.
pub fn primary_ipv4() -> Option<std::net::IpAddr> {
    use std::net::UdpSocket;
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    s.local_addr().ok().map(|a| a.ip())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Platform { Macos, Windows, Linux, Ios, Android, Unknown }

/// Map an mDNS `plat` TXT value to a platform. Unrecognized labels (including
/// the legacy "unknown" emitted by older mobile builds) become `Unknown` rather
/// than masquerading as macOS.
pub fn platform_from_str(s: &str) -> Platform {
    match s {
        "macos" => Platform::Macos,
        "windows" => Platform::Windows,
        "linux" => Platform::Linux,
        "ios" => Platform::Ios,
        "android" => Platform::Android,
        _ => Platform::Unknown,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub device_id: String,
    pub name: String,
    pub platform: Platform,
    pub proto_version: u16,
    pub addrs: Vec<SocketAddr>,
    pub port: u16,
    pub last_seen_ms: u128,
}

#[derive(Debug, Clone)]
pub enum PeerEvent { Found(Peer), Lost(String) }

#[async_trait]
pub trait Discovery: Send + Sync {
    async fn start(&self) -> anyhow::Result<()>;
    async fn peers(&self) -> Vec<Peer>;
    async fn set_display_name(&self, name: &str) -> anyhow::Result<()>;
    /// 重新广播自己。iOS 从后台回前台后 Bonjour 注册会失效,需要重注册。默认 no-op。
    async fn reannounce(&self) -> anyhow::Result<()> {
        Ok(())
    }
    /// 手动添加一个 peer(绕过 mDNS,用于输入 IP:port 或主动探测)
    async fn add_manual_peer(&self, addr: std::net::SocketAddr) -> anyhow::Result<()>;
    /// Android: Kotlin NsdManager 解析到服务后推来。默认 no-op。
    async fn on_service(
        &self,
        name: &str,
        host: &str,
        port: u16,
        txt: &std::collections::HashMap<String, String>,
    ) {
        let _ = (name, host, port, txt);
    }
    /// Android: NsdManager 报告服务丢失。默认 no-op。
    async fn on_service_lost(&self, name: &str) {
        let _ = name;
    }
}

#[derive(Default)]
pub struct PeerRegistry { peers: HashMap<String, Peer> }

impl PeerRegistry {
    pub fn new() -> Self { Self::default() }
    pub fn upsert(&mut self, now: Instant, mut p: Peer) -> Option<PeerEvent> {
        p.last_seen_ms = now.elapsed().as_millis();
        let id = p.device_id.clone();
        let existed = self.peers.contains_key(&id);
        self.peers.insert(id.clone(), p.clone());
        if existed { None } else { Some(PeerEvent::Found(p)) }
    }
    pub fn remove(&mut self, device_id: &str) -> Option<PeerEvent> {
        if self.peers.remove(device_id).is_some() { Some(PeerEvent::Lost(device_id.to_string())) } else { None }
    }
    pub fn sweep(&mut self, now: Instant) -> Vec<PeerEvent> {
        let now_ms = now.elapsed().as_millis();
        let stale: Vec<String> = self.peers.iter()
            .filter(|(_, p)| now_ms.saturating_sub(p.last_seen_ms) > STALE_AFTER.as_millis())
            .map(|(k, _)| k.clone()).collect();
        stale.into_iter().filter_map(|k| self.remove(&k)).collect()
    }
    pub fn list(&self) -> Vec<Peer> { self.peers.values().cloned().collect() }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn peer(id: &str) -> Peer {
        Peer { device_id: id.into(), name: id.into(), platform: Platform::Macos,
               proto_version: 1, addrs: vec!["127.0.0.1:52225".parse().unwrap()],
               port: 52225, last_seen_ms: 0 }
    }
    #[test]
    fn upsert_dedup_emits_only_once() {
        let mut r = PeerRegistry::new();
        let now = Instant::now();
        assert!(matches!(r.upsert(now, peer("a")), Some(PeerEvent::Found(_))));
        assert!(r.upsert(now, peer("a")).is_none());
        assert_eq!(r.list().len(), 1);
    }
    #[test]
    fn remove_emits_lost() {
        let mut r = PeerRegistry::new();
        let now = Instant::now();
        r.upsert(now, peer("a"));
        assert!(matches!(r.remove("a"), Some(PeerEvent::Lost(_))));
        assert!(r.list().is_empty());
    }
    #[test]
    fn platform_labels_map() {
        assert_eq!(platform_from_str("ios"), Platform::Ios);
        assert_eq!(platform_from_str("android"), Platform::Android);
        assert_eq!(platform_from_str("macos"), Platform::Macos);
        assert_eq!(platform_from_str("unknown"), Platform::Unknown);
        assert_eq!(platform_from_str(""), Platform::Unknown);
    }
}
