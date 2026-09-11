// 桌面端偏好设置面板 · Cmd+, 快捷键唤起
// macOS HIG 风格:固定宽度、左侧分组列表(含 search)、右侧内容

import { useEffect, useMemo, useRef, useState } from "react";
import { XIcon, SettingsIcon, SendIcon, InfoIcon, SearchIcon, QrCodeIcon } from "./Icons";
import { SettingsForm } from "./SettingsForm";
import { TransferConfigEditor } from "./TransferConfigEditor";
import { MyQrDialog } from "./MyQrDialog";
import { getIdentity, getDefaultSaveDir, getMyAddresses } from "../lib/invoke";

type PrefsTab = "general" | "transfer" | "about";

const TABS: Array<{ id: PrefsTab; label: string; icon: typeof SettingsIcon; keywords: string[] }> = [
  { id: "general", label: "通用", icon: SettingsIcon, keywords: ["name", "display", "save", "保存", "名", "通用"] },
  { id: "transfer", label: "传输", icon: SendIcon, keywords: ["conn", "chunk", "split", "speed", "传输", "并发", "分片", "块"] },
  { id: "about", label: "关于", icon: InfoIcon, keywords: ["about", "version", "tech", "qr", "关于", "版本"] },
];

const DEFAULT_PORT = 52225;

export function PrefsPanel({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const [tab, setTab] = useState<PrefsTab>("general");
  const [q, setQ] = useState("");
  const [qrOpen, setQrOpen] = useState(false);
  const [identity, setIdentity] = useState<{ name: string } | null>(null);
  const [saveDir, setSaveDir] = useState("/Downloads/sendsent");
  const [addresses, setAddresses] = useState<Array<{ interface: string; ip: string }>>([]);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      if ((e.metaKey || e.ctrlKey) && e.key === "f") {
        e.preventDefault();
        searchRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    setTimeout(() => searchRef.current?.focus(), 50);
    // 拉取 identity + save dir 给"关于"页用
    getIdentity().then(setIdentity).catch(() => {});
    getDefaultSaveDir().then(setSaveDir).catch(() => {});
    getMyAddresses().then(setAddresses).catch(() => setAddresses([]));
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  const filteredTabs = useMemo(() => {
    const k = q.trim().toLowerCase();
    if (!k) return TABS;
    return TABS.filter((t) =>
      t.label.toLowerCase().includes(k) ||
      t.keywords.some((kw) => kw.toLowerCase().includes(k))
    );
  }, [q]);

  useEffect(() => {
    if (q && filteredTabs.length > 0 && !filteredTabs.find((t) => t.id === tab)) {
      setTab(filteredTabs[0].id);
    }
  }, [q, filteredTabs, tab]);

  if (!open) return null;

  return (
    <>
      <div
        className="prefs-backdrop"
        onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
        role="presentation"
      >
        <div className="prefs-panel" role="dialog" aria-modal aria-label="偏好设置">
          <aside className="prefs-sidebar">
            <div className="prefs-sidebar-header">偏好设置</div>
            <div className="prefs-search">
              <SearchIcon size={14} />
              <input
                ref={searchRef}
                type="text"
                placeholder="搜索设置…"
                value={q}
                onChange={(e) => setQ(e.target.value)}
                spellCheck={false}
                autoComplete="off"
              />
              {q && (
                <button
                  className="prefs-search-clear"
                  onClick={() => { setQ(""); searchRef.current?.focus(); }}
                  aria-label="清空"
                >
                  <XIcon size={12} />
                </button>
              )}
              <kbd className="prefs-search-kbd">⌘F</kbd>
            </div>
            <nav className="prefs-nav">
              {filteredTabs.length === 0 ? (
                <div className="prefs-nav-empty">没有匹配的设置</div>
              ) : (
                filteredTabs.map((t) => {
                  const Icon = t.icon;
                  return (
                    <button
                      key={t.id}
                      className={`prefs-nav-item${tab === t.id ? " active" : ""}`}
                      onClick={() => setTab(t.id)}
                    >
                      <Icon size={16} />
                      <span>{t.label}</span>
                    </button>
                  );
                })
              )}
            </nav>
            <div className="prefs-sidebar-footer">
              <button className="prefs-close" onClick={onClose} aria-label="关闭">
                关闭
              </button>
            </div>
          </aside>
          <main className="prefs-content">
            <header className="prefs-content-header">
              <h2>{TABS.find((t) => t.id === tab)?.label ?? "设置"}</h2>
              <button
                className="icon-btn prefs-content-close"
                onClick={onClose}
                aria-label="关闭"
              >
                <XIcon size={16} />
              </button>
            </header>
            <div className="prefs-content-body">
              {tab === "general" && <SettingsForm />}
              {tab === "transfer" && (
                <div className="card">
                  <div className="card-body">
                    <TransferConfigEditor />
                  </div>
                </div>
              )}
              {tab === "about" && (
                <div className="card">
                  <div className="card-body">
                    <div className="about-stack">
                      <div className="about-mark"><SettingsIcon size={28} /></div>
                      <div className="about-name">SendSent</div>
                      <div className="about-version">v0.1.0 · LAN P2P</div>
                      <p className="about-desc">
                        基于 Tauri 2 + React 19 的局域网 P2P 文件传输工具。
                        <br />
                        支持 macOS / iOS / Android / Windows / Linux。
                      </p>
                      <div className="about-tech">
                        <span className="about-chip">Rust</span>
                        <span className="about-chip">React 19</span>
                        <span className="about-chip">mDNS</span>
                        <span className="about-chip">TCP</span>
                      </div>
                      <button
                        className="btn btn-primary about-qr-btn"
                        onClick={() => setQrOpen(true)}
                      >
                        <QrCodeIcon size={16} />
                        <span>显示我的设备 QR</span>
                      </button>

                      <div className="about-addrs">
                        <div className="about-addrs-title">本机 IP 地址(自动枚举)</div>
                        {addresses.length === 0 ? (
                          <div className="about-addrs-empty">未发现可用的局域网接口</div>
                        ) : (
                          <ul className="about-addrs-list">
                            {addresses.map((a) => (
                              <li key={`${a.interface}-${a.ip}`}>
                                <span className="about-addrs-iface">{a.interface}</span>
                                <span className="about-addrs-ip">{a.ip}:{DEFAULT_PORT}</span>
                              </li>
                            ))}
                          </ul>
                        )}
                        <div className="about-addrs-foot">
                          扫码时,对方设备会使用列表中第一个可达地址;若你的电脑同时连接多个网络,可在 QR 对话框中切换
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </main>
        </div>
      </div>

      <MyQrDialog
        open={qrOpen}
        onClose={() => setQrOpen(false)}
        identity={identity}
        saveDir={saveDir}
        port={DEFAULT_PORT}
      />
    </>
  );
}
