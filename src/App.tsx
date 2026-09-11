import { useEffect, useMemo, useState } from "react";
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
  SunIcon, MoonIcon, AutoIcon, RefreshIcon,
} from "./components/Icons";
import { AddDeviceSheet } from "./components/AddDeviceSheet";
import { PrefsPanel } from "./components/PrefsPanel";
import { PullToRefresh } from "./components/PullToRefresh";
import { SwipeIndicator } from "./components/SwipeIndicator";
import { CommandPalette, type CommandItem } from "./components/CommandPalette";
import { usePlatform } from "./lib/platform";
import { useSwipeTabs } from "./lib/useSwipeTabs";
import { addPeer, sendText, respond } from "./lib/invoke";
import { registerSendFilenames } from "./hooks/useTransfer";
import type { Peer } from "./lib/types";

type Tab = "devices" | "transfers" | "settings";
type ThemePref = "auto" | "light" | "dark";

const PAGE_TITLE: Record<Tab, string> = {
  devices: "设备",
  transfers: "传输",
  settings: "设置",
};

function pageTitle(t: Tab): string {
  return PAGE_TITLE[t];
}

function App() {
  const { request, progress, history, clearHistory, clearRequest } = useTransfer();
  const { layout, platform } = usePlatform();
  const { peers, refreshing, refresh: refreshPeers } = usePeers();
  const [selected, setSelected] = useState<Peer[]>([]);
  const [tab, setTab] = useState<Tab>("devices");
  const [theme, setTheme] = useState<ThemePref>("auto");
  const [manualAddr, setManualAddr] = useState("");
  const [addMsg, setAddMsg] = useState<{ type: "ok" | "err"; text: string } | null>(null);
  const [textContent, setTextContent] = useState("");
  const [textBusy, setTextBusy] = useState(false);
  const [addSheetOpen, setAddSheetOpen] = useState(false);
  const [prefsOpen, setPrefsOpen] = useState(false);
  const [cmdOpen, setCmdOpen] = useState(false);
  const [atTop, setAtTop] = useState(true);
  const [toast, setToast] = useState<{ text: string; tone?: "ok" | "err" } | null>(null);

  useEffect(() => {
    const root = document.documentElement;
    root.classList.remove("light", "dark");
    if (theme === "light") root.classList.add("light");
    else if (theme === "dark") root.classList.add("dark");
  }, [theme]);

  const isMobile = layout === "mobile";
  const isPhone = platform === "ios" || platform === "android";
  const isIos = platform === "ios";
  const isAndroid = platform === "android";
  const showThemeToggle = !isPhone; // 手机端跟随系统,不展示主题切换
  const charCountWarn = textContent.length > 3500;
  const charCountMax = 4096;
  const peerNames = selected.map((p) => p.name).join("、");
  const selectionCount = selected.length;
  const showLargeTitle = isIos && isMobile && atTop;

  const TAB_ORDER: Tab[] = ["devices", "transfers", "settings"];

  // 触发一个简短的全局 toast 反馈(2.4s 自动消失)
  const showToast = (text: string, tone: "ok" | "err" = "ok") => {
    setToast({ text, tone });
    window.setTimeout(() => setToast((cur) => (cur?.text === text ? null : cur)), 2400);
  };
  const onRefreshWithToast = async () => {
    const n = await refreshPeers();
    showToast(n === 0 ? "未发现其他设备" : `发现 ${n} 台设备`);
  };

  // 桌面端快捷键套件
  useEffect(() => {
    if (isMobile) return;
    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (!mod) return;
      const key = e.key.toLowerCase();
      // ⌘⇧P 命令面板
      if (key === "p" && e.shiftKey) {
        e.preventDefault();
        setCmdOpen((p) => !p);
        return;
      }
      // ⌘, 偏好设置
      if (key === ",") {
        e.preventDefault();
        setPrefsOpen((p) => !p);
        return;
      }
      // ⌘W 关闭任意模态/面板
      if (key === "w") {
        e.preventDefault();
        if (prefsOpen) setPrefsOpen(false);
        else if (addSheetOpen) setAddSheetOpen(false);
        else if (request) {
          respond(request.session_id, false);
          clearRequest();
        }
        return;
      }
      // ⌘1/⌘2/⌘3 切 tab
      if (key === "1") { e.preventDefault(); setTab("devices"); return; }
      if (key === "2") { e.preventDefault(); setTab("transfers"); return; }
      if (key === "3") { e.preventDefault(); setTab("settings"); return; }
      // ⌘[ / ⌘] 切 tab(浏览器风格)
      if (key === "[") { e.preventDefault();
        setTab((t) => TAB_ORDER[Math.max(TAB_ORDER.indexOf(t) - 1, 0)]); return; }
      if (key === "]") { e.preventDefault();
        setTab((t) => TAB_ORDER[Math.min(TAB_ORDER.indexOf(t) + 1, TAB_ORDER.length - 1)]); return; }
      // ⌘R 刷新设备列表
      if (key === "r") {
        e.preventDefault();
        onRefreshWithToast();
        return;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [isMobile, prefsOpen, addSheetOpen, request, onRefreshWithToast]);

  // 命令列表
  const commands: CommandItem[] = useMemo(() => [
    {
      id: "tab.devices",
      group: "导航",
      label: "切换到「设备」",
      hint: "查看局域网发现的设备",
      keywords: ["device", "设备", "tab"],
      shortcut: "⌘1",
      action: () => setTab("devices"),
      icon: <DevicesIcon size={14} />,
    },
    {
      id: "tab.transfers",
      group: "导航",
      label: "切换到「传输」",
      hint: "查看进行中 / 已完成的传输",
      keywords: ["transfer", "传输", "tab"],
      shortcut: "⌘2",
      action: () => setTab("transfers"),
      icon: <TransferIcon size={14} />,
    },
    {
      id: "tab.settings",
      group: "导航",
      label: "切换到「设置」",
      hint: "配置应用偏好",
      keywords: ["settings", "prefs", "设置", "tab"],
      shortcut: "⌘3",
      action: () => setTab("settings"),
      icon: <SettingsIcon size={14} />,
    },
    {
      id: "refresh",
      group: "设备",
      label: "刷新设备列表",
      hint: "重新 mDNS 扫描",
      keywords: ["refresh", "scan", "刷新", "扫描"],
      shortcut: "⌘R",
      action: () => { onRefreshWithToast(); },
      icon: <RefreshIcon size={14} />,
    },
    {
      id: "add-device",
      group: "设备",
      label: "添加设备",
      hint: "手动输入 IP:port 或扫码",
      keywords: ["add", "device", "ip", "添加", "扫码"],
      action: () => setAddSheetOpen(true),
      icon: <PlusIcon size={14} />,
    },
    {
      id: "prefs",
      group: "应用",
      label: "偏好设置",
      hint: "打开偏好面板",
      keywords: ["prefs", "settings", "设置", "偏好"],
      shortcut: "⌘,",
      action: () => setPrefsOpen(true),
      icon: <SettingsIcon size={14} />,
    },
    {
      id: "theme.light",
      group: "主题",
      label: "切换为浅色",
      keywords: ["theme", "light", "主题", "浅色"],
      action: () => setTheme("light"),
      icon: <SunIcon size={14} />,
    },
    {
      id: "theme.dark",
      group: "主题",
      label: "切换为深色",
      keywords: ["theme", "dark", "主题", "深色"],
      action: () => setTheme("dark"),
      icon: <MoonIcon size={14} />,
    },
    {
      id: "theme.auto",
      group: "主题",
      label: "跟随系统",
      keywords: ["theme", "auto", "system", "主题", "自动", "系统"],
      action: () => setTheme("auto"),
      icon: <AutoIcon size={14} />,
    },
  ], [onRefreshWithToast]);

  // iOS mobile:tab 之间左右滑动切换
  const { progress: swipeProgress } = useSwipeTabs({
    enabled: isIos && isMobile,
    onLeft: () =>
      setTab((t) => TAB_ORDER[Math.min(TAB_ORDER.indexOf(t) + 1, TAB_ORDER.length - 1)]),
    onRight: () =>
      setTab((t) => TAB_ORDER[Math.max(TAB_ORDER.indexOf(t) - 1, 0)]),
  });

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
    } catch (e) { setAddMsg({ type: "err", text: "发送失败: " + String(e) }); }
    finally { setTextBusy(false); }
  };

  const cycleTheme = () => {
    setTheme((t) => (t === "auto" ? "light" : t === "light" ? "dark" : "auto"));
  };
  const ThemeIcon = theme === "light" ? SunIcon : theme === "dark" ? MoonIcon : AutoIcon;
  const themeTitle =
    theme === "auto" ? "跟随系统 (点击切换为浅色)"
    : theme === "light" ? "浅色 (点击切换为深色)"
    : "深色 (点击切换为跟随系统)";

  return (
    <div className={`app ${platform}${isMobile ? " mobile" : ""}`}>
      {/* ── Top bar ──────────────────────────────────── */}
      <header className="topbar" data-tauri-drag-region>
        {/* iOS / Android: 极简大标题风格,无品牌 logo(节省空间) */}
        {isPhone ? (
          isMobile ? (
            <div className="brand brand-mobile" data-tauri-drag-region>
              <span className="brand-text">{pageTitle(tab)}</span>
            </div>
          ) : (
            <div className="brand" data-tauri-drag-region>
              <div className="brand-mark"><LogoIcon size={18} /></div>
              <span className="brand-text">SendSent</span>
            </div>
          )
        ) : (
          <div className="brand" data-tauri-drag-region>
            <div className="brand-mark"><LogoIcon size={18} /></div>
            <span className="brand-text">SendSent</span>
          </div>
        )}

        <div className="topbar-spacer" data-tauri-drag-region />

        <div className="topbar-actions">
          {/* 桌面端 · 偏好设置(等同 Cmd+,) */}
          {!isPhone && (
            <button
              className="icon-btn"
              onClick={() => setPrefsOpen(true)}
              title="偏好设置 (⌘,)"
              aria-label="偏好设置"
            >
              <SettingsIcon size={16} />
            </button>
          )}
          {/* Android mobile · 设备 tab:右侧 + 按钮(快速添加) */}
          {isAndroid && isMobile && tab === "devices" && (
            <button
              className="icon-btn"
              onClick={() => setAddSheetOpen(true)}
              title="添加设备"
              aria-label="添加设备"
            >
              <PlusIcon size={18} />
            </button>
          )}
          {/* 桌面端:主题切换按钮(手机跟随系统) */}
          {showThemeToggle && (
            <button
              className="icon-btn theme-cycle"
              onClick={cycleTheme}
              title={themeTitle}
              aria-label={themeTitle}
            >
              <ThemeIcon size={16} />
            </button>
          )}
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
                  <button
                    key={item.id}
                    type="button"
                    className={`nav-item${tab === item.id ? " active" : ""}`}
                    onClick={() => setTab(item.id)}
                    role="tab"
                    aria-selected={tab === item.id}
                  >
                    <Icon />
                    <span>{item.label}</span>
                    {item.badge !== undefined && <span className="badge">{item.badge}</span>}
                  </button>
                );
              })}
            </div>
            <div className="sidebar-footer">
              <div className="row">
                <WifiIcon size={12} />
                <span>端口 52225 · mDNS 发现中</span>
              </div>
              <div className="id-name">
                {selectionCount === 0
                  ? "未选择设备"
                  : `已选 ${selectionCount} 台设备`}
              </div>
              {!isPhone && (
                <button
                  className={`refresh-btn${refreshing ? " refreshing" : ""}`}
                  onClick={onRefreshWithToast}
                  title="刷新设备列表 (⌘R)"
                  aria-label="刷新设备列表"
                >
                  <RefreshIcon size={12} />
                  <span>刷新设备</span>
                </button>
              )}
              <div className="shortcut-hints">
                <kbd className="kbd">⌘ ,</kbd>
                <span>偏好</span>
                <kbd className="kbd">⌘R</kbd>
                <span>刷新</span>
                <kbd className="kbd">⌘ 1/2/3</kbd>
                <span>切换</span>
              </div>
            </div>
          </aside>
        )}

        {/* iOS mobile · pull-to-refresh(只在设备 tab 生效) */}
        {isIos && isMobile && tab === "devices" && (
          <PullToRefresh
            onRefresh={async () => {
              try { await onRefreshWithToast(); } catch { /* ignore */ }
            }}
          />
        )}

        {/* iOS mobile · swipe-back 拖影指示器 */}
        {isIos && isMobile && Math.abs(swipeProgress) > 0.01 && (
          <SwipeIndicator progress={swipeProgress} />
        )}

        <main
          className="main"
          onScroll={(e) => {
            // Only re-render when the large-title threshold is crossed, so
            // scrolling does not trigger a React render on every frame.
            const top = (e.target as HTMLElement).scrollTop < 60;
            if (top !== atTop) setAtTop(top);
          }}
        >
          {isIos && isMobile && (
            <div className={`large-title${showLargeTitle ? "" : " collapsed"}`}>{pageTitle(tab)}</div>
          )}
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
                    {selectionCount > 0 && <span className="count">{selectionCount}</span>}
                  </div>
                  <FilePicker peers={selected} />
                </section>

                <section className="section">
                  <div className="section-title"><span>文字消息</span></div>
                  <div className="text-send">
                    <textarea
                      rows={2}
                      placeholder={selectionCount === 0
                        ? "先选择设备,再输入文字发送…"
                        : `发送一段文字到 ${peerNames}`}
                      value={textContent}
                      onChange={(e) => setTextContent(e.target.value.slice(0, charCountMax))}
                      maxLength={charCountMax}
                      disabled={selectionCount === 0 || textBusy}
                    />
                    <div className="text-send-footer">
                      <span className={`text-send-hint${charCountWarn ? " warn" : ""}`}>
                        {textContent.length} / {charCountMax}
                      </span>
                      <button
                        className="btn btn-primary"
                        disabled={selectionCount === 0 || !textContent.trim() || textBusy}
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
        <nav className="tabbar" role="tablist">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                type="button"
                className={`tabbar-item${tab === item.id ? " active" : ""}`}
                onClick={() => setTab(item.id)}
                role="tab"
                aria-selected={tab === item.id}
              >
                <Icon size={24} />
                <span>{item.label}</span>
                {item.badge !== undefined && <span className="dot" aria-hidden />}
              </button>
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

      {/* ── Add device sheet (Android + 触发) ────────── */}
      <AddDeviceSheet
        open={addSheetOpen}
        onClose={() => setAddSheetOpen(false)}
      />

      {/* ── Prefs panel (桌面端 ⌘, 触发) ────────────── */}
      <PrefsPanel
        open={prefsOpen}
        onClose={() => setPrefsOpen(false)}
      />

      {/* ── Command palette (桌面端 ⌘⇧P 触发) ─────── */}
      <CommandPalette
        open={cmdOpen}
        onClose={() => setCmdOpen(false)}
        commands={commands}
      />

      {/* ── Global toast (e.g. ⌘R 反馈) ──────────── */}
      {toast && (
        <div className={`app-toast${toast.tone === "err" ? " err" : ""}`} role="status" aria-live="polite">
          {toast.text}
        </div>
      )}
    </div>
  );
}

export default App;
