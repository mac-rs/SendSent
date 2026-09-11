import { useState, useEffect } from "react";
import { usePeers } from "./hooks/usePeers";
import { useTransfer } from "./hooks/useTransfer";
import { PeerList } from "./components/PeerList";
import { FilePicker } from "./components/FilePicker";
import { IncomingRequest } from "./components/IncomingRequest";
import { TransferProgress, TransferHistory } from "./components/TransferProgress";
import { TransferConfigEditor } from "./components/TransferConfigEditor";
import { Settings } from "./components/Settings";
import {
  DevicesIcon, TransferIcon, SettingsIcon,
  PlusIcon, LogoIcon, SendIcon, WifiIcon,
} from "./components/Icons";
import { usePlatform } from "./lib/platform";
import { addPeer, sendText, respond } from "./lib/invoke";
import { registerSendFilenames } from "./hooks/useTransfer";
import type { Peer } from "./lib/types";

type Tab = "devices" | "transfers" | "settings";
type ThemePref = "auto" | "light" | "dark";

function App() {
  const peers = usePeers();
  const { request, progress, history, clearHistory, clearRequest } = useTransfer();
  const { layout, platform } = usePlatform();
  const [selected, setSelected] = useState<Peer[]>([]);
  const [tab, setTab] = useState<Tab>("devices");
  const [theme, setTheme] = useState<ThemePref>("auto");
  const [manualAddr, setManualAddr] = useState("");
  const [addMsg, setAddMsg] = useState<{ type: "ok" | "err"; text: string } | null>(null);
  const [textContent, setTextContent] = useState("");
  const [textBusy, setTextBusy] = useState(false);

  useEffect(() => {
    const root = document.documentElement;
    root.classList.remove("light", "dark");
    if (theme === "light") root.classList.add("light");
    else if (theme === "dark") root.classList.add("dark");
  }, [theme]);

  const togglePeer = (p: Peer) =>
    setSelected((c) =>
      c.some((x) => x.device_id === p.device_id)
        ? c.filter((x) => x.device_id !== p.device_id)
        : [...c, p]
    );

  const transfers = Object.values(progress);
  const activeCount = transfers.filter((t) => !t.done).length;

  const navItems: Array<{
    id: Tab; label: string; icon: typeof DevicesIcon; badge?: number;
  }> = [
    { id: "devices", label: "设备", icon: DevicesIcon, badge: selected.length || undefined },
    { id: "transfers", label: "传输", icon: TransferIcon, badge: activeCount || undefined },
    { id: "settings", label: "设置", icon: SettingsIcon },
  ];

  const sendTextNow = async () => {
    if (selected.length === 0 || !textContent.trim()) return;
    setTextBusy(true);
    try {
      for (const p of selected) {
        const sid = await sendText(p.device_id, textContent);
        registerSendFilenames(sid, ["message.txt"]);
      }
      setTextContent("");
    } catch (e) { alert("发送失败: " + String(e)); }
    finally { setTextBusy(false); }
  };

  const isMobile = layout === "mobile";

  return (
    <div className={`app ${platform}${isMobile ? " mobile" : ""}`}>
      {/* ── Top bar ──────────────────────────────────── */}
      <header className="topbar" data-tauri-drag-region>
        <div className="brand">
          <div className="brand-mark"><LogoIcon size={18} /></div>
          <span className="brand-text">SendSent</span>
        </div>
        <div className="topbar-spacer" />
        <div className="topbar-actions">
          <div className="segmented" role="group" aria-label="主题">
            {(["auto", "light", "dark"] as ThemePref[]).map((t) => (
              <button
                key={t}
                className={`segmented-item${theme === t ? " active" : ""}`}
                onClick={() => setTheme(t)}
                aria-pressed={theme === t}
              >
                {t === "auto" ? "系统" : t === "light" ? "浅色" : "深色"}
              </button>
            ))}
          </div>
        </div>
      </header>

      {/* ── Body ─────────────────────────────────────── */}
      <div className="body">
        {!isMobile && (
          <aside className="sidebar">
            <div className="nav-group">
              <div className="nav-group-label">导航</div>
              {navItems.map((item) => {
                const Icon = item.icon;
                return (
                  <div
                    key={item.id}
                    className={`nav-item${tab === item.id ? " active" : ""}`}
                    onClick={() => setTab(item.id)}
                  >
                    <Icon />
                    <span>{item.label}</span>
                    {item.badge !== undefined && <span className="badge">{item.badge}</span>}
                  </div>
                );
              })}
            </div>
            <div className="sidebar-footer">
              <div className="row">
                <WifiIcon size={12} />
                <span>端口 52225 · mDNS 发现中</span>
              </div>
              <div className="id-name">
                {selected.length === 0
                  ? "未选择设备"
                  : `已选 ${selected.length} 台设备`}
              </div>
            </div>
          </aside>
        )}

        <main className="main">
          <div className="main-inner">
            {tab === "devices" && (
              <>
                <div className="page-header">
                  <h1 className="page-title">设备</h1>
                  <p className="page-subtitle">选择目标设备,然后发送文件或文字</p>
                </div>

                <section className="section">
                  <div className="section-title">
                    <span>附近设备</span>
                    <span className="count">{peers.length}</span>
                  </div>
                  <div className="card">
                    <div className="card-body" style={{ padding: 8 }}>
                      <PeerList peers={peers} selected={selected} onToggle={togglePeer} />
                    </div>
                  </div>
                </section>

                <section className="section">
                  <div className="section-title"><span>添加设备</span></div>
                  <div className="add-peer">
                    <input
                      placeholder="IP:port,如 192.168.1.5:52225"
                      value={manualAddr}
                      onChange={(e) => { setManualAddr(e.target.value); setAddMsg(null); }}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") {
                          if (!manualAddr) return;
                          addPeer(manualAddr)
                            .then(() => { setManualAddr(""); setAddMsg({ type: "ok", text: "已添加" }); })
                            .catch((err) => setAddMsg({ type: "err", text: String(err) }));
                        }
                      }}
                    />
                    <button
                      disabled={!manualAddr}
                      onClick={() => {
                        if (!manualAddr) return;
                        addPeer(manualAddr)
                          .then(() => { setManualAddr(""); setAddMsg({ type: "ok", text: "已添加" }); })
                          .catch((err) => setAddMsg({ type: "err", text: String(err) }));
                      }}
                    >
                      <PlusIcon size={14} />添加
                    </button>
                  </div>
                  {addMsg && (
                    <div className={`toast-msg${addMsg.type === "err" ? " error" : ""}`}>
                      {addMsg.text}
                    </div>
                  )}
                </section>

                <section className="section">
                  <div className="section-title">
                    <span>发送</span>
                    {selected.length > 0 && <span className="count">{selected.length}</span>}
                  </div>
                  <FilePicker peers={selected} />
                </section>

                <section className="section">
                  <div className="text-send">
                    <textarea
                      rows={2}
                      placeholder={selected.length === 0
                        ? "先选择设备,再输入文字发送…"
                        : `发送一段文字到 ${selected.length} 台设备`}
                      value={textContent}
                      onChange={(e) => setTextContent(e.target.value)}
                      disabled={selected.length === 0 || textBusy}
                    />
                    <div className="text-send-footer">
                      <span className="text-send-hint">
                        {textContent.length} / 4096
                      </span>
                      <button
                        className="btn btn-primary"
                        disabled={selected.length === 0 || !textContent.trim() || textBusy}
                        onClick={sendTextNow}
                      >
                        <SendIcon size={14} />
                        {textBusy ? "发送中…" : "发送"}
                      </button>
                    </div>
                  </div>
                </section>

                {activeCount > 0 && (
                  <section className="section">
                    <div className="section-title">
                      <span>进行中的传输</span>
                      <span className="count">{activeCount}</span>
                    </div>
                    <TransferProgress items={transfers.filter((t) => !t.done)} compact />
                    <button className="btn btn-ghost" onClick={() => setTab("transfers")}>
                      查看全部 →
                    </button>
                  </section>
                )}
              </>
            )}

            {tab === "transfers" && (
              <>
                <div className="page-header">
                  <h1 className="page-title">传输</h1>
                  <p className="page-subtitle">
                    {transfers.length === 0
                      ? "暂无传输记录"
                      : `共 ${transfers.length} 项 · 进行中 ${activeCount}`}
                  </p>
                </div>
                <section className="section">
                  <TransferProgress items={transfers} />
                </section>
                <section className="section">
                  <div className="section-title"><span>历史记录</span></div>
                  <TransferHistory
                    items={history}
                    onClear={() => { if (confirm("清空全部传输记录？")) clearHistory(); }}
                  />
                </section>
              </>
            )}

            {tab === "settings" && (
              <>
                <div className="page-header">
                  <h1 className="page-title">设置</h1>
                  <p className="page-subtitle">个性化与传输参数</p>
                </div>
                <section className="section">
                  <div className="section-title"><span>身份</span></div>
                  <div className="card">
                    <div className="card-body">
                      <Settings />
                    </div>
                  </div>
                </section>
                <section className="section">
                  <div className="section-title"><span>传输参数</span></div>
                  <div className="card">
                    <div className="card-body">
                      <TransferConfigEditor />
                    </div>
                  </div>
                </section>
              </>
            )}
          </div>
        </main>
      </div>

      {/* ── Mobile tab bar ───────────────────────────── */}
      {isMobile && (
        <nav className="tabbar">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <div
                key={item.id}
                className={`tabbar-item${tab === item.id ? " active" : ""}`}
                onClick={() => setTab(item.id)}
                style={{ position: "relative" }}
              >
                <Icon size={22} />
                <span>{item.label}</span>
                {item.badge !== undefined && <span className="dot" />}
              </div>
            );
          })}
        </nav>
      )}

      {/* ── Incoming request modal ───────────────────── */}
      <IncomingRequest
        req={request}
        onRespond={(accept) => {
          if (request) respond(request.session_id, accept);
          clearRequest();
        }}
      />
    </div>
  );
}

export default App;
