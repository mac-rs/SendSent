import { open } from "@tauri-apps/plugin-dialog";
import { sendFiles } from "../lib/invoke";
import type { Peer } from "../lib/types";
import { useState } from "react";

export function FilePicker({ peer }: { peer: Peer | null }) {
  const [busy, setBusy] = useState(false);
  async function pick() {
    if (!peer) return;
    const selected = await open({ multiple: true, directory: false });
    if (!selected || (Array.isArray(selected) && selected.length === 0)) return;
    const files = Array.isArray(selected) ? selected : [selected];
    setBusy(true);
    try { await sendFiles(peer.device_id, files); } finally { setBusy(false); }
  }
  return <button disabled={!peer || busy} onClick={pick}>选择文件发送{peer ? ` → ${peer.name}` : ""}</button>;
}
