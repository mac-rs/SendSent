// 添加设备面板
// 两种模式:
//   · input  - 手动输入 IP:port
//   · scan   - 调用摄像头 + BarcodeDetector 解码 QR
// 扫码成功会解析 sendsent:// 协议 payload 并自动 addPeer

import { useEffect, useRef, useState } from "react";
import { addPeer } from "../lib/invoke";
import { addRecentPeer, getRecentPeers, removeRecentPeer } from "../lib/recentPeers";
import {
  PlusIcon, XIcon, CheckIcon, KeyboardIcon,
  QrCodeIcon, HistoryIcon, TrashIcon, CameraIcon,
} from "./Icons";

type Mode = "input" | "scan";

// W3C BarcodeDetector 形状(types 暂未在 lib.dom 中定义)
type BarcodeDetectorLike = {
  detect(source: CanvasImageSource): Promise<Array<{ rawValue: string }>>;
};
type BarcodeDetectorCtor = new (opts: { formats: string[] }) => BarcodeDetectorLike;

declare global {
  interface Window {
    BarcodeDetector?: BarcodeDetectorCtor;
  }
}

function isBarcodeDetectorSupported(): boolean {
  return typeof window !== "undefined" && typeof window.BarcodeDetector === "function";
}

// 解析 sendsent://<name>?addr=<ip>:<port>&dir=<path>
// 兼容 addr 不带 port 的情况
function parseSendsentPayload(text: string): { addr: string; name?: string } | null {
  const t = text.trim();
  if (!t) return null;
  // 直接是 IP:port
  if (/^[0-9.]+:[0-9]+$/.test(t)) return { addr: t };
  // sendsent:// scheme
  if (t.startsWith("sendsent://")) {
    try {
      const u = new URL(t.replace("sendsent://", "http://"));
      const addr = u.searchParams.get("addr") ?? u.host;
      if (!addr) return null;
      return { addr, name: u.hostname || undefined };
    } catch {
      return null;
    }
  }
  return null;
}

