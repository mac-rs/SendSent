// Android 平台布局 — Material 3
// 对齐 docs/design-mocks/android.html:
//   · 沉浸式状态栏:AppBar 背景延伸(顶部 padding 用 safe-area-inset)
//   · M3 Top AppBar(surface-container + 48px 圆形 action)
//   · M3 Search Bar + M3 List + Extended FAB
//   · M3 NavigationBar(active 用 64×32 pill,底部手势条避让)
// Material You dynamic color:JS 从壁纸 URL 提取 palette
// (webview 无法直接读 Android 系统的 monet palette)

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
  ChevronRightIcon,
} from "../../components/Icons";

export function AndroidApp(props: PlatformAppProps) {
  const {
    tab, setTab, theme: _t, setTheme: _st, peers, selected, togglePeer,
    refreshing: _r, refresh: _refresh, progress, history, request,
    prefsOpen, setPrefsOpen, addSheetOpen, setAddSheetOpen,
    commands, cmdOpen, setCmdOpen, clearHistory, clearRequest, respond, toast,
  } = props;

  const [palette, setPalette] = useState<MaterialPalette | null>(null);

  // Material You dynamic color:从壁纸 URL 提取
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
      {/* M3 Top AppBar — 背景延伸至状态栏(沉浸式),内容不遮挡 */}
      <header className="appbar">
        <div className="appbar-row">
          <div className="appbar-title">{label(tab)}</div>
          <button
            className="appbar-action"
            onClick={() => setCmdOpen(true)}
            aria-label="搜索"
          >
            <SearchIcon size={24} />
          </button>
          {(tab === "devices" || tab === "settings") && (
            <button
              className="appbar-action"
              onClick={() => setAddSheetOpen(true)}
              aria-label="添加"
            >
              <PlusIcon size={24} />
            </button>
          )}
        </div>
        {tab === "devices" && peers.length > 0 && (
          <div className="m3-search">
            <SearchIcon size={20} />
            <input placeholder="搜索设备、文件" />
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
              <div className="m3-avatar" style={{ background: "var(--m3-primary-container)", color: "var(--m3-on-primary-container)" }}>
                <SettingsIcon size={20} />
              </div>
              <div className="info">
                <div className="name">偏好设置</div>
                <div className="sub">显示名称、保存目录、传输参数</div>
              </div>
              <div className="trailing"><ChevronRightIcon size={20} /></div>
            </button>
            </Ripple>
            <Ripple>
            <button className="m3-row" onClick={() => setAddSheetOpen(true)}>
              <div className="m3-avatar" style={{ background: "var(--m3-tertiary-container)", color: "var(--m3-on-tertiary-container)" }}>
                <PlusIcon size={20} />
              </div>
              <div className="info">
                <div className="name">添加设备</div>
                <div className="sub">手动输入或扫码</div>
              </div>
              <div className="trailing"><ChevronRightIcon size={20} /></div>
            </button>
            </Ripple>
            <Ripple>
            <button className="m3-row" onClick={() => setCmdOpen(true)}>
              <div className="m3-avatar" style={{ background: "var(--m3-secondary-container)", color: "var(--m3-on-secondary-container)" }}>
                <SearchIcon size={20} />
              </div>
              <div className="info">
                <div className="name">命令面板</div>
                <div className="sub">快速跳转到任意操作</div>
              </div>
              <div className="trailing"><ChevronRightIcon size={20} /></div>
            </button>
            </Ripple>
            <Ripple>
            <button className="m3-row" onClick={onPickWallpaper}>
              <div className="m3-avatar" style={{ background: "var(--m3-primary)", color: "var(--m3-on-primary)" }}>
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
              <div className="trailing"><ChevronRightIcon size={20} /></div>
            </button>
            </Ripple>
          </div>
        )}
      </main>

      {/* M3 Extended FAB — 仅设备页 */}
      {tab === "devices" && (
        <button className="fab-ext" onClick={() => setAddSheetOpen(true)}>
          <PlusIcon size={22} />
          <span>添加设备</span>
        </button>
      )}

      {/* M3 NavigationBar */}
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
                <Icon size={24} />
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
