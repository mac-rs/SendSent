import type { Peer } from "../lib/types";
import { CheckIcon, WifiIcon } from "./Icons";

function initials(name: string): string {
  const t = name.trim();
  if (!t) return "?";
  // 取第一个非空白字符
  return t[0].toUpperCase();
}

function platformLabel(p: Peer["platform"]): string {
  switch (p) {
    case "macos": return "macOS";
    case "ios": return "iOS";
    case "android": return "Android";
    case "windows": return "Windows";
    case "linux": return "Linux";
    default: return "Unknown";
  }
}

export function PeerList({
  peers,
  selected,
  onToggle,
}: {
  peers: Peer[];
  selected: Peer[];
  onToggle: (p: Peer) => void;
}) {
  const ids = new Set(selected.map((p) => p.device_id));
  const isManual = (p: Peer) => p.device_id.startsWith("manual-");

  if (peers.length === 0) {
    return (
      <div className="peer-empty">
        <div className="pulse"><WifiIcon size={26} /></div>
        <div className="title">正在发现附近设备…</div>
        <div className="hint">确保设备在同一局域网,或在上方手动添加 IP</div>
      </div>
    );
  }

  return (
    <div className="peer-list">
      {peers.map((p, i) => (
        <div
          key={p.device_id}
          className={`peer-item${ids.has(p.device_id) ? " selected" : ""} fade-in`}
          style={{ animationDelay: `${Math.min(i, 12) * 30}ms` }}
          onClick={() => onToggle(p)}
          role="button"
          aria-pressed={ids.has(p.device_id)}
          tabIndex={0}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              onToggle(p);
            }
          }}
        >
          <div className={`peer-avatar ${p.platform}`}>
            {initials(p.name)}
            <span className={`peer-presence${isManual(p) ? " manual" : ""}`} />
          </div>
          <div className="peer-info">
            <div className="peer-name">{p.name}</div>
            <div className="peer-meta">
              <span>{platformLabel(p.platform)}</span>
              <span aria-hidden>·</span>
              <span className="mono">{p.addrs[0] || `:${p.port}`}</span>
            </div>
          </div>
          <div className="peer-check" aria-hidden>
            <CheckIcon size={14} />
          </div>
        </div>
      ))}
    </div>
  );
}
