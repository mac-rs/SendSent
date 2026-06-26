import type { Peer } from "../lib/types";

export function PeerList({ peers, selected, onToggle }: { peers: Peer[]; selected: Peer[]; onToggle: (p: Peer) => void }) {
  if (peers.length === 0) return <p>正在发现附近设备…</p>;
  const ids = new Set(selected.map((p) => p.device_id));
  return (
    <ul style={{ listStyle: "none", padding: 0 }}>
      {peers.map((p) => (
        <li key={p.device_id}>
          <button
            onClick={() => onToggle(p)}
            style={{ fontWeight: ids.has(p.device_id) ? "bold" : "normal" }}
          >
            {ids.has(p.device_id) ? "☑" : "☐"} {p.name} <small>({p.platform})</small>
          </button>
        </li>
      ))}
    </ul>
  );
}
