import type { Peer } from "../lib/types";

function PlatformGlyph({ platform, size = 9 }: { platform: string; size?: number }) {
  const common = {
    width: size, height: size, viewBox: "0 0 24 24",
    fill: "none", stroke: "currentColor", strokeWidth: 2.4,
    strokeLinecap: "round" as const, strokeLinejoin: "round" as const,
  };
  switch (platform) {
    case "ios":
    case "android":
      return (
        <svg {...common}>
          <rect x="7" y="2" width="10" height="20" rx="2.6" />
          <line x1="11" y1="18.4" x2="13" y2="18.4" />
        </svg>
      );
    case "macos":
      return (
        <svg {...common}>
          <rect x="3" y="4" width="18" height="12" rx="1.8" />
          <line x1="2" y1="20" x2="22" y2="20" />
        </svg>
      );
    case "windows":
      return (
        <svg {...common}>
          <rect x="3" y="3" width="8" height="8" rx="1" />
          <rect x="13" y="3" width="8" height="8" rx="1" />
          <rect x="3" y="13" width="8" height="8" rx="1" />
          <rect x="13" y="13" width="8" height="8" rx="1" />
        </svg>
      );
    case "linux":
      return (
        <svg {...common}>
          <rect x="3" y="4" width="18" height="16" rx="2" />
          <polyline points="7 9 10 12 7 15" />
          <line x1="12" y1="15" x2="17" y2="15" />
        </svg>
      );
    default:
      return (
        <svg {...common}>
          <circle cx="12" cy="12" r="9" />
        </svg>
      );
  }
}

function angleFor(index: number, count: number): number {
  if (count <= 1) return -Math.PI / 2;
  const start = (-140 * Math.PI) / 180;
  const end = (-40 * Math.PI) / 180;
  if (count <= 4) return start + ((end - start) * index) / (count - 1);
  return -Math.PI / 2 + (2 * Math.PI * index) / count;
}

export function Radar({
  peers,
  meName,
  selectedId,
  onSelect,
  onDeselect,
}: {
  peers: Peer[];
  meName: string;
  selectedId?: string;
  onSelect: (p: Peer) => void;
  onDeselect: () => void;
}) {
  const radius = 33; // % of container
  return (
    <div className="radar" onClick={onDeselect} role="presentation">
      <div className="radar-rings" aria-hidden>
        <span className="ring r1" />
        <span className="ring r2" />
        <span className="ring r3" />
        <span className="radar-sweep" />
        <span className="radar-pulse" />
      </div>

      <div className="radar-me" style={{ left: "50%", top: "44%" }}>
        <div className="radar-me-avatar">{(meName.trim()[0] ?? "我").toUpperCase()}</div>
        <span className="radar-pill">我 · 本机</span>
      </div>

      {peers.map((p, i) => {
        const a = angleFor(i, peers.length);
        const x = 50 + Math.cos(a) * radius;
        const y = 44 + Math.sin(a) * radius;
        const sel = selectedId === p.device_id;
        return (
          <button
            key={p.device_id}
            className={`radar-node${sel ? " selected" : ""}`}
            style={{ left: `${x}%`, top: `${y}%` }}
            onClick={(e) => {
              e.stopPropagation();
              onSelect(p);
            }}
            title={`${p.name} · ${p.platform}`}
          >
            <span className={`radar-avatar ${p.platform}`}>
              {p.name.slice(0, 1).toUpperCase()}
              <span className="radar-badge">
                <PlatformGlyph platform={p.platform} />
              </span>
              <span className="radar-online" />
            </span>
            <span className="radar-name">{p.name}</span>
          </button>
        );
      })}
    </div>
  );
}
