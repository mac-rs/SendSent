import { open } from "@tauri-apps/plugin-dialog";
import { sendFiles, pickFilesIos } from "../lib/invoke";
import type { Peer } from "../lib/types";
import { useState } from "react";

export function FilePicker({ peers }: { peers: Peer[] }) {
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");
  const [secure, setSecure] = useState(false);
  const [verify, setVerify] = useState(false);

  async function pick() {
    if (peers.length === 0) { setStatus("请先选择至少一台设备"); return; }
    try {
      const selected = await open({ multiple: true, directory: false });
      if (!selected || (Array.isArray(selected) && selected.length === 0)) return;
      const files = Array.isArray(selected) ? selected : [selected];
      setBusy(true);
      setStatus(`发送 ${files.length} 文件到 ${peers.length} 设备…`);
      for (const p of peers) await sendFiles(p.device_id, files, secure, verify);
      setStatus("已发起");
    } catch (e: any) { setStatus("出错: " + String(e)); }
    finally { setBusy(false); }
  }

  const names = peers.map((p) => p.name).join(", ");

  return (
    <div className="drop-zone" onClick={busy ? undefined : pick}>
      <div className="drop-zone-icon">📁</div>
      <div className="drop-zone-text">
        {peers.length === 0 ? "请先选择设备" : `发到 ${names}`}
      </div>
      <button className="drop-zone-btn" disabled={peers.length === 0 || busy} onClick={(e) => { e.stopPropagation(); pick(); }}>
        {busy ? "发送中…" : "选择文件"}
      </button>
      <button className="drop-zone-btn" style={{ marginLeft: 8, background: "var(--bg-secondary)", color: "var(--text)", border: "1px solid var(--border)" }} disabled={peers.length === 0 || busy}
        onClick={async (e) => {
          e.stopPropagation();
          try {
            const iosFiles = await pickFilesIos();
            if (!iosFiles || iosFiles.length === 0) return;
            setBusy(true);
            setStatus(`发送 ${iosFiles.length} 文件到 ${peers.length} 设备…`);
            for (const p of peers) await sendFiles(p.device_id, iosFiles, secure, verify);
            setStatus("已发起");
          } catch (err: any) { if (!String(err).includes("iOS only")) setStatus("出错: " + String(err)); }
          finally { setBusy(false); }
        }}
      >📱 从手机选择</button>
      <label className="secure-row" onClick={(e) => e.stopPropagation()}>
        <input type="checkbox" checked={secure} onChange={(e) => setSecure(e.target.checked)} />加密
        <input type="checkbox" checked={verify} onChange={(e) => setVerify(e.target.checked)} style={{ marginLeft: 8 }} />sha256 校验
      </label>
      {status && <span className="status-text">{status}</span>}
    </div>
  );
}
