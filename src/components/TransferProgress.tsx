import type { ProgressView } from "../hooks/useTransfer";

function fmt(bps: number) {
  if (bps >= 1_000_000) return `${(bps / 1_000_000).toFixed(1)} MB/s`;
  if (bps >= 1_000) return `${(bps / 1_000).toFixed(0)} KB/s`;
  return `${bps} B/s`;
}

export function TransferProgress({ items }: { items: ProgressView[] }) {
  if (items.length === 0) return null;
  return (
    <ul style={{ listStyle: "none", padding: 0 }}>
      {items.map((p) => {
        const pct = p.bytes_total ? (p.bytes_done / p.bytes_total) * 100 : 0;
        return (
          <li key={p.session_id}>
            {p.done ? (p.error ? `失败:${p.error}` : "完成") : `${pct.toFixed(0)}% · ${fmt(p.speed_bps)}`}
          </li>
        );
      })}
    </ul>
  );
}
