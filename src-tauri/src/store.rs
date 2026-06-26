use anyhow::{Context, Result};
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub device_id: String,
    pub name: String,
    pub platform: String,
}

pub fn default_save_dir() -> Result<PathBuf> {
    // Desktop default. iOS computes its writable dir from the app handle (see lib.rs setup),
    // because $HOME is read-only in the iOS sandbox.
    let base = home_dir().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    Ok(base.join("Downloads").join("sendsent"))
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    { std::env::var_os("HOME").map(PathBuf::from)}
    #[cfg(target_os = "windows")]
    { return std::env::var_os("USERPROFILE").map(PathBuf::from); }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    { None }
}

pub fn load_or_create(data_dir: &Path, platform: &str, fallback_name: &str) -> Result<Identity> {
    std::fs::create_dir_all(data_dir).ok();
    let p = data_dir.join("identity.json");
    if p.exists() {
        let s = std::fs::read_to_string(&p).context("read identity")?;
        let id: Identity = serde_json::from_str(&s).context("parse identity")?;
        return Ok(id);
    }
    let id = Identity {
        device_id: Uuid::new_v4().to_string(),
        name: fallback_name.to_string(),
        platform: platform.to_string(),
    };
    let s = serde_json::to_string_pretty(&id).context("serialize identity")?;
    std::fs::write(&p, s).context("write identity")?;
    Ok(id)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransferConfig {
    pub conns: u32,
    pub chunk_size: u64,   // bytes
    pub split_threshold: u64, // bytes
}

impl TransferConfig {
    pub const DEFAULT_CONNS: u32 = 4;
    pub const DEFAULT_CHUNK: u64 = 1024 * 1024;       // 1 MiB (== MAX_DATA_PAYLOAD)
    pub const DEFAULT_SPLIT: u64 = 4 * 1024 * 1024;   // 4 MiB

    pub fn defaults() -> Self {
        Self { conns: Self::DEFAULT_CONNS, chunk_size: Self::DEFAULT_CHUNK, split_threshold: Self::DEFAULT_SPLIT }
    }

    /// clamp 到合法区间,避免恶意/手抖配置
    pub fn sanitized(mut self) -> Self {
        self.conns = self.conns.clamp(1, 16);
        self.chunk_size = self.chunk_size.clamp(64 * 1024, 1024 * 1024); // ≤ MAX_DATA_PAYLOAD
        self.split_threshold = self.split_threshold.max(self.chunk_size);
        self
    }
}

pub fn load_or_create_transfer_config(data_dir: &Path) -> TransferConfig {
    let p = data_dir.join("transfer.json");
    let mut cfg = match (|| -> anyhow::Result<TransferConfig> {
        if p.exists() {
            let s = std::fs::read_to_string(&p).context("read transfer config")?;
            return Ok(serde_json::from_str(&s).context("parse transfer config")?);
        }
        Ok(TransferConfig::defaults())
    })() {
        Ok(c) => c,
        Err(_) => TransferConfig::defaults(),
    };
    // 环境变量覆盖(便于压测/测试)
    if let Ok(v) = std::env::var("SENDSENT_CONNS") { if let Ok(n) = v.parse::<u32>() { cfg.conns = n; } }
    if let Ok(v) = std::env::var("SENDSENT_CHUNK_KB") { if let Ok(n) = v.parse::<u64>() { cfg.chunk_size = n * 1024; } }
    if let Ok(v) = std::env::var("SENDSENT_SPLIT_MB") { if let Ok(n) = v.parse::<u64>() { cfg.split_threshold = n * 1024 * 1024; } }
    cfg.sanitized()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creates_then_loads() {
        let tmp = std::env::temp_dir().join(format!("ss-id-{}", Uuid::new_v4()));
        let id1 = load_or_create(&tmp, "macos", "host").unwrap();
        assert_eq!(id1.platform, "macos");
        let id2 = load_or_create(&tmp, "macos", "host").unwrap();
        assert_eq!(id1.device_id, id2.device_id, "second load reuses stored id");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn transfer_config_sanitizes() {
        let c = TransferConfig { conns: 99, chunk_size: 10, split_threshold: 1 }.sanitized();
        assert_eq!(c.conns, 16);
        assert_eq!(c.chunk_size, 64 * 1024);
        assert_eq!(c.split_threshold, c.chunk_size);
    }

    #[test]
    fn transfer_config_loads_defaults_then_file() {
        let tmp = std::env::temp_dir().join(format!("ss-tcfg-{}", Uuid::new_v4()));
        let c1 = load_or_create_transfer_config(&tmp);
        assert_eq!(c1.conns, TransferConfig::DEFAULT_CONNS);
        let custom = TransferConfig { conns: 2, chunk_size: 256 * 1024, split_threshold: 8 * 1024 * 1024 };
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("transfer.json"), serde_json::to_string(&custom).unwrap()).unwrap();
        let c2 = load_or_create_transfer_config(&tmp);
        assert_eq!(c2.conns, 2);
        assert_eq!(c2.chunk_size, 256 * 1024);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
