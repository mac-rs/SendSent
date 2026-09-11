import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { sendFiles, pickFilesIos } from "../lib/invoke";
import { registerSendFilenames } from "../hooks/useTransfer";
import type { Peer } from "../lib/types";
import { FileIcon, PhoneIcon, ShieldIcon, HashIcon, SendIcon } from "./Icons";

export function FilePicker({ peers }: { peers: Peer[] }) {
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<{ type: "ok" | "err"; text: string } | null>(null);
  const [secure, setSecure] = useState(false);
  const [verify, setVerify] = useState(false);

  const disabled = peers.length === 0 || busy;
  const names = peers.map((p) => p.name).join("、");

  async function sendPaths(paths: string[]) {
    if (paths.length === 0) return;
    const filenames = paths.map((f) => f.split("/").pop() ?? f);
    setBusy(true);
    setStatus({ type: "ok", text: `发送 ${paths.length} 个文件到 ${peers.length} 台设备…` });
    try {
      for (const p of peers) {
        const sid = await sendFiles(p.device_id, paths, secure, verify);
        registerSendFilenames(sid, filenames);
      }
      setStatus({ type: "ok", text: "已发送" });
    } catch (e) {
      setStatus({ type: "err", text: "发送失败: " + String(e) });
    } finally {
      setBusy(false);
    }
  }

  async function pickDesktop() {
    if (peers.length === 0) { setStatus({ type: "err", text: "请先选择至少一台设备" }); return; }
    try {
      const sel = await open({ multiple: true, directory: false });
      if (!sel || (Array.isArray(sel) && sel.length === 0)) return;
      const files = Array.isArray(sel) ? sel : [sel];
      await sendPaths(files);
    } catch (e) {
      setStatus({ type: "err", text: "选择文件失败: " + String(e) });
    }
  }

  async function pickIos() {
    if (peers.length === 0) { setStatus({ type: "err", text: "请先选择至少一台设备" }); return; }
    try {
      const iosFiles = await pickFilesIos();
      if (!iosFiles || iosFiles.length === 0) return;
      await sendPaths(iosFiles);
    } catch (e) {
      if (!String(e).includes("iOS only")) {
        setStatus({ type: "err", text: String(e) });
      }
    }
  }

  return (
    <div className="col">
      <div
        className="dropzone"
        onClick={busy ? undefined : pickDesktop}
        role="button"
        aria-disabled={busy}
        tabIndex={0}
        onKeyDown={(e) => { if ((e.key === "Enter" || e.key === " ") && !busy) pickDesktop(); }}
      >
        <div className="dropzone-icon"><FileIcon size={28} /></div>
        <div className="dropzone-title">
          {peers.length === 0
            ? "请先选择目标设备"
            : `发送到 ${names}`}
        </div>
        <div className="dropzone-hint">
          {busy ? "正在发送…" : "点击选择文件,或拖入文件"}
        </div>
        <div className="dropzone-actions">
          <button
            className="btn btn-primary"
            disabled={disabled}
            onClick={(e) => { e.stopPropagation(); pickDesktop(); }}
          >
            <SendIcon size={14} />
            {busy ? "发送中…" : "选择文件发送"}
          </button>
          <button
            className="btn btn-secondary"
            disabled={disabled}
            onClick={(e) => { e.stopPropagation(); pickIos(); }}
            title="从 iOS 设备选择"
          >
            <PhoneIcon size={14} />从手机选择
          </button>
        </div>
      </div>

      <div className="options-row">
        <label className="toggle">
          <input type="checkbox" checked={secure} onChange={(e) => setSecure(e.target.checked)} />
          <span className="switch" />
          <ShieldIcon size={14} />加密传输
        </label>
        <label className="toggle">
          <input type="checkbox" checked={verify} onChange={(e) => setVerify(e.target.checked)} />
          <span className="switch" />
          <HashIcon size={14} />SHA-256 校验
        </label>
      </div>

      {status && (
        <div className={`toast-msg${status.type === "err" ? " error" : ""}`}>
          {status.text}
        </div>
      )}
    </div>
  );
}
