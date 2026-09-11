use crate::discovery::Peer;
use crate::proto::messages::{ErrorCode, Manifest};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

pub mod name {
    pub const PEER_FOUND: &str = "peer://found";
    pub const PEER_LOST: &str = "peer://lost";
    pub const TRANSFER_REQUEST: &str = "transfer://request";
    pub const TRANSFER_PROGRESS: &str = "transfer://progress";
    pub const TRANSFER_FINISHED: &str = "transfer://finished";
    pub const TRANSFER_HISTORY: &str = "transfer://history";
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionState { Connecting, AwaitingAccept, Transferring, Finalizing }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FinishedState { Completed, Rejected, Cancelled, Failed }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TransferEvent {
    Request { session_id: Uuid, sender: Peer, manifest: Manifest },
    Progress { session_id: Uuid, state: SessionState,
               bytes_done: u64, bytes_total: u64, files_done: u64, files_total: u64, speed_bps: u64 },
    Finished { session_id: Uuid, state: FinishedState, error: Option<ErrorPayload> },
    Recorded(crate::history::HistoryRecord),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload { pub code: ErrorCode, pub message: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerFoundPayload { pub peer: Peer }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerLostPayload { pub device_id: String }
