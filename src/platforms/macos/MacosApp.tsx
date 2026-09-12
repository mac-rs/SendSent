// macOS 平台布局
// 实现 docs/design-mocks/macos.html 的设计:
//   · 44px 原生 titlebar(traffic lights + segmented tab + 右侧 ⌘F/⌘⇧P/⌘,)
//   · 240px Liquid Glass sidebar(分组导航 + 在线状态点 + 角标)
//   · 主区:大标题 hero + searchbar + stat strip + device grid + 传输卡片
//   · 右下 floating dock(FAB 式 quick actions)

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
  DevicesIcon, TransferIcon, SettingsIcon, SearchIcon, PlusIcon,
  RefreshIcon, QrCodeIcon, ScanIcon,
  SunIcon, MoonIcon, AutoIcon,
} from "../../components/Icons";

function cycleTheme(t: PlatformAppProps["theme"]) {
  return t === "auto" ? "light" : t === "light" ? "dark" : "auto";
}

export function MacosApp(props: PlatformAppProps) {
  const {
    tab, setTab, theme, setTheme, peers, selected, togglePeer, refreshing, refresh,
    progress, history, request, prefsOpen, setPrefsOpen, addSheetOpen, setAddSheetOpen,
    commands, cmdOpen, setCmdOpen, sidebarCollapsed, setSidebarCollapsed,
    clearHistory, clearRequest, respond, toast,
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
    <div className="app macos desktop" data-platform="macos">
      {/* ── Titlebar ──────────────────────────────────────── */}
      <header className="titlebar" data-tauri-drag-region>
        <div className="traffic-lights" data-tauri-drag-region>
          <span className="close" data-tauri-drag-region />
          <span className="min" data-tauri-drag-region />
          <span className="max" data-tauri-drag-region />
        </div>

        <div className="titlebar-center" data-tauri-drag-region>
          <div className="segmented">
            {navItems.map((it) => (
              <button
                key={it.id}
                className={`seg-btn${tab === it.id ? " active" : ""}`}
                onClick={() => setTab(it.id)}
              >
                {it.label}
                {it.badge !== undefined && it.badge > 0 && (
                  <span className="seg-badge">{it.badge}</span>
                )}
              </button>
            ))}
          </div>
        </div>

        <div className="titlebar-right" data-tauri-drag-region>
          <button
            className="icon-btn"
            title="折叠侧栏 (⌘B)"
            onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="3" width="18" height="18" rx="2"/>
              <path d={sidebarCollapsed ? "M9 3v18" : "M15 3v18"}/>
            </svg>
          </button>
          <button className="icon-btn" title="搜索 (⌘F)">
            <SearchIcon size={14} />
          </button>
          <button className="icon-btn" title="命令面板 (⌘⇧P)" onClick={() => setCmdOpen(true)}>
            <span style={{ font: "500 11px var(--font-mono)", letterSpacing: 1 }}>⌘⇧P</span>
          </button>
          <button className="icon-btn" title="偏好设置 (⌘,)" onClick={() => setPrefsOpen(true)}>
            <SettingsIcon size={14} />
          </button>
          <button
            className="icon-btn"
            title="主题"
            onClick={() => setTheme(cycleTheme(theme))}
          >
            <ThemeIconEl size={14} />
          </button>
        </div>
      </header>

      {/* ── Body ──────────────────────────────────────────── */}
      <div className={`body${sidebarCollapsed ? " collapsed" : ""}`}>
        {/* Sidebar */}
        <aside className="sidebar">
          <div className="nav-section nav-section-id">
            {/* 折叠态用:小设备图标,带 tooltip 提示本机信息 */}
            <div className="sidebar-id-icon" title={`本机设备 · 端口 52225 · 在线`}>
              <DevicesIcon size={16} />
            </div>
            <div className="sidebar-id">
              <div className="dot online" />
              <div className="sidebar-id-text">
                <div className="sidebar-id-name">本机设备</div>
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
                  role="tab"
                  aria-selected={tab === it.id}
                  title={it.label}
                >
                  <Icon size={14} />
                  <span className="nav-label">{it.label}</span>
                  {it.badge !== undefined && it.badge > 0 && (
                    <span className="nav-badge">{it.badge}</span>
                  )}
                </button>
              );
            })}
          </div>

          <div className="nav-section nav-section-fav">
            <div className="nav-section-label">收藏</div>
            {peers.slice(0, 3).map((p) => (
              <div className="nav-fav" key={p.device_id} title={p.name}>
                <div className={`peer-avatar small ${p.platform}`}>{p.name.slice(0, 1).toUpperCase()}</div>
                <span className="nav-label">{p.name}</span>
              </div>
            ))}
            {peers.length > 3 && (
              <div className="nav-fav nav-fav-more" title={`还有 ${peers.length - 3} 个`}>
                <div className="peer-avatar small more">+{peers.length - 3}</div>
              </div>
            )}
            {peers.length === 0 && (
              <div className="nav-empty">暂无设备</div>
            )}
          </div>

          <div className="sidebar-footer">
            <button
              className={`refresh-btn${refreshing ? " refreshing" : ""}`}
              onClick={refresh}
              title="刷新设备 (⌘R)"
            >
              <RefreshIcon size={11} />
              <span className="nav-label">刷新设备</span>
            </button>
            <div className="shortcut-hints">
              <kbd className="kbd">⌘,</kbd><span>偏好</span>
              <kbd className="kbd">⌘R</kbd><span>刷新</span>
              <kbd className="kbd">⌘B</kbd><span>折叠</span>
            </div>
          </div>
        </aside>

        {/* Main */}
        <main className="main">
          {tab === "devices" && (
            <>
              <div className="hero">
                <h1 className="hero-title">设备</h1>
                <p className="hero-sub">
                  {peers.length === 0
                    ? "正在发现附近的局域网设备…"
                    : `已发现 ${peers.length} 台设备 · ${selected.length} 台已选择`}
                </p>
                <div className="searchbar">
                  <SearchIcon size={13} />
                  <input placeholder="搜索设备、文件…" />
                  <span className="kbd">⌘F</span>
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
                <p className="hero-sub">
                  {activeCount > 0 ? `${activeCount} 个进行中` : "当前无活跃传输"}
                </p>
              </div>
              <TransferProgress items={transfers} />
              <div className="section-title">
                <h2>历史</h2>
                {history.length > 0 && (
                  <button className="btn btn-ghost" onClick={clearHistory}>清空记录</button>
                )}
              </div>
              <TransferHistory items={history} onClear={clearHistory} />
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

      {/* Floating dock (macOS only) */}
      <div className="floating-dock">
        <button title="添加设备" onClick={() => setAddSheetOpen(true)}>
          <PlusIcon size={14} />
        </button>
        <button title="我的 QR" onClick={() => setPrefsOpen(true)}>
          <QrCodeIcon size={14} />
        </button>
        <div className="dock-divider" />
        <button title="扫码添加" onClick={() => setAddSheetOpen(true)}>
          <ScanIcon size={14} />
        </button>
        <button title="主题" onClick={() => setTheme(cycleTheme(theme))}>
          <ThemeIconEl size={14} />
        </button>
      </div>

      {/* Overlays */}
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
        <div className={`app-toast${toast.tone === "err" ? " err" : ""}`}>
          {toast.text}
        </div>
      )}
    </div>
  );
}
