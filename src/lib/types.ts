export type Platform = "macos" | "windows" | "linux" | "ios" | "android" | "unknown";

export interface Peer {
  device_id: string;
  name: string;
  platform: Platform;
  proto_version: number;
  addrs: string[];
  port: number;
  last_seen_ms: number;
}

export interface Identity { device_id: string; name: string; platform: string; }

export type TransferDirection = "send" | "recv";
export type HistoryStatus = "completed" | "failed" | "rejected" | "cancelled";
export interface HistoryFile { name: string; size: number; rel_path: string }
export interface HistoryRecord {
  session_id: string;
  direction: TransferDirection;
  peer_name: string;
  peer_platform: Platform;
  files: HistoryFile[];
  total_size: number;
  bytes_done: number;
  status: HistoryStatus;
  started_at_ms: number;
  ended_at_ms: number;
  save_dir: string | null;
  error: string | null;
}

export type FileKind = "File" | "Dir";
export interface FileMeta {
  id: string; name: string; rel_path: string;
  size: number; kind: FileKind; hash: string | null;
}
export interface Manifest {
  session_id: string; files: FileMeta[]; total_size: number; total_count: number;
}

export type SessionState = "connecting" | "awaiting_accept" | "transferring" | "finalizing";
export type FinishedState = "completed" | "rejected" | "cancelled" | "failed";

export type TransferEvent =
  | { kind: "Request"; session_id: string; sender: Peer; manifest: Manifest }
  | { kind: "Progress"; session_id: string; state: SessionState;
      bytes_done: number; bytes_total: number; files_done: number; files_total: number; speed_bps: number }
  | { kind: "Finished"; session_id: string; state: FinishedState;
      error: { code: string; message: string } | null };
