import { useEffect } from "react";
import type { RequestView } from "../hooks/useTransfer";
import { CheckIcon, XIcon, FileIcon } from "./Icons";

function initials(name: string): string {
  const t = name.trim();
  if (!t) return "?";
  return t[0].toUpperCase();
}

function formatSize(bytes: number): { value: string; unit: string } {
  if (bytes >= 1024 * 1024 * 1024) {
    return { value: (bytes / (1024 ** 3)).toFixed(2), unit: "GB" };
  }
  if (bytes >= 1024 * 1024) {
    return { value: (bytes / (1024 ** 2)).toFixed(1), unit: "MB" };
  }
  if (bytes >= 1024) {
    return { value: (bytes / 1024).toFixed(0), unit: "KB" };
  }
  return { value: String(bytes), unit: "B" };
}

export function IncomingRequest({
  req,
  onRespond,
}: {
  req: RequestView | null;
  onRespond: (accept: boolean) => void;
}) {
  useEffect(() => {
    if (!req) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onRespond(false);
      else if (e.key === "Enter") onRespond(true);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [req, onRespond]);

  if (!req) return null;
  const size = formatSize(req.size);

  return (
    <div
      className="modal-backdrop"
      onClick={(e) => { if (e.target === e.currentTarget) onRespond(false); }}
    >
      <div className="modal" role="dialog" aria-modal>
        <div className="modal-header">
          <div className="modal-avatar">{initials(req.sender_name)}</div>
          <div className="modal-title">{req.sender_name}</div>
          <div className="modal-sub">想要发送文件给你</div>
        </div>

        <div className="modal-stats">
          <div className="modal-stat">
            <FileIcon size={18} />
            <div className="v">{req.count}</div>
            <div className="l">个文件</div>
          </div>
          <div className="modal-stat">
            <div className="v">{size.value}</div>
            <div className="l">{size.unit}</div>
          </div>
        </div>

        <div className="modal-actions">
          <button className="btn btn-secondary" onClick={() => onRespond(false)}>
            <XIcon size={14} />拒绝
          </button>
          <button className="btn btn-primary" onClick={() => onRespond(true)} autoFocus>
            <CheckIcon size={14} />接受
          </button>
        </div>
      </div>
    </div>
  );
}
