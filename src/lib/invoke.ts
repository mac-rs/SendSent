import { invoke } from "@tauri-apps/api/core";
import type { Identity, Peer } from "./types";

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
