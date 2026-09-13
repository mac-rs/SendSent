import { useEffect, useRef, useState } from "react";
import { revealItemInDir, openPath } from "@tauri-apps/plugin-opener";
import type { ProgressView } from "../hooks/useTransfer";
import type { HistoryRecord } from "../lib/types";
import { usePlatform } from "../lib/platform";
import { CheckIcon, XIcon, SendIcon, TrashIcon, ArrowUpIcon, ArrowDownIcon } from "./Icons";

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

function DirBadge({ dir }: { dir?: "send" | "recv" }) {
  if (!dir) return null;
  const send = dir === "send";
  return (
    <span className={`dir-badge ${send ? "send" : "recv"}`}>
      {send ? <ArrowUpIcon size={12} /> : <ArrowDownIcon size={12} />}
      {send ? "发送" : "接收"}
    </span>
  );
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
  const stateClass = p.done
    ? (p.error ? "failed" : "done")
    : "in-progress";
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
            : (p.direction === "recv" ? <ArrowDownIcon size={16} /> : <SendIcon size={16} />)}
        </div>
        <div className="transfer-info">
          <ItemNames p={p} />
          <div className="transfer-sub">
            <DirBadge dir={p.direction} />
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

function HistoryRow({ r }: { r: HistoryRecord }) {
  const names = r.files.map((f) => f.name);
  const { platform } = usePlatform();
  const isMobile = platform === "ios" || platform === "android";
  const canReveal = r.direction === "recv" && r.status === "completed" && !!r.save_dir && !isMobile;
  const reveal = () => {
    if (!r.save_dir) return;
    const first = r.files[0]?.rel_path;
    const target = first ? `${r.save_dir}/${first}` : r.save_dir;
    revealItemInDir(target).catch(() => openPath(r.save_dir!));
  };
  const statusText =
    r.status === "completed" ? "完成"
      : r.status === "rejected" ? "已拒绝"
      : r.status === "cancelled" ? "已取消"
      : "失败";
  return (
    <div className={`transfer ${r.status === "completed" ? "done" : "failed"} fade-in`}>
      <div className="transfer-head">
        <div className="transfer-icon">
          {r.status === "completed" ? <CheckIcon size={16} /> : <XIcon size={16} />}
        </div>
        <div className="transfer-info">
          <span className="transfer-name">
            {names.length <= 1 ? names[0] : `${names[0]} 等 ${names.length} 个文件`}
          </span>
          <div className="transfer-sub">
            <DirBadge dir={r.direction} />
            {r.peer_name} · {fmtSize(r.bytes_done)} / {fmtSize(r.total_size)}
          </div>
        </div>
        <div className="transfer-pct">{statusText}</div>
      </div>
      {canReveal && (
        <div className="transfer-meta">
          <button className="btn btn-ghost" onClick={reveal}>在文件夹中显示</button>
        </div>
      )}
    </div>
  );
}

// 长按阈值 (ms) - 接近 iOS 标准 (0.5s)
const LONG_PRESS_MS = 520;

function dayLabel(ms: number): string {
  const d = new Date(ms);
  const now = new Date();
  const same = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
  const y = new Date(now);
  y.setDate(now.getDate() - 1);
  if (same(d, now)) return "今天";
  if (same(d, y)) return "昨天";
  return `${d.getMonth() + 1}月${d.getDate()}日`;
}

function groupByDay(items: HistoryRecord[]): { label: string; items: HistoryRecord[] }[] {
  const out: { label: string; items: HistoryRecord[] }[] = [];
  for (const r of items) {
    const label = dayLabel(r.ended_at_ms);
    const last = out[out.length - 1];
    if (last && last.label === label) last.items.push(r);
    else out.push({ label, items: [r] });
  }
  return out;
}

export function TransferHistory({
  items,
  onClear,
}: {
  items: HistoryRecord[];
  onClear: () => void;
}) {
  const { platform } = usePlatform();
  const isMobile = platform === "ios" || platform === "android";
  const [pressed, setPressed] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const timerRef = useRef<number | null>(null);
  const startPos = useRef<{ x: number; y: number } | null>(null);

  const clearTimer = () => {
    if (timerRef.current != null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  };

  const onPressStart = (e: React.PointerEvent<HTMLElement>) => {
    if (!isMobile) return;
    startPos.current = { x: e.clientX, y: e.clientY };
    setPressed(true);
    setConfirming(false);
    clearTimer();
    timerRef.current = window.setTimeout(() => {
      // 触发"长按清空"反馈
      setPressed(false);
      setConfirming(true);
      // 触觉反馈(iOS Taptic / Android Vibrate)
      try { navigator.vibrate?.(12); } catch { /* ignore */ }
    }, LONG_PRESS_MS);
  };

  const onPressMove = (e: React.PointerEvent<HTMLElement>) => {
    if (!isMobile || !startPos.current) return;
    const dx = Math.abs(e.clientX - startPos.current.x);
    const dy = Math.abs(e.clientY - startPos.current.y);
    // 拖动超过 10px 视为滚动,取消长按
    if (dx > 10 || dy > 10) {
      clearTimer();
      setPressed(false);
    }
  };

  const onPressEnd = () => {
    clearTimer();
    setPressed(false);
    if (!isMobile) return;
    // 短按不触发任何动作
  };

  // 触发真正清空
  const commitClear = () => {
    setConfirming(false);
    onClear();
  };

  if (items.length === 0) {
    return (
      <div className="card">
        <div className="transfer-empty">暂无历史记录</div>
      </div>
    );
  }

  return (
    <div
      className={`transfers history${pressed ? " longpressing" : ""}`}
      onPointerDown={onPressStart}
      onPointerMove={onPressMove}
      onPointerUp={onPressEnd}
      onPointerCancel={onPressEnd}
      onPointerLeave={onPressEnd}
    >
      <div className="history-actions">
        {isMobile ? (
          <span className="history-hint">长按列表可清空记录</span>
        ) : (
          <button className="btn btn-ghost" onClick={onClear}>清空记录</button>
        )}
      </div>
      {groupByDay(items).map((g) => (
        <div className="history-day" key={g.label}>
          <div className="day-header">
            <span>{g.label}</span>
            <span className="dim">{g.items.length} 项</span>
          </div>
          {g.items.map((r) => <HistoryRow key={r.session_id} r={r} />)}
        </div>
      ))}

      {confirming && (
        <div className="history-confirm-backdrop" onClick={() => setConfirming(false)}>
          <div className="history-confirm" onClick={(e) => e.stopPropagation()}>
            <div className="history-confirm-icon">
              <TrashIcon size={28} />
            </div>
            <div className="history-confirm-title">清空全部历史记录?</div>
            <div className="history-confirm-sub">共 {items.length} 条,清空后无法恢复</div>
            <div className="history-confirm-actions">
              <button
                className="btn btn-ghost"
                onClick={() => setConfirming(false)}
              >
                取消
              </button>
              <button
                className="btn btn-danger"
                onClick={commitClear}
                autoFocus
              >
                清空
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
