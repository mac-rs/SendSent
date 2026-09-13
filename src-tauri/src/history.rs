//! Persistent transfer history, stored as JSON in the app data dir.

use crate::discovery::Platform;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MAX_RECORDS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Send,
    Recv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryStatus {
    Completed,
    Failed,
    Rejected,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryFile {
    pub name: String,
    pub size: u64,
    pub rel_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub session_id: String,
    pub direction: Direction,
    pub peer_name: String,
    pub peer_platform: Platform,
    pub files: Vec<HistoryFile>,
    pub total_size: u64,
    pub bytes_done: u64,
    pub status: HistoryStatus,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub save_dir: Option<String>,
    pub error: Option<String>,
}

pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[allow(clippy::too_many_arguments)]
pub fn make_record(
    direction: Direction,
    session_id: &str,
    peer_name: &str,
    peer_platform: Platform,
    files: Vec<HistoryFile>,
    total_size: u64,
    bytes_done: u64,
    status: HistoryStatus,
    started_at_ms: i64,
    save_dir: Option<String>,
    error: Option<String>,
) -> HistoryRecord {
    HistoryRecord {
        session_id: session_id.to_string(),
        direction,
        peer_name: peer_name.to_string(),
        peer_platform,
        files,
        total_size,
        bytes_done,
        status,
        started_at_ms,
        ended_at_ms: now_ms(),
        save_dir,
        error,
    }
}

pub struct HistoryStore {
    path: PathBuf,
    records: Vec<HistoryRecord>,
}

impl HistoryStore {
    pub fn load(dir: &Path) -> Self {
        let _ = std::fs::create_dir_all(dir);
        let path = dir.join("history.json");
        let records = match std::fs::read_to_string(&path) {
            Ok(s) => match serde_json::from_str::<Vec<HistoryRecord>>(&s) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("history.json parse failed ({e}); starting empty");
                    let _ = std::fs::rename(&path, dir.join("history.json.bad"));
                    Vec::new()
                }
            },
            Err(_) => Vec::new(),
        };
        let mut records = records;
        records.truncate(MAX_RECORDS);
        Self { path, records }
    }

    pub fn append(&mut self, record: HistoryRecord) {
        self.records.insert(0, record);
        self.records.truncate(MAX_RECORDS);
        self.persist();
    }

    pub fn list(&self) -> Vec<HistoryRecord> {
        self.records.clone()
    }

    pub fn clear(&mut self) {
        self.records.clear();
        self.persist();
    }

    /// 删除单条记录(按 session_id)。
    pub fn remove(&mut self, session_id: &str) {
        self.records.retain(|r| r.session_id != session_id);
        self.persist();
    }

    fn persist(&self) {
        match serde_json::to_string_pretty(&self.records) {
            Ok(s) => {
                if let Err(e) = std::fs::write(&self.path, s) {
                    tracing::warn!("history persist failed: {e}");
                }
            }
            Err(e) => tracing::warn!("history serialize failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn rec(id: &str) -> HistoryRecord {
        make_record(
            Direction::Recv, id, "peer", Platform::Ios,
            vec![HistoryFile { name: "a.bin".into(), size: 10, rel_path: "a.bin".into() }],
            10, 10, HistoryStatus::Completed, 1000, Some("/tmp/save".into()), None,
        )
    }

    fn tmp_dir() -> PathBuf {
        std::env::temp_dir().join(format!("ss-hist-{}", Uuid::new_v4()))
    }

    #[test]
    fn append_is_newest_first_and_persists() {
        let dir = tmp_dir();
        let mut store = HistoryStore::load(&dir);
        store.append(rec("one"));
        store.append(rec("two"));
        assert_eq!(store.list()[0].session_id, "two");
        let reloaded = HistoryStore::load(&dir);
        assert_eq!(reloaded.list().len(), 2);
        assert_eq!(reloaded.list()[1].session_id, "one");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn caps_at_max_records() {
        let dir = tmp_dir();
        let mut store = HistoryStore::load(&dir);
        for i in 0..(MAX_RECORDS + 5) {
            store.append(rec(&format!("id-{i}")));
        }
        assert_eq!(store.list().len(), MAX_RECORDS);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clear_empties_and_persists() {
        let dir = tmp_dir();
        let mut store = HistoryStore::load(&dir);
        store.append(rec("one"));
        store.clear();
        assert!(store.list().is_empty());
        assert!(HistoryStore::load(&dir).list().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_deletes_and_persists() {
        let dir = tmp_dir();
        let mut store = HistoryStore::load(&dir);
        store.append(rec("one"));
        store.append(rec("two"));
        store.remove("one");
        assert_eq!(store.list().len(), 1);
        assert_eq!(store.list()[0].session_id, "two");
        assert_eq!(HistoryStore::load(&dir).list().len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_file_falls_back_to_empty() {
        let dir = tmp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("history.json"), b"{not json").unwrap();
        assert!(HistoryStore::load(&dir).list().is_empty());
        assert!(dir.join("history.json.bad").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
