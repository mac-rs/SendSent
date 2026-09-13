import { useEffect, useState } from "react";
import { getIdentity, getMyAddresses, getMyQr } from "../lib/invoke";

type Addr = { interface: string; ip: string };

function platformLabel(p: string): string {
  switch (p) {
    case "macos": return "macOS";
    case "ios": return "iOS";
    case "android": return "Android";
    case "windows": return "Windows";
    case "linux": return "Linux";
    default: return "Unknown";
  }
}

export function ProfilePage() {
  const [name, setName] = useState("");
  const [platform, setPlatform] = useState("");
  const [addrs, setAddrs] = useState<Addr[]>([]);
  const [qr, setQr] = useState<string | null>(null);

  const port = 52225;

  useEffect(() => {
    let cancelled = false;
    getIdentity()
      .then((i) => { if (!cancelled) { setName(i.name); setPlatform(i.platform); } })
      .catch(() => {});
    getMyAddresses()
      .then((a) => { if (!cancelled) setAddrs(a); })
      .catch(() => {});
    getMyQr(1024)
      .then((b64) => { if (!cancelled) setQr(b64); })
      .catch(() => {});
    return () => { cancelled = true; };
  }, []);

  return (
    <div className="profile">
      <div className="profile-label">我的</div>
      <h1 className="profile-name">{name || "未命名"}</h1>
      <div className="profile-status">
        <span className="dot online" />
        <span>可被发现</span>
        <span className="dim">·</span>
        <span>{platformLabel(platform || "macos")}</span>
        <span className="dim">·</span>
        <span>端口 {port}</span>
      </div>

      <div className="profile-qr-wrap">
        <div className="profile-qr-glow" aria-hidden />
        {qr ? (
          <img
            className="profile-qr"
            src={`data:image/png;base64,${qr}`}
            alt={`${name} QR`}
            width={196}
            height={196}
          />
        ) : (
          <div className="profile-qr placeholder">生成中…</div>
        )}
      </div>
      <p className="profile-qr-caption">让对方在 SendSent 里扫码连接</p>

      <div className="profile-section">本机地址</div>
      {addrs.length === 0 && <div className="profile-row"><span className="dim">未发现可用地址</span></div>}
      {addrs.map((a) => (
        <div className="profile-row" key={`${a.interface}-${a.ip}`}>
          <span className="mono">{a.ip}:{port}</span>
          <span className="dim">{a.interface}</span>
        </div>
      ))}

      <div className="profile-section">关于</div>
      <div className="profile-row"><span>版本</span><span className="dim">0.1.0</span></div>
      <div className="profile-row"><span>技术栈</span><span className="dim">Rust + Tauri</span></div>
    </div>
  );
}
