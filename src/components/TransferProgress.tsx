import { useEffect, useState } from "react";
import type { ProgressView } from "../hooks/useTransfer";
import { CheckIcon, XIcon, SendIcon } from "./Icons";

function fmtSize(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${bytes} B`;
}

function fmtSpeed(bps: number): string {
  if (bps >= 1024 ** 2) return `${(bps / 1024 ** 2).toFixed(1)} MB/s`;
  if (bps >= 1024) return `${(bps / 1024).toFixed(0)} KB/s`;
  return `${bps} B/s`;
}

function fmtElapsed(ms: number): string {
  if (ms < 1000) return "0秒";
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  if (m > 0) return `${m}分${s % 60}秒`;
  return `${s}秒`;
}

function Elapsed({ start }: { start: number }) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 500);
    return () => clearInterval(t);
  }, []);
  return <>{fmtElapsed(now - start)}</>;
}

function ItemNames({ p }: { p: ProgressView }) {
  const names = p.filenames ?? [];
  if (names.length === 0) {
    return (
      <span className="transfer-name">
        {p.files_total} 个文件 · {fmtSize(p.bytes_total)}
      </span>
    );
  }
  if (names.length === 1) {
    return <span className="transfer-name">{names[0]}</span>;
  }
  return (
    <span className="transfer-name" title={names.join(", ")}>
      {names[0]} <span className="muted">等 {names.length} 个文件</span>
    </span>
  );
}

function TransferRow({ p, compact }: { p: ProgressView; compact?: boolean }) {
  const pct = p.bytes_total ? Math.min((p.bytes_done / p.bytes_total) * 100, 100) : 0;
  const stateClass = p.done ? (p.error ? "failed" : "done") : "";
  const statusText = p.error
    ? (p.error === "rejected" ? "已拒绝" : p.error === "cancelled" ? "已取消" : `失败:${p.error}`)
    : p.done
      ? "完成"
      : "传输中";
  const elapsed = p.ended_at && p.started_at ? p.ended_at - p.started_at : 0;

  return (
    <div className={`transfer ${stateClass} fade-in`}>
      <div className="transfer-head">
        <div className="transfer-icon">
          {p.done
            ? (p.error ? <XIcon size={16} /> : <CheckIcon size={16} />)
            : <SendIcon size={16} />}
        </div>
        <div className="transfer-info">
          <ItemNames p={p} />
          <div className="transfer-sub">
            {p.files_total > 1 && `${p.files_done}/${p.files_total} · `}
            {fmtSize(p.bytes_done)} / {fmtSize(p.bytes_total)}
          </div>
        </div>
        <div className="transfer-pct">{pct.toFixed(0)}%</div>
      </div>

      <div className="transfer-bar">
        <div
          className="transfer-bar-fill"
          style={{ width: `${pct > 0 ? Math.max(pct, 2) : 0}%` }}
        />
      </div>

      {!compact && (
        <div className="transfer-meta">
          {!p.done && p.speed_bps > 0 && (
            <span className="speed">{fmtSpeed(p.speed_bps)}</span>
          )}
          <span>
            {p.done && p.started_at && p.ended_at
              ? `耗时 ${fmtElapsed(elapsed)}`
              : p.started_at
                ? <Elapsed start={p.started_at} />
                : statusText}
          </span>
          <span className="spacer" />
          <span className="dim">{statusText}</span>
        </div>
      )}
    </div>
  );
}

export function TransferProgress({
  items,
  compact,
}: {
  items: ProgressView[];
  compact?: boolean;
}) {
  if (items.length === 0) {
    if (compact) return null;
    return (
      <div className="card">
        <div className="transfer-empty">
          暂无传输记录,发送的文件会显示在这里
        </div>
      </div>
    );
  }

  return (
    <div className={`transfers${compact ? " compact" : ""}`}>
      {items.map((p) => (
        <TransferRow key={p.session_id} p={p} compact={compact} />
      ))}
    </div>
  );
}
