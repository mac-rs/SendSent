// Windows 平台布局
// 基于 macOS 布局,但:
//   · titlebar 左侧加 app icon + 名称(Segoe UI Variable)
//   · 右侧 caption buttons(最小/最大/关闭,不是 traffic lights)
//   · accent 色用 #0078D4(Win 11 蓝)

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

export function WindowsApp(props: PlatformAppProps) {
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
    <div className="app windows desktop" data-platform="windows">
      <header className="titlebar" data-tauri-drag-region>
        <div className="titlebar-left" data-tauri-drag-region>
          <div className="titlebar-icon" data-tauri-drag-region>
            <LogoIcon size={14} />
          </div>
          <span className="titlebar-name" data-tauri-drag-region>SendSent</span>
        </div>

        <div className="titlebar-center" data-tauri-drag-region>
          <div className="ribbon">
            {navItems.map((it) => (
              <button
                key={it.id}
                className={`ribbon-tab${tab === it.id ? " active" : ""}`}
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
          <div className="win-controls" data-tauri-drag-region>
            <button className="win-btn" data-tauri-drag-region aria-label="最小化">
              <svg width="10" height="10" viewBox="0 0 10 10"><rect x="2" y="5" width="6" height="1" fill="currentColor"/></svg>
            </button>
            <button className="win-btn" data-tauri-drag-region aria-label="最大化">
              <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor"><rect x="2" y="2" width="6" height="6"/></svg>
            </button>
            <button className="win-btn close" data-tauri-drag-region aria-label="关闭">
              <svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor"><path d="M2 2 L8 8 M8 2 L2 8"/></svg>
            </button>
          </div>
        </div>
      </header>

      <div className="body">
        <aside className="sidebar">
          <div className="nav-section">
            <div className="nav-section-label">本机</div>
            <div className="sidebar-id">
              <div className="dot online" />
              <div className="sidebar-id-text">
                <div className="sidebar-id-name">SendSent</div>
                <div className="sidebar-id-meta">端口 52225 · 在线</div>
              </div>
            </div>
          </div>
          <div className="nav-section">
            <div className="nav-section-label">导航</div>
            {navItems.map((it) => {
              const Icon = it.icon;
              return (
                <button
                  key={it.id}
                  className={`nav-item${tab === it.id ? " active" : ""}`}
                  onClick={() => setTab(it.id)}
                >
                  <Icon size={14} />
                  <span>{it.label}</span>
                  {it.badge !== undefined && <span className="nav-badge">{it.badge}</span>}
                </button>
              );
            })}
          </div>
          <div className="nav-section">
            <div className="nav-section-label">收藏</div>
            {peers.slice(0, 3).map((p) => (
              <div className="nav-fav" key={p.device_id}>
                <div className={`peer-avatar small ${p.platform}`}>{p.name.slice(0, 1).toUpperCase()}</div>
                <span>{p.name}</span>
              </div>
            ))}
            {peers.length === 0 && <div className="nav-empty">暂无设备</div>}
          </div>
          <div className="sidebar-footer">
            <button
              className={`refresh-btn${refreshing ? " refreshing" : ""}`}
              onClick={refresh}
            >
              <RefreshIcon size={11} />
              <span>刷新设备</span>
            </button>
            <div className="shortcut-hints">
              <kbd className="kbd">Ctrl ,</kbd><span>偏好</span>
              <kbd className="kbd">Ctrl R</kbd><span>刷新</span>
              <kbd className="kbd">Ctrl 1-3</kbd><span>切换</span>
            </div>
          </div>
        </aside>

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
