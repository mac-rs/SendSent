import type { ProgressView } from "../hooks/useTransfer";

function fmt(bps: number) {
  if (bps >= 1_000_000) return `${(bps / 1_000_000).toFixed(1)} MB/s`;
  if (bps >= 1_000) return `${(bps / 1_000).toFixed(0)} KB/s`;
  return `${bps} B/s`;
}

export function TransferProgress({ items }: { items: ProgressView[] }) {
  if (items.length === 0) return null;
  return (
    <div className="progress-section">
      {items.map((p) => {
        const pct = p.bytes_total ? Math.min((p.bytes_done / p.bytes_total) * 100, 100) : 0;
        const cls = p.done
          ? (p.error ? "failed" : "done")
          : "active";
        return (
          <div key={p.session_id} className={`progress-item ${cls}`}>
            <span style={{ fontSize: 11, minWidth: 60, flexShrink: 0 }}>
              {p.done ? (p.error ? "失败" : "完成") : `${pct.toFixed(0)}%`}
            </span>
            <div className="progress-track">
              <div
                className={`progress-fill ${cls}`}
                style={{ width: `${Math.max(pct, 2)}%` }}
              />
            </div>
            <span className="progress-speed">{p.done ? "" : fmt(p.speed_bps)}</span>
          </div>
        );
      })}
    </div>
  );
}