export function AddDeviceSheet({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const [mode, setMode] = useState<Mode>("input");
  const [addr, setAddr] = useState("");
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<{ type: "ok" | "err"; text: string } | null>(null);
  const [scanMsg, setScanMsg] = useState<string | null>(null);
  const [scanState, setScanState] = useState<"idle" | "starting" | "scanning" | "unsupported" | "denied" | "error">("idle");
  const [recents, setRecents] = useState<string[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const rafRef = useRef<number | null>(null);
  const stoppedRef = useRef(false);

  useEffect(() => {
    if (open) {
      setAddr("");
      setMsg(null);
      setScanMsg(null);
      setMode("input");
      setRecents(getRecentPeers());
      setTimeout(() => inputRef.current?.focus(), 280);
    } else {
      stopScan();
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  // 切换到 scan 模式时启动摄像头
  useEffect(() => {
    if (mode === "scan" && open) {
      startScan();
    } else {
      stopScan();
    }
  }, [mode, open]);

  const submit = async () => {
    if (!addr.trim() || busy) return;
    setBusy(true);
    setMsg(null);
    try {
      await addPeer(addr.trim());
      addRecentPeer(addr.trim());
      setRecents(getRecentPeers());
      setMsg({ type: "ok", text: "已添加" });
      setTimeout(onClose, 600);
    } catch (e) {
      setMsg({ type: "err", text: String(e) });
    } finally {
      setBusy(false);
    }
  };

  const removeRecent = (a: string) => {
    removeRecentPeer(a);
    setRecents(getRecentPeers());
  };

  const stopScan = () => {
    stoppedRef.current = true;
    if (rafRef.current != null) {
      cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    }
    if (streamRef.current) {
      for (const t of streamRef.current.getTracks()) t.stop();
      streamRef.current = null;
    }
    if (videoRef.current) {
      videoRef.current.srcObject = null;
    }
  };

  const handleDecoded = async (raw: string) => {
    const parsed = parseSendsentPayload(raw);
    if (!parsed) {
      setScanState("error");
      setScanMsg(`无法解析: ${raw.slice(0, 60)}`);
      return;
    }
    stopScan();
    setScanState("scanning");
    setScanMsg(`识别成功 → ${parsed.addr}`);
    setBusy(true);
    try {
      await addPeer(parsed.addr);
      addRecentPeer(parsed.addr);
      setTimeout(onClose, 500);
    } catch (e) {
      setScanState("error");
      setScanMsg(`添加失败: ${String(e)}`);
    } finally {
      setBusy(false);
    }
  };

  const startScan = async () => {
    stoppedRef.current = false;
    setScanMsg(null);

    if (!isBarcodeDetectorSupported()) {
      setScanState("unsupported");
      setScanMsg("当前 WebView 不支持 BarcodeDetector,无法实时识别二维码");
      return;
    }
    if (!navigator.mediaDevices?.getUserMedia) {
      setScanState("unsupported");
      setScanMsg("当前环境不支持 getUserMedia");
      return;
    }

    setScanState("starting");
    let stream: MediaStream;
    try {
      stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: { ideal: "environment" } },
        audio: false,
      });
    } catch (e) {
      const err = e as DOMException;
      if (err.name === "NotAllowedError" || err.name === "SecurityError") {
        setScanState("denied");
        setScanMsg("相机权限被拒绝,请在系统设置中允许");
      } else {
        setScanState("error");
        setScanMsg(`相机启动失败: ${err.message || err.name}`);
      }
      return;
    }
    streamRef.current = stream;
    const video = videoRef.current;
    if (!video) return;
    video.srcObject = stream;
    try {
      await video.play();
    } catch {
      // iOS WKWebView may throw AbortError if interrupted; non-fatal
    }
    setScanState("scanning");

    const detector = new window.BarcodeDetector!({ formats: ["qr_code"] });
    const tick = async () => {
      if (stoppedRef.current) return;
      const v = videoRef.current;
      if (v && v.readyState >= 2) {
        try {
          const codes = await detector.detect(v);
          if (codes.length > 0) {
            handleDecoded(codes[0].rawValue);
            return;
          }
        } catch {
          // detect may throw on transient frames, keep looping
        }
      }
      rafRef.current = requestAnimationFrame(tick);
    };
    rafRef.current = requestAnimationFrame(tick);
  };

  if (!open) return null;

  return (
    <div
      className="sheet-backdrop"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
      role="presentation"
    >
      <div className="sheet" role="dialog" aria-modal aria-labelledby="add-device-title">
        <div className="sheet-handle" aria-hidden />
        <div className="sheet-header">
          <button className="sheet-close" onClick={onClose} aria-label="关闭">
            <XIcon size={18} />
          </button>
          <div id="add-device-title" className="sheet-title">添加设备</div>
          <button
            className="sheet-confirm"
            onClick={submit}
            disabled={mode !== "input" || !addr.trim() || busy}
            aria-label="确认"
          >
            <CheckIcon size={20} />
          </button>
        </div>

        <div className="sheet-mode-toggle" role="tablist">
          <button
            className={`sheet-mode${mode === "input" ? " active" : ""}`}
            onClick={() => setMode("input")}
            role="tab"
            aria-selected={mode === "input"}
          >
            <KeyboardIcon size={14} />
            <span>手动输入</span>
          </button>
          <button
            className={`sheet-mode${mode === "scan" ? " active" : ""}`}
            onClick={() => setMode("scan")}
            role="tab"
            aria-selected={mode === "scan"}
          >
            <CameraIcon size={14} />
            <span>扫码添加</span>
          </button>
        </div>

        <div className="sheet-body">
          {mode === "input" ? (
            <>
              <label className="field">
                <span className="field-label">IP:port</span>
                <input
                  ref={inputRef}
                  type="text"
                  inputMode="url"
                  autoComplete="off"
                  autoCorrect="off"
                  autoCapitalize="off"
                  spellCheck={false}
                  placeholder="192.168.1.5:52225"
                  value={addr}
                  onChange={(e) => { setAddr(e.target.value); setMsg(null); }}
                  onKeyDown={(e) => { if (e.key === "Enter") submit(); }}
                  disabled={busy}
                />
                <span className="field-hint">
                  设备必须运行 SendSent 并开启 mDNS,或手动输入其内网 IP 与端口
                </span>
              </label>

              {recents.length > 0 && (
                <div className="recent-list">
                  <div className="recent-list-header">
                    <HistoryIcon size={12} />
                    <span>最近添加</span>
                  </div>
                  <div className="recent-list-items">
                    {recents.map((r) => (
                      <div className="recent-item" key={r}>
                        <button
                          className="recent-item-main"
                          onClick={() => setAddr(r)}
                          type="button"
                        >
                          <span className="recent-item-text">{r}</span>
                        </button>
                        <button
                          className="recent-item-remove"
                          onClick={() => removeRecent(r)}
                          aria-label={`删除 ${r}`}
                          type="button"
                        >
                          <TrashIcon size={12} />
                        </button>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {msg && (
                <div className={`toast-msg${msg.type === "err" ? " error" : ""}`}>
                  {msg.text}
                </div>
              )}

              <div className="sheet-tip">
                <PlusIcon size={14} />
                <span>同一局域网下,设备通常会自动出现在列表中,无需手动添加</span>
              </div>
            </>
          ) : (
            <div className="scan-stage">
              <div className="scan-viewport">
                <video
                  ref={videoRef}
                  className="scan-video"
                  playsInline
                  muted
                  autoPlay
                />
                {scanState === "scanning" && (
                  <>
                    <div className="scan-corners" aria-hidden>
                      <span className="scan-corner scan-corner-tl" />
                      <span className="scan-corner scan-corner-tr" />
                      <span className="scan-corner scan-corner-bl" />
                      <span className="scan-corner scan-corner-br" />
                    </div>
                    <div className="scan-line" aria-hidden />
                  </>
                )}
                {scanState !== "scanning" && (
                  <div className="scan-placeholder">
                    <CameraIcon size={36} />
                    <p>
                      {scanState === "starting" && "正在启动相机…"}
                      {scanState === "unsupported" && "当前 WebView 不支持二维码识别"}
                      {scanState === "denied" && "请在系统设置中允许相机权限"}
                      {scanState === "error" && "相机出错"}
                      {scanState === "idle" && "将对方设备的 QR 对准取景框"}
                    </p>
                  </div>
                )}
              </div>
              {scanMsg && (
                <div className={`toast-msg${scanState === "error" || scanState === "denied" || scanState === "unsupported" ? " error" : ""}`}>
                  {scanMsg}
                </div>
              )}
              <div className="sheet-tip">
                <QrCodeIcon size={14} />
                <span>
                  在桌面端 SendSent "设置 → 关于"页或顶栏偏好面板中可显示当前设备 QR
                </span>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
