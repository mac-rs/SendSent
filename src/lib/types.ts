export type Platform = "macos" | "windows" | "linux" | "ios" | "android";

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
