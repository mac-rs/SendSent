import type { ProgressView } from "../hooks/useTransfer";
import { useEffect, useState } from "react";

function fmt(bps: number) {
  if (bps >= 1_000_000) return `${(bps / 1_000_000).toFixed(1)} MB/s`;
  if (bps >= 1_000) return `${(bps / 1_000).toFixed(0)} KB/s`;
  return `${bps} B/s`;
}

function fmtElapsed(ms: number) {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  if (m > 0) return `${m}分${s % 60}秒`;
  return `${s}秒`;
}

function Elapsed({ start }: { start: number }) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, []);
  return <>{fmtElapsed(now - start)}</>;
}

export function TransferProgress({ items }: { items: ProgressView[] }) {
  if (items.length === 0) return null;
  return (
    <div className="progress-section">
      {items.map((p) => {
        const pct = p.bytes_total ? Math.min((p.bytes_done / p.bytes_total) * 100, 100) : 0;
        const cls = p.done ? (p.error ? "failed" : "done") : "active";
        return (
          <div key={p.session_id} className={`progress-item ${cls}`}>
            {/* filenames */}
            {p.filenames && p.filenames.length > 0 && (
              <div className="progress-files">{p.filenames.join(", ")}</div>
            )}

            {/* progress bar row */}
            <div className="progress-bar-row">
              <span className="progress-pct">{p.done ? "" : `${pct.toFixed(0)}%`}</span>
              <div className="progress-track">
                <div className={`progress-fill ${cls}`} style={{ width: `${Math.max(pct, 2)}%` }} />
              </div>
            </div>

            {/* info row: speed + elapsed */}
            <div className="progress-info">
              {!p.done && p.speed_bps > 0 && <span className="progress-speed">{fmt(p.speed_bps)}</span>}
              {p.started_at && <span className="progress-elapsed">{p.done ? "耗时 " : ""}<Elapsed start={p.started_at} /></span>}
            </div>

            {/* status */}
            {p.done && <div className="progress-status">{p.error ?? "完成"}</div>}
          </div>
        );
      })}
    </div>
  );
}
