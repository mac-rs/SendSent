import { invoke } from "@tauri-apps/api/core";
import type { Identity, Peer } from "./types";

export const getIdentity = () => invoke<Identity>("get_identity");
export const setDisplayName = (name: string) => invoke<void>("set_display_name", { name });
export const listPeers = () => invoke<Peer[]>("list_peers");
export const sendFiles = (peer_device_id: string, files: string[], secure: boolean = false) =>
  invoke<string>("send_files", { peerDeviceId: peer_device_id, files, secure });
export const respond = (session_id: string, accept: boolean, save_dir?: string, pin?: string) =>
  invoke<void>("respond", { sessionId: session_id, accept, saveDir: save_dir, pin });
export const cancel = (session_id: string) => invoke<void>("cancel", { sessionId: session_id });
export const getDefaultSaveDir = () => invoke<string>("get_default_save_dir");
