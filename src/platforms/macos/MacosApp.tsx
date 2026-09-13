// macOS 平台布局
// 实现 docs/design-mocks/macos.html 的设计:
//   · 44px 原生 titlebar(traffic lights + segmented tab + 右侧 ⌘F/⌘⇧P/⌘,)
//   · 240px Liquid Glass sidebar(分组导航 + 在线状态点 + 角标)
//   · 主区:大标题 hero + searchbar + stat strip + device grid + 传输卡片
//   · 右下 floating dock(FAB 式 quick actions)

import { useMemo, useState } from "react";
import type { PlatformAppProps } from "../types";
import { PeerList } from "../../components/PeerList";
import { Radar } from "../../components/Radar";
import { FilePicker } from "../../components/FilePicker";
import { TransferProgress, TransferHistory } from "../../components/TransferProgress";
import { IncomingRequest } from "../../components/IncomingRequest";
import { AddDeviceSheet } from "../../components/AddDeviceSheet";
import { PrefsPanel } from "../../components/PrefsPanel";
import { CommandPalette } from "../../components/CommandPalette";
import { ProfilePage } from "../../components/ProfilePage";
import { TransferConfigEditor } from "../../components/TransferConfigEditor";
import { SettingsForm } from "../../components/SettingsForm";
import {
  DevicesIcon, TransferIcon, SettingsIcon, SearchIcon, PlusIcon,
  RefreshIcon, QrCodeIcon, ScanIcon, PersonIcon, FunnelIcon,
  SunIcon, MoonIcon,
} from "../../components/Icons";

export function MacosApp(props: PlatformAppProps) {
  const {
    tab, setTab, theme, setTheme, peers, selected, togglePeer, selectOnly, clearSelection,
    refreshing, refresh,
    progress, history, request, prefsOpen, setPrefsOpen, addSheetOpen, setAddSheetOpen,
    commands, cmdOpen, setCmdOpen, sidebarCollapsed, setSidebarCollapsed,
    clearHistory, clearRequest, respond, toast,
  } = props;

  const transfers = Object.values(progress);
  const activeCount = transfers.filter((t) => !t.done).length;

  const [tDir, setTDir] = useState<"all" | "sent" | "recv">("all");
  const [tStatus, setTStatus] = useState<"all" | "done" | "failed">("all");
  const filteredHistory = useMemo(
    () => history.filter((h) => {
      const d = tDir === "all" || (tDir === "sent" ? h.direction === "send" : h.direction === "recv");
      const s = tStatus === "all" || (tStatus === "done" ? h.status === "completed" : h.status !== "completed");
      return d && s;
    }),
    [history, tDir, tStatus],
  );

  const navItems = useMemo(() => [
    { id: "devices" as const, label: "附近", icon: DevicesIcon, badge: peers.length },
    { id: "transfers" as const, label: "传输", icon: TransferIcon, badge: activeCount || undefined },
    { id: "profile" as const, label: "我的", icon: PersonIcon },
    { id: "settings" as const, label: "设置", icon: SettingsIcon },
  ], [peers.length, activeCount]);

  const ThemeIconEl = theme === "dark" ? MoonIcon : SunIcon;

  return (
    <div className="app macos desktop" data-platform="macos">
      {/* ── Titlebar ──────────────────────────────────────── */}
      <header className="titlebar" data-tauri-drag-region>
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
            onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
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
                <h1 className="hero-title">附近</h1>
                <p className="hero-sub">
                  {peers.length === 0
                    ? "正在同一局域网里寻找…"
                    : `同一局域网 · 找到 ${peers.length} 台设备`}
                </p>
              </div>
              <Radar
                peers={peers}
                meName="我"
                selectedId={selected[0]?.device_id}
                onSelect={selectOnly}
                onDeselect={clearSelection}
              />
              {selected.length > 0 ? (
                <FilePicker peers={selected} />
              ) : (
                <p className="radar-hint">点按设备选择,再选择文件发送</p>
              )}
              <div className="section-title">
                <h2>全部设备</h2>
              </div>
              <PeerList peers={peers} selected={selected} onToggle={togglePeer} />
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
              </div>
              <div className="filter-bar">
                <span className="filter-bar-label"><FunnelIcon size={13} />筛选</span>
                <div className="filter-chips">
                  {([["all", "全部"], ["sent", "发送"], ["recv", "接收"]] as const).map(([v, l]) => (
                    <button
                      key={v}
                      className={`filter-chip${tDir === v ? " active" : ""}`}
                      onClick={() => setTDir(v)}
                    >
                      {l}
                    </button>
                  ))}
                  <span className="filter-sep" />
                  {([["all", "全部"], ["done", "完成"], ["failed", "失败"]] as const).map(([v, l]) => (
                    <button
                      key={v}
                      className={`filter-chip${tStatus === v ? " active" : ""}`}
                      onClick={() => setTStatus(v)}
                    >
                      {l}
                    </button>
                  ))}
                </div>
              </div>
              <TransferHistory items={filteredHistory} onClear={clearHistory} />
            </>
          )}

          {tab === "profile" && <ProfilePage />}

          {tab === "settings" && (
            <>
              <div className="hero">
                <h1 className="hero-title">设置</h1>
                <p className="hero-sub">外观、传输与关于</p>
              </div>

              <div className="settings-group">
                <div className="settings-group-label">外观</div>
                <div className="settings-card">
                  <div className="settings-row">
                    <span className="settings-row-label">主题</span>
                    <div className="segmented sm">
                      {(["auto", "light", "dark"] as const).map((v) => (
                        <button
                          key={v}
                          className={`seg-btn${theme === v ? " active" : ""}`}
                          onClick={() => setTheme(v)}
                        >
                          {v === "auto" ? "跟随系统" : v === "light" ? "浅色" : "深色"}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>
              </div>

              <div className="settings-group">
                <div className="settings-group-label">传输参数</div>
                <div className="settings-card"><TransferConfigEditor /></div>
              </div>

              <div className="settings-group">
                <div className="settings-group-label">关于</div>
                <div className="settings-card"><SettingsForm /></div>
              </div>
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
