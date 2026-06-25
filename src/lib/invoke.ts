import { invoke } from "@tauri-apps/api/core";
import type { Identity, Peer } from "./types";

export const getIdentity = () => invoke<Identity>("get_identity");
export const setDisplayName = (name: string) => invoke<void>("set_display_name", { name });
export const listPeers = () => invoke<Peer[]>("list_peers");
export const sendFiles = (peer_device_id: string, files: string[]) =>
  invoke<string>("send_files", { peer_device_id, files });
export const respond = (session_id: string, accept: boolean, save_dir?: string) =>
  invoke<void>("respond", { session_id, accept, save_dir });
export const cancel = (session_id: string) => invoke<void>("cancel", { session_id });
export const getDefaultSaveDir = () => invoke<string>("get_default_save_dir");
