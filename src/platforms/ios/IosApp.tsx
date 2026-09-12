// iOS 平台布局
// · NavigationStack 大标题(滚动收缩到 17pt inline)
// · Liquid Glass 玻璃 tab bar + 中央 FAB
// · iOS Inset Grouped List 设备列表
// · 半屏 sheet(请求/添加设备/QR)
// · 触觉 + 安全区

import type { PlatformAppProps } from "../types";
import { PeerList } from "../../components/PeerList";
import { FilePicker } from "../../components/FilePicker";
import { TransferProgress, TransferHistory } from "../../components/TransferProgress";
import { IncomingRequest } from "../../components/IncomingRequest";
import { AddDeviceSheet } from "../../components/AddDeviceSheet";
import { PrefsPanel } from "../../components/PrefsPanel";
import { CommandPalette } from "../../components/CommandPalette";
import { PullToRefresh } from "../../components/PullToRefresh";
import { Sheet } from "../../components/Sheet";
import { useSwipeTabs } from "../../lib/useSwipeTabs";
import { useScrollCollapse } from "../../lib/useScrollCollapse";
import {
  DevicesIcon, TransferIcon, SettingsIcon, PlusIcon, ReceiveIcon,
  QrCodeIcon, ScanIcon, SearchIcon, SettingsIcon as CogIcon,
} from "../../components/Icons";

const TAB_ORDER: PlatformAppProps["tab"][] = ["devices", "transfers", "settings"];

