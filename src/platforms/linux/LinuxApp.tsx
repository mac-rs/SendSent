// Linux 平台布局
// 类似 macOS 但:
//   · 不要 traffic lights(很多 Linux 桌面都自己画)
//   · titlebar 左侧是 app icon + 名称(GTK HeaderBar 风格)
//   · accent 用 GNOME blue #3584e4
//   · 字体用 Cantarell / Inter / system-ui

import { useMemo } from "react";
import type { PlatformAppProps } from "../types";
import { PeerList } from "../../components/PeerList";
import { FilePicker } from "../../components/FilePicker";
import { TransferProgress, TransferHistory } from "../../components/TransferProgress";
import { IncomingRequest } from "../../components/IncomingRequest";
import { AddDeviceSheet } from "../../components/AddDeviceSheet";
import { PrefsPanel } from "../../components/PrefsPanel";
import { CommandPalette } from "../../components/CommandPalette";
import {
  DevicesIcon, TransferIcon, SettingsIcon, SearchIcon,
  RefreshIcon, LogoIcon,
  SunIcon, MoonIcon, AutoIcon,
} from "../../components/Icons";

function cycleTheme(t: PlatformAppProps["theme"]) {
  return t === "auto" ? "light" : t === "light" ? "dark" : "auto";
}

export function LinuxApp(props: PlatformAppProps) {
  const {
    tab, setTab, theme, setTheme, peers, selected, togglePeer, refreshing, refresh,
    progress, history, request, prefsOpen, setPrefsOpen, addSheetOpen, setAddSheetOpen,
    commands, cmdOpen, setCmdOpen, clearHistory, clearRequest, respond, toast,
  } = props;

  const transfers = Object.values(progress);
  const activeCount = transfers.filter((t) => !t.done).length;

  const navItems = useMemo(() => [
    { id: "devices" as const, label: "设备", icon: DevicesIcon, badge: peers.length },
    { id: "transfers" as const, label: "传输", icon: TransferIcon, badge: activeCount || undefined },
    { id: "settings" as const, label: "设置", icon: SettingsIcon },
  ], [peers.length, activeCount]);

  const ThemeIconEl = theme === "light" ? SunIcon : theme === "dark" ? MoonIcon : AutoIcon;

  return (
    <div className="app linux desktop" data-platform="linux">
      <header className="titlebar" data-tauri-drag-region>
        <div className="titlebar-left" data-tauri-drag-region>
          <button className="hamburger" data-tauri-drag-region aria-label="菜单">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M3 6h18M3 12h18M3 18h18"/>
            </svg>
          </button>
          <div className="titlebar-icon" data-tauri-drag-region>
            <LogoIcon size={14} />
          </div>
          <span className="titlebar-name" data-tauri-drag-region>SendSent</span>
        </div>

        <div className="titlebar-center" data-tauri-drag-region>
          <div className="gnome-tabs">
            {navItems.map((it) => (
              <button
                key={it.id}
                className={`gnome-tab${tab === it.id ? " active" : ""}`}
                onClick={() => setTab(it.id)}
              >
                {it.label}
              </button>
            ))}
          </div>
        </div>

        <div className="titlebar-right" data-tauri-drag-region>
          <button className="caption-btn" onClick={() => setCmdOpen(true)} title="搜索 (Ctrl+F)">
            <SearchIcon size={14} />
          </button>
          <button className="caption-btn" onClick={() => setPrefsOpen(true)} title="设置 (Ctrl+,)">
            <SettingsIcon size={14} />
          </button>
          <button className="caption-btn" onClick={() => setTheme(cycleTheme(theme))} title="主题">
            <ThemeIconEl size={14} />
          </button>
        </div>
      </header>

      <div className="body">
        <main className="main">
          {tab === "devices" && (
            <>
              <div className="hero">
                <h1 className="hero-title">设备</h1>
                <p className="hero-sub">
                  {peers.length === 0 ? "正在发现附近的局域网设备…" : `已发现 ${peers.length} 台设备 · ${selected.length} 台已选择`}
                </p>
                <div className="searchbar">
                  <SearchIcon size={13} />
                  <input placeholder="搜索设备、文件…" />
                  <span className="kbd">Ctrl F</span>
                </div>
              </div>
              <div className="stats">
                <div className="stat">
                  <div className="stat-label">本机 IP</div>
                  <div className="stat-value">{peers[0]?.addrs[0] ?? "—"}</div>
                </div>
                <div className="stat">
                  <div className="stat-label">在线设备</div>
                  <div className="stat-value">{peers.length}</div>
                </div>
                <div className="stat">
                  <div className="stat-label">活跃传输</div>
                  <div className="stat-value">{activeCount}</div>
                </div>
              </div>
              <div className="toolbar-row">
                <button
                  className={`refresh-btn${refreshing ? " refreshing" : ""}`}
                  onClick={refresh}
                >
                  <RefreshIcon size={12} />
                  <span>刷新设备</span>
                </button>
              </div>
              <PeerList peers={peers} selected={selected} onToggle={togglePeer} />
              <FilePicker peers={selected} />
            </>
          )}

          {tab === "transfers" && (
            <>
              <div className="hero">
                <h1 className="hero-title">传输</h1>
                <p className="hero-sub">{activeCount > 0 ? `${activeCount} 个进行中` : "当前无活跃传输"}</p>
              </div>
              <TransferProgress items={transfers} />
              {history.length > 0 && (
                <>
                  <div className="section-title">
                    <h2>历史</h2>
                    <button className="btn btn-ghost" onClick={clearHistory}>清空记录</button>
                  </div>
                  <TransferHistory items={history} onClear={clearHistory} />
                </>
              )}
            </>
          )}

          {tab === "settings" && (
            <>
              <div className="hero">
                <h1 className="hero-title">设置</h1>
                <p className="hero-sub">管理应用偏好与传输配置</p>
              </div>
              <button className="btn btn-primary" onClick={() => setPrefsOpen(true)}>
                打开偏好面板
              </button>
            </>
          )}
        </main>
      </div>

      <AddDeviceSheet open={addSheetOpen} onClose={() => setAddSheetOpen(false)} />
      <PrefsPanel open={prefsOpen} onClose={() => setPrefsOpen(false)} />
      <CommandPalette
        open={cmdOpen}
        onClose={() => setCmdOpen(false)}
        commands={commands}
      />
      {request && (
        <IncomingRequest
          req={request}
          onRespond={(accept) => {
            respond(request.session_id, accept);
            clearRequest();
          }}
        />
      )}

      {toast && (
        <div className={`app-toast${toast.tone === "err" ? " err" : ""}`}>{toast.text}</div>
      )}
    </div>
  );
}
