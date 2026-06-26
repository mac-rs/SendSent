import { open } from "@tauri-apps/plugin-dialog";
import { sendFiles } from "../lib/invoke";
import type { Peer } from "../lib/types";
import { useState } from "react";

export function FilePicker({ peers }: { peers: Peer[] }) {
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string>("");
  const [secure, setSecure] = useState(false);

  async function pick() {
    if (peers.length === 0) { setStatus("未选择 peer"); return; }
    setStatus("打开选择器…");
    setStatus("打开选择器…");
    let selected;
    try {
      selected = await open({ multiple: true, directory: false });
    } catch (e) {
      console.error("[FilePicker] open() 抛错", e);
      setStatus("选择器错误: " + String(e));
      return;
    }
    console.log("[FilePicker] open() 返回", selected);
    if (!selected || (Array.isArray(selected) && selected.length === 0)) {
      setStatus("未选文件");
      return;
    }
    const files = Array.isArray(selected) ? selected : [selected];
    setBusy(true);
    setStatus(`发送 ${files.length} 个文件到 ${peers.length} 个设备…`);
    try {
      for (const p of peers) {
        await sendFiles(p.device_id, files, secure);
      }
      setStatus("已发起");
    } catch (e) {
      console.error("[FilePicker] sendFiles 抛错", e);
      setStatus("发送错误: " + String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div>
      <button disabled={peers.length === 0 || busy} onClick={pick}>
        选择文件发送{peers.length > 0 ? ` → ${peers.map(p=>p.name).join(", ")}` : ""}
      </button>
      <label style={{ marginLeft: 12 }}>
        <input type="checkbox" checked={secure} onChange={(e) => setSecure(e.target.checked)} />
        加密
      </label>
      {status ? <span style={{ marginLeft: 12 }}>{status}</span> : null}
    </div>
  );
}
