import { invoke } from "@tauri-apps/api/core";
import type { HistoryRecord, Identity, Peer } from "./types";

export const getIdentity = () => invoke<Identity>("get_identity");
export const setDisplayName = (name: string) => invoke<void>("set_display_name", { name });
export const listPeers = () => invoke<Peer[]>("list_peers");
export const sendFiles = (peer_device_id: string, files: string[], secure: boolean = false, verify: boolean = false) =>
  invoke<string>("send_files", { peerDeviceId: peer_device_id, files, secure, verify });
export const respond = (session_id: string, accept: boolean, save_dir?: string, pin?: string) =>
  invoke<void>("respond", { sessionId: session_id, accept, saveDir: save_dir, pin });
export const cancel = (session_id: string) => invoke<void>("cancel", { sessionId: session_id });
export const getDefaultSaveDir = () => invoke<string>("get_default_save_dir");
export const addPeer = (address: string) => invoke<void>("add_peer", { address });
export const sendText = (peerDeviceId: string, text: string, secure: boolean = false, verify: boolean = false) =>
  invoke<string>("send_text", { peerDeviceId, text, secure, verify });
export const pickFilesIos = () => invoke<string[]>("pick_files_ios");
export const getTransferConfig = () => invoke<{ conns: number; chunk_size: number; split_threshold: number }>("get_transfer_config");
export const setTransferConfig = (conns: number, chunk_kb: number, split_mb: number) =>
  invoke<void>("set_transfer_config", { conns, chunkKb: chunk_kb, splitMb: split_mb });
export const listTransferHistory = () => invoke<HistoryRecord[]>("list_transfer_history");
export const clearTransferHistory = () => invoke<void>("clear_transfer_history");

// Android: `dialog.open()` does not resolve on the first call unless the
// backend keeps receiving IPC (tauri plugins-workspace #3366). Ping a no-op
// command while a native dialog is open.
export const noop = () => invoke<void>("noop");
export async function withBackendKeepAlive<T>(fn: () => Promise<T>): Promise<T> {
  const id = window.setInterval(() => { noop().catch(() => {}); }, 200);
  try {
    return await fn();
  } finally {
    window.clearInterval(id);
  }
}
export const getMyQr = (size?: number, ip?: string) =>
  invoke<string>("get_my_qr", { size, ip });
export const getMyAddresses = () =>
  invoke<Array<{ interface: string; ip: string }>>("get_my_addresses");

// 启动 splash screen:通知 Rust 端关闭 splash 窗口、显示主窗口。
// 安全无副作用,多次调用幂等(Rust 端 splash 关闭后 noop)。
export const splashReady = () => invoke<void>("splash_ready");
