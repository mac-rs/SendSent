// Android 平台布局
// Material 3 风格:
//   · 顶部 AppBar(中等高度,左侧菜单可选,右侧 actions)
//   · 内容区:Material 3 Card + List
//   · 底部:NavigationBar(无 FAB,新建走 AppBar 的 + 按钮)
//   · FAB:仅在 devices tab 出现(extended FAB)
//
// 复用现有 React 组件,只换 wrapper 和样式
// Android 12+ 支持 Material You dynamic color,这里用 JS 从 wallpaper 提取 fallback
// (因为 webview 无法直接读 Android 系统的 wallpaper / monet palette)

import { useEffect, useState } from "react";
import type { PlatformAppProps } from "../types";
import { PeerList } from "../../components/PeerList";
import { FilePicker } from "../../components/FilePicker";
import { TransferProgress, TransferHistory } from "../../components/TransferProgress";
import { IncomingRequest } from "../../components/IncomingRequest";
import { AddDeviceSheet } from "../../components/AddDeviceSheet";
import { PrefsPanel } from "../../components/PrefsPanel";
import { CommandPalette } from "../../components/CommandPalette";
import { Ripple } from "../../components/Ripple";
import {
  applyAndroidDynamicColor,
  extractPaletteFromUrl,
  type MaterialPalette,
} from "../../lib/dynamicColor";
import {
  DevicesIcon, TransferIcon, SettingsIcon, PlusIcon, SearchIcon,
} from "../../components/Icons";

