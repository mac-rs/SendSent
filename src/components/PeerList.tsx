import type { Peer } from "../lib/types";

export function PeerList({ peers, selected, onToggle }: { peers: Peer[]; selected: Peer[]; onToggle: (p: Peer) => void }) {
  const ids = new Set(selected.map((p) => p.device_id));
  return (
    <section>
      <div className="peer-count">
        附近设备 · {peers.length} 台
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        {peers.map((p, i) => (
          <div
            key={p.device_id}
            className={`peer-item fade-in${selected.length > 0 ? "" : ` fade-in-${(i % 3) + 1}`}`}
            style={i > 0 ? { animationDelay: `${0.1 + i * 0.05}s` } : undefined}
            onClick={() => onToggle(p)}
          >
            <span className={`peer-dot ${p.device_id.startsWith("manual-") ? "manual" : "online"}`} />
            <div className="peer-info">
              <div className="peer-name">{p.name}</div>
              <div className="peer-meta">{p.platform} · {p.addrs[0] || `:${p.port}`}</div>
            </div>
            <input type="checkbox" className="peer-check" checked={ids.has(p.device_id)} readOnly />
          </div>
        ))}
        {peers.length === 0 && (
          <div style={{ fontSize: 12, color: "var(--text-secondary)", textAlign: "center", padding: 20 }}>
            正在发现附近设备…
          </div>
        )}
      </div>
    </section>
  );
}
