// 显示我的设备 QR 对话框
// 真实可扫描 QR 由 Rust 后端 `get_my_qr()` 生成(包含所有本地 IP),
// 前端在多网卡环境下可切换要展示的 IP,其他设备扫码即 addPeer。
//
// sendsent:// 协议 payload:
//   sendsent://<name>?addr=<ip>:<port>&dir=<save_dir>

import { useEffect, useState } from "react";
import { XIcon, CopyIcon, CheckIcon, QrCodeIcon, WifiIcon } from "./Icons";
import { getMyAddresses, getMyQr } from "../lib/invoke";

type Address = { interface: string; ip: string };

export function MyQrDialog({
  open,
  onClose,
  identity,
  saveDir,
  port,
}: {
  open: boolean;
  onClose: () => void;
  identity: { name: string } | null;
  saveDir: string;
  port: number;
}) {
  const [pngBase64, setPngBase64] = useState<string | null>(null);
  const [addrs, setAddrs] = useState<Address[]>([]);
  const [selectedIp, setSelectedIp] = useState<string>("");
  const [qrError, setQrError] = useState<string | null>(null);
  const [copied, setCopied] = useState<"text" | "qr" | null>(null);

  // 加载 IP 列表 + 当前端口 → 重新生成 QR
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    (async () => {
      try {
        const list = await getMyAddresses();
        if (cancelled) return;
        setAddrs(list);
        // 优先用之前的选中 IP,否则取第一个
        setSelectedIp((cur) => {
          if (cur && list.some((a) => a.ip === cur)) return cur;
          return list[0]?.ip ?? "";
        });
      } catch (e) {
        if (!cancelled) setQrError(`枚举网卡失败: ${String(e)}`);
      }
    })();
    return () => { cancelled = true; };
  }, [open]);

  // 选中 IP 变化时,重新拉 PNG
  useEffect(() => {
    if (!open || !selectedIp) return;
    let cancelled = false;
    setQrError(null);
    setPngBase64(null);
    (async () => {
      try {
        const b64 = await getMyQr(300, selectedIp || undefined);
        if (!cancelled) setPngBase64(b64);
      } catch (e) {
        if (!cancelled) setQrError(String(e));
      }
    })();
    return () => { cancelled = true; };
  }, [open, selectedIp]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  const payload = (() => {
    if (!identity) return "";
    const ip = selectedIp || (addrs[0]?.ip ?? "0.0.0.0");
    return `sendsent://${encodeURIComponent(identity.name)}?addr=${ip}:${port}&dir=${encodeURIComponent(saveDir)}`;
  })();

  const copyText = async () => {
    if (!payload) return;
    try {
      await navigator.clipboard.writeText(payload);
      setCopied("text");
      setTimeout(() => setCopied(null), 1500);
    } catch { /* ignore */ }
  };

  const copyQrImage = async () => {
    if (!pngBase64) return;
    try {
      const bin = atob(pngBase64);
      const arr = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
      const blob = new Blob([arr], { type: "image/png" });
      await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
      setCopied("qr");
      setTimeout(() => setCopied(null), 1500);
    } catch { /* ignore */ }
  };

  if (!open) return null;

  return (
    <div
      className="myqr-backdrop"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
      role="presentation"
    >
      <div className="myqr-dialog" role="dialog" aria-modal aria-labelledby="myqr-title">
        <div className="myqr-header">
          <h3 id="myqr-title">
            <QrCodeIcon size={16} />
            <span>我的设备 QR</span>
          </h3>
          <button className="icon-btn" onClick={onClose} aria-label="关闭">
            <XIcon size={16} />
          </button>
        </div>
        <div className="myqr-body">
          <div className="myqr-canvas-wrap">
            {pngBase64 ? (
              <img
                className="myqr-canvas"
                src={`data:image/png;base64,${pngBase64}`}
                alt={`${identity?.name ?? ""} QR`}
                width={220}
                height={220}
              />
            ) : (
              <div className="myqr-canvas myqr-canvas-placeholder">
                {qrError ?? "生成中…"}
              </div>
            )}
            <div className="myqr-canvas-legend">
              扫描后自动添加 · {addrs.length} 个可用地址
            </div>
          </div>
          <div className="myqr-info">
            <div className="myqr-name">{identity?.name ?? "未命名"}</div>
            <div className="myqr-meta">
              <span>端口 {port}</span>
              <span>·</span>
              <span>接收 {saveDir}</span>
            </div>
            {addrs.length > 0 && (
              <div className="myqr-ips" role="radiogroup" aria-label="选择要展示的 IP">
                <div className="myqr-ips-label">
                  <WifiIcon size={12} />
                  <span>本机 IP · 点选可切换</span>
                </div>
                <div className="myqr-ips-list">
                  {addrs.map((a) => {
                    const isSel = a.ip === selectedIp;
                    return (
                      <button
                        key={`${a.interface}-${a.ip}`}
                        type="button"
                        className={`myqr-ip-chip${isSel ? " active" : ""}`}
                        onClick={() => setSelectedIp(a.ip)}
                        role="radio"
                        aria-checked={isSel}
                        title={a.interface}
                      >
                        <span className="myqr-ip-iface">{a.interface}</span>
                        <span className="myqr-ip-addr">{a.ip}</span>
                      </button>
                    );
                  })}
                </div>
              </div>
            )}
            <div className="myqr-payload" title={payload}>
              {payload}
            </div>
            <div className="myqr-actions">
              <button className="btn btn-secondary" onClick={copyText}>
                {copied === "text" ? <CheckIcon size={14} /> : <CopyIcon size={14} />}
                <span>{copied === "text" ? "已复制" : "复制 payload"}</span>
              </button>
              <button
                className="btn btn-secondary"
                onClick={copyQrImage}
                disabled={!pngBase64}
              >
                {copied === "qr" ? <CheckIcon size={14} /> : <CopyIcon size={14} />}
                <span>{copied === "qr" ? "已复制" : "复制图片"}</span>
              </button>
            </div>
          </div>
        </div>
        <div className="myqr-hint">
          在另一台设备的 SendSent 中打开"添加设备 → 扫码添加"扫描此 QR,即可连接
        </div>
      </div>
    </div>
  );
}
