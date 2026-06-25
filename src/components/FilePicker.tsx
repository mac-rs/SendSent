import { open } from "@tauri-apps/plugin-dialog";
import { sendFiles } from "../lib/invoke";
import type { Peer } from "../lib/types";
import { useState } from "react";

export function FilePicker({ peer }: { peer: Peer | null }) {
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string>("");

  async function pick() {
    console.log("[FilePicker] pick clicked; peer=", peer);
    if (!peer) {
      setStatus("未选择 peer");
      return;
    }
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
    setStatus(`发送 ${files.length} 个文件…`);
    try {
      const sid = await sendFiles(peer.device_id, files);
      console.log("[FilePicker] sendFiles ->", sid);
      setStatus("已发起 (session " + sid + ")");
    } catch (e) {
      console.error("[FilePicker] sendFiles 抛错", e);
      setStatus("发送错误: " + String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div>
      <button disabled={!peer || busy} onClick={pick}>
        选择文件发送{peer ? ` → ${peer.name}` : ""}
      </button>
      {status ? <span style={{ marginLeft: 12 }}>{status}</span> : null}
    </div>
  );
}