export function AndroidApp(props: PlatformAppProps) {
  const {
    tab, setTab, theme: _t, setTheme: _st, peers, selected, togglePeer,
    refreshing: _r, refresh: _refresh, progress, history, request,
    prefsOpen, setPrefsOpen, addSheetOpen, setAddSheetOpen,
    commands, cmdOpen, setCmdOpen, clearHistory, clearRequest, respond, toast,
  } = props;

  const [search] = useState("");
  const [palette, setPalette] = useState<MaterialPalette | null>(null);

  // Material You dynamic color:从 wallhaven 等 sample 提取
  // 真实 Android 上是系统注入的 monets palette,这里做 web fallback
  useEffect(() => {
    const url = localStorage.getItem("sendsent.android.wallpaper");
    if (!url) return;
    const isDark = document.documentElement.getAttribute("data-theme") === "dark";
    extractPaletteFromUrl(url, isDark)
      .then(setPalette)
      .catch(() => {/* ignore */});
  }, []);

  useEffect(() => {
    if (palette) applyAndroidDynamicColor(palette);
  }, [palette]);

  const onPickWallpaper = () => {
    const url = window.prompt(
      "贴入壁纸 URL 启用 Material You 动态取色 (留空使用默认紫色):",
      "https://w.wallhaven.cc/full/wq/wallhaven-wq8r6p.jpg",
    );
    if (url === null) return;
    if (url === "") {
      localStorage.removeItem("sendsent.android.wallpaper");
      setPalette(null);
      // 重置 m3-* 到默认
      ["--m3-primary", "--m3-on-primary", "--m3-primary-container",
       "--m3-secondary", "--m3-tertiary", "--m3-surface"].forEach((k) =>
        document.documentElement.style.removeProperty(k));
      return;
    }
    localStorage.setItem("sendsent.android.wallpaper", url);
    const isDark = document.documentElement.getAttribute("data-theme") === "dark";
    extractPaletteFromUrl(url, isDark).then(setPalette).catch(() => {/* ignore */});
  };

  const transfers = Object.values(progress);
  const activeCount = transfers.filter((t) => !t.done).length;
  const tabs = [
    { id: "devices" as const, label: "设备", icon: DevicesIcon, badge: peers.length },
    { id: "transfers" as const, label: "传输", icon: TransferIcon, badge: activeCount },
    { id: "settings" as const, label: "设置", icon: SettingsIcon },
  ];

  return (
    <div className="app android mobile" data-platform="android">
      {/* AppBar (M3 medium) */}
      <header className="appbar">
        <div className="appbar-head">
          <div className="appbar-title">{label(tab)}</div>
          <div className="appbar-actions">
            <button className="icon-btn" onClick={() => setCmdOpen(true)} title="搜索">
              <SearchIcon size={20} />
            </button>
            {(tab === "devices" || tab === "settings") && (
              <button
                className="icon-btn"
                onClick={() => setAddSheetOpen(true)}
                title="添加"
              >
                <PlusIcon size={22} />
              </button>
            )}
          </div>
        </div>
        {tab === "devices" && peers.length > 0 && (
          <div className="appbar-search">
            <SearchIcon size={16} />
            <input placeholder="搜索设备" defaultValue={search} />
          </div>
        )}
      </header>

      {/* Content */}
      <main className="content">
        {tab === "devices" && (
          <>
            <PeerList peers={peers} selected={selected} onToggle={togglePeer} />
            {selected.length > 0 && <FilePicker peers={selected} />}
          </>
        )}
        {tab === "transfers" && (
          <>
            <TransferProgress items={transfers} />
            {history.length > 0 && (
              <>
                <div className="section-title">
                  <h2>历史</h2>
                  <button className="btn btn-text" onClick={clearHistory}>清空</button>
                </div>
                <TransferHistory items={history} onClear={clearHistory} />
              </>
            )}
          </>
        )}
        {tab === "settings" && (
          <div className="m3-list">
            <Ripple>
            <button className="m3-row" onClick={() => setPrefsOpen(true)}>
              <div className="m3-row-icon" style={{ background: "var(--m3-primary)" }}>
                <SettingsIcon size={18} />
              </div>
              <div className="info">
                <div className="name">偏好设置</div>
                <div className="sub">显示名称、保存目录、传输参数</div>
              </div>
            </button>
            </Ripple>
            <Ripple>
            <button className="m3-row" onClick={() => setAddSheetOpen(true)}>
              <div className="m3-row-icon" style={{ background: "var(--m3-tertiary)" }}>
                <PlusIcon size={18} />
              </div>
              <div className="info">
                <div className="name">添加设备</div>
                <div className="sub">手动输入或扫码</div>
              </div>
            </button>
            </Ripple>
            <Ripple>
            <button className="m3-row" onClick={() => setCmdOpen(true)}>
              <div className="m3-row-icon" style={{ background: "var(--m3-secondary)" }}>
                <SearchIcon size={18} />
              </div>
              <div className="info">
                <div className="name">命令面板</div>
                <div className="sub">快速跳转到任意操作</div>
              </div>
            </button>
            </Ripple>
            <Ripple>
            <button className="m3-row" onClick={onPickWallpaper}>
              <div
                className="m3-row-icon"
                style={{ background: "var(--m3-primary)" }}
              >
                <span style={{ font: "700 14px var(--font-m3)" }}>M</span>
              </div>
              <div className="info">
                <div className="name">Material You 动态取色</div>
                <div className="sub">
                  {palette
                    ? `已应用: ${palette.primary}`
                    : "默认紫色,贴入壁纸 URL 启用"}
                </div>
              </div>
            </button>
            </Ripple>
          </div>
        )}
      </main>

      {/* Navigation bar (M3) */}
      <nav className="m3-navbar" aria-label="主导航">
        {tabs.map((t) => {
          const Icon = t.icon;
          const active = tab === t.id;
          const badge = t.badge ?? 0;
          return (
            <button
              key={t.id}
              className={`m3-nav-item${active ? " active" : ""}`}
              onClick={() => setTab(t.id)}
            >
              <div className="m3-nav-pill">
                <Icon size={20} />
                {badge > 0 && active && <span className="badge">{badge}</span>}
              </div>
              <span className="m3-nav-label">{t.label}</span>
            </button>
          );
        })}
      </nav>

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
        <div className={`app-toast${toast.tone === "err" ? " err" : ""}`}>{toast.text}</div>
      )}
    </div>
  );
}

function label(t: PlatformAppProps["tab"]): string {
  return t === "devices" ? "设备" : t === "transfers" ? "传输" : "设置";
}
