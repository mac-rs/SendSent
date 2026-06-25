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
    Ok(home_dir().unwrap_or_else(|| std::env::current_dir().unwrap_or_default()).join("Downloads").join("sendsent"))
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
}
