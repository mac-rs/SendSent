import type { Peer } from "../lib/types";

export function PeerList({ peers, onPick }: { peers: Peer[]; onPick: (p: Peer) => void }) {
  if (peers.length === 0) return <p>正在发现附近设备…</p>;
  return (
    <ul style={{ listStyle: "none", padding: 0 }}>
      {peers.map((p) => (
        <li key={p.device_id}>
          <button onClick={() => onPick(p)}>
            {p.name} <small>({p.platform})</small>
          </button>
        </li>
      ))}
    </ul>
  );
}
