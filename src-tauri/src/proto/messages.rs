use serde::{Serialize, Deserialize};
use uuid::Uuid;

pub const MAGIC: u8 = 0x53;
pub const PROTO_VER: u8 = 1;
pub const MAX_CONTROL_PAYLOAD: u32 = 4 * 1024 * 1024;
pub const DATA_TAG: u8 = 0xD5;
pub const DEFAULT_CHUNK_SIZE: usize = 256 * 1024;
pub const MAX_DATA_PAYLOAD: u32 = 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Platform { Macos, Windows, Linux, Ios, Android, Unknown }

impl Platform {
    /// Map a platform label (as stored in `Identity.platform`) to the wire enum.
    pub fn from_label(s: &str) -> Self {
        match s {
            "macos" => Platform::Macos,
            "windows" => Platform::Windows,
            "linux" => Platform::Linux,
            "ios" => Platform::Ios,
            "android" => Platform::Android,
            _ => Platform::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum ErrorCode {
    IncompatibleVersion = 0, ManifestTooLarge = 1, ConnectionLost = 2,
    DiskFull = 3, WriteFailed = 4, ProtocolError = 5,
    Timeout = 6, Cancelled = 7, Internal = 8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum MsgType {
    Hello = 0x01, HelloAck = 0x02, Manifest = 0x03, Accept = 0x04,
    Reject = 0x05, Progress = 0x06, Complete = 0x07, Error = 0x08,
    Cancel = 0x09, DataOpen = 0x0A, PinCode = 0x0B, VerifyInfo = 0x0C,
}

impl TryFrom<u8> for MsgType {
    type Error = ();
    fn try_from(v: u8) -> Result<Self, ()> {
        Ok(match v {
            0x01 => MsgType::Hello, 0x02 => MsgType::HelloAck, 0x03 => MsgType::Manifest,
            0x04 => MsgType::Accept, 0x05 => MsgType::Reject, 0x06 => MsgType::Progress,
            0x07 => MsgType::Complete, 0x08 => MsgType::Error, 0x09 => MsgType::Cancel,
            0x0A => MsgType::DataOpen, 0x0B => MsgType::PinCode, 0x0C => MsgType::VerifyInfo, _ => return Err(()),
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileKind { File, Dir }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileMeta {
    pub id: Uuid, pub name: String, pub rel_path: String,
    pub size: u64, pub kind: FileKind, pub hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub session_id: Uuid, pub files: Vec<FileMeta>,
    pub total_size: u64, pub total_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hello { pub device_id: String, pub name: String, pub platform: Platform, pub session_id: Uuid, pub proto_ver: u8, pub secure: bool, pub verify: bool }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HelloAck { pub device_id: String, pub name: String, pub secure_ok: bool }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Accept { pub save_dir: String }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reject { pub reason: String }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Progress { pub file_id: Uuid, pub bytes_done: u64 }
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Complete;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorMsg { pub code: ErrorCode, pub message: String }
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Cancel;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataOpen { pub session_id: Uuid }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PinCode { pub pin: String }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerifyInfo { pub hashes: Vec<(Uuid, String)> } // (file_id, sha256 hex)

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn msgtype_roundtrip() {
        for b in [0x01u8, 0x05, 0x0A] {
            let t = MsgType::try_from(b).unwrap();
            assert_eq!(t as u8, b);
        }
        assert!(MsgType::try_from(0xFF).is_err());
    }
    #[test]
    fn platform_from_label() {
        assert_eq!(Platform::from_label("ios"), Platform::Ios);
        assert_eq!(Platform::from_label("android"), Platform::Android);
        assert_eq!(Platform::from_label("macos"), Platform::Macos);
        assert_eq!(Platform::from_label("nonsense"), Platform::Unknown);
    }
    #[test]
    fn manifest_bincode_roundtrip() {
        let m = Manifest {
            session_id: Uuid::new_v4(), files: vec![FileMeta {
                id: Uuid::new_v4(), name: "a.bin".into(), rel_path: "d/a.bin".into(),
                size: 10, kind: FileKind::File, hash: None,
            }], total_size: 10, total_count: 1,
        };
        let bytes = postcard::to_stdvec(&m).unwrap();
        let back: Manifest = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(m, back);
    }
}