export function IosApp(props: PlatformAppProps) {
  const {
    tab, setTab, theme: _theme, setTheme: _setTheme, peers, selected, togglePeer,
    refresh, progress, history, request,
    prefsOpen, setPrefsOpen, addSheetOpen, setAddSheetOpen,
    commands, cmdOpen, setCmdOpen, clearHistory, clearRequest, respond, toast,
  } = props;

  const { collapse, scrolling } = useScrollCollapse(".content");

  // iOS:tab 之间左右滑动切换
  const { progress: swipeProgress } = useSwipeTabs({
    enabled: true,
    onLeft: () =>
      setTab(TAB_ORDER[Math.min(TAB_ORDER.indexOf(tab) + 1, TAB_ORDER.length - 1)]),
    onRight: () =>
      setTab(TAB_ORDER[Math.max(TAB_ORDER.indexOf(tab) - 1, 0)]),
  });

  const transfers = Object.values(progress);
  const activeCount = transfers.filter((t) => !t.done).length;

  const tabs = [
    { id: "devices" as const, label: "设备", icon: DevicesIcon, badge: selected.length },
    { id: "transfers" as const, label: "传输", icon: TransferIcon, badge: activeCount },
    { id: "settings" as const, label: "设置", icon: SettingsIcon },
  ];

  return (
    <div className="app ios mobile" data-platform="ios">
      {/* Status bar */}
      <div className="status-bar">
        <span>9:41</span>
        <div className="status-right">
          <span className="signal" />
          <span className="wifi" />
          <span className="battery" />
        </div>
      </div>

      {/* Navbar — 大标题 + inline title(滚动切换) */}
      <header
        className={`navbar${scrolling ? " scrolling" : ""}`}
        style={{
          maxHeight: 96 + (1 - collapse) * 56,
          paddingTop: 4 + collapse * 0,
        } as React.CSSProperties}
      >
        <div className="nav-head">
          <span
            className="nav-title-inline"
            style={{
              opacity: collapse,
              transform: `translateY(${(1 - collapse) * -4}px)`,
            }}
          >{label(tab)}</span>
          <div className="nav-actions">
            <button className="nav-action" title="搜索">
              <SearchIcon size={18} />
            </button>
            <button className="nav-action" title="偏好" onClick={() => setPrefsOpen(true)}>
              <CogIcon size={18} />
            </button>
          </div>
        </div>
        <h1
          className={`large-title${scrolling ? " scrolling" : ""}`}
          style={{
            fontSize: 34 - collapse * 17,
            letterSpacing: -0.4 + collapse * 0.1,
            paddingBottom: 10 - collapse * 8,
            transform: `translateY(${-collapse * 2}px)`,
            opacity: 1 - collapse * 0.2,
          }}
        >
          {label(tab)}
        </h1>
      </header>

      {/* Content — 3 tab 页共用一个滚动容器,大标题平滑过渡 */}
      <div className="content" key={tab}>
        {tab === "devices" && (
          <>
            {peers.length > 0 && (
              <div className="ios-search">
                <SearchIcon size={15} />
                <input placeholder="搜索设备、文件" />
              </div>
            )}

            {activeCount > 0 && (
              <div className="transfer-glass">
                <div className="head">
                  <span className="head-icon">
                    <TransferIcon size={16} />
                  </span>
                  <div className="name">{transfers.find((t) => !t.done)?.filenames?.[0] ?? "传输中"}</div>
                  <div className="pct">
                    {Math.round(
                      ((transfers.find((t) => !t.done)?.bytes_done ?? 0) /
                        (transfers.find((t) => !t.done)?.bytes_total ?? 1)) *
                        100,
                    )}%
                  </div>
                </div>
                <div className="progress-track">
                  <div
                    className="progress-fill"
                    style={{
                      width: `${Math.min(
                        100,
                        ((transfers.find((t) => !t.done)?.bytes_done ?? 0) /
                          (transfers.find((t) => !t.done)?.bytes_total ?? 1)) *
                          100,
                      )}%`,
                    }}
                  />
                </div>
                <div className="meta">
                  <span>发送中</span>
                  <span className="spacer" />
                </div>
              </div>
            )}

            <PullToRefresh
              onRefresh={async () => { try { await refresh(); } catch { /* ignore */ } }}
            />

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
                  <button className="btn btn-ghost" onClick={clearHistory}>清空</button>
                </div>
                <TransferHistory items={history} onClear={clearHistory} />
              </>
            )}
          </>
        )}

        {tab === "settings" && (
          <>
            <div className="ios-list">
              <button className="ios-row" onClick={() => setPrefsOpen(true)}>
                <span className="ios-row-icon" style={{ background: "var(--ios-blue)" }}>
                  <CogIcon size={16} />
                </span>
                <div className="info">
                  <div className="name">偏好设置</div>
                  <div className="sub">显示名称、保存目录、传输参数</div>
                </div>
                <span className="chev">›</span>
              </button>
              <button className="ios-row" onClick={() => setAddSheetOpen(true)}>
                <span className="ios-row-icon" style={{ background: "var(--ios-green)" }}>
                  <ScanIcon size={16} />
                </span>
                <div className="info">
                  <div className="name">扫码添加设备</div>
                  <div className="sub">用相机扫描对方设备 QR</div>
                </div>
                <span className="chev">›</span>
              </button>
              <button className="ios-row" onClick={() => setCmdOpen(true)}>
                <span className="ios-row-icon" style={{ background: "var(--ios-purple)" }}>
                  <span style={{ font: "500 11px var(--font-rd)" }}>⌘P</span>
                </span>
                <div className="info">
                  <div className="name">命令面板</div>
                  <div className="sub">快速跳转到任意操作</div>
                </div>
                <span className="chev">›</span>
              </button>
            </div>

            <div className="section-label">本机</div>
            <div className="ios-list">
              <div className="ios-row">
                <span className="ios-row-icon" style={{ background: "var(--ios-orange)" }}>
                  <ReceiveIcon size={16} />
                </span>
                <div className="info">
                  <div className="name">本机端口</div>
                  <div className="sub mono">52225</div>
                </div>
              </div>
              <div className="ios-row">
                <span className="ios-row-icon" style={{ background: "var(--ios-pink)" }}>
                  <QrCodeIcon size={16} />
                </span>
                <div className="info">
                  <div className="name">我的设备 QR</div>
                  <div className="sub">让其他设备扫码连接</div>
                </div>
                <span className="chev">›</span>
              </div>
            </div>
          </>
        )}
      </div>

      {/* Bottom: Glass tab bar + FAB */}
      <nav className="tabbar" aria-label="主导航">
        {tabs.slice(0, 2).map((t) => {
          const Icon = t.icon;
          const badge = t.badge ?? 0;
          return (
            <button
              key={t.id}
              className={`tab-item${tab === t.id ? " active" : ""}`}
              onClick={() => setTab(t.id)}
            >
              <div className="icon-wrap">
                <Icon size={22} />
                {badge > 0 && <span className="tab-badge">{badge}</span>}
              </div>
              <span>{t.label}</span>
            </button>
          );
        })}

        <button
          className="tab-fab"
          onClick={() => setAddSheetOpen(true)}
          title="新建"
        >
          <PlusIcon size={22} />
        </button>

        {tabs.slice(2).map((t) => {
          const Icon = t.icon;
          return (
            <button
              key={t.id}
              className={`tab-item${tab === t.id ? " active" : ""}`}
              onClick={() => setTab(t.id)}
            >
              <div className="icon-wrap">
                <Icon size={22} />
              </div>
              <span>{t.label}</span>
            </button>
          );
        })}
      </nav>
      <div className="home-indicator" />

      {/* Swipe progress indicator */}
      {Math.abs(swipeProgress) > 0.01 && (
        <div
          className="swipe-ghost"
          style={{ opacity: Math.min(1, Math.abs(swipeProgress) * 2) }}
        />
      )}

      {/* Overlays — iOS 用半屏 Sheet 包 AddDeviceSheet / PrefsPanel */}
      <Sheet open={addSheetOpen} onClose={() => setAddSheetOpen(false)} size="medium">
        <AddDeviceSheet open={addSheetOpen} onClose={() => setAddSheetOpen(false)} />
      </Sheet>
      <Sheet open={prefsOpen} onClose={() => setPrefsOpen(false)} size="large">
        <PrefsPanel open={prefsOpen} onClose={() => setPrefsOpen(false)} />
      </Sheet>
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

function label(t: PlatformAppProps["tab"]): string {
  return t === "devices" ? "设备" : t === "transfers" ? "传输" : "设置";
}
