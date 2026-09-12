// 应用根组件
// 只负责:
//   1. 状态管理(tab / theme / 模态开关 / toast / 业务状态)
//   2. 平台路由:根据 detectPlatform() 选一个 PlatformModule
//   3. 快捷键套件
//   4. 命令面板的 command 列表生成
// 平台布局的细节(标题栏 / sidebar / tab bar / sheet)全部由 platforms/* 接管

import { useEffect, useMemo, useState } from "react";
import { usePeers } from "./hooks/usePeers";
import { useTransfer } from "./hooks/useTransfer";
import { usePlatform } from "./lib/platform";
import { respond, splashReady } from "./lib/invoke";
import type { Peer } from "./lib/types";
import {
  DevicesIcon, TransferIcon, SettingsIcon, PlusIcon, RefreshIcon,
  SunIcon, MoonIcon, AutoIcon,
} from "./components/Icons";
import type { CommandItem } from "./components/CommandPalette";
import { platformModules } from "./platforms";
import type { AppTab, ThemePref, PlatformAppProps } from "./platforms";

const TAB_ORDER: AppTab[] = ["devices", "transfers", "settings"];

export default function App() {
  const { request, progress, history, clearHistory, clearRequest } = useTransfer();
  const { platform, layout } = usePlatform();
  const { peers, refreshing, refresh: refreshPeers } = usePeers();

  // ── 路由状态 ──────────────────────────────────────────────
  const [tab, setTab] = useState<AppTab>("devices");
  const [theme, setTheme] = useState<ThemePref>("auto");
  const [selected, setSelected] = useState<Peer[]>([]);
  const [prefsOpen, setPrefsOpen] = useState(false);
  const [addSheetOpen, setAddSheetOpen] = useState(false);
  const [cmdOpen, setCmdOpen] = useState(false);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [toast, setToast] = useState<{ text: string; tone?: "ok" | "err" } | null>(null);

  // ── 主题应用 ──────────────────────────────────────────────
  useEffect(() => {
    const root = document.documentElement;
    if (theme === "auto") root.removeAttribute("data-theme");
    else root.setAttribute("data-theme", theme);
  }, [theme]);

  // ── 通知 Rust 端:主窗口 React 已挂载,可以关闭 splash 并显示 ──
  // 仅桌面端在 Tauri 上下文内有效;浏览器跑 dev server 时调用会抛错,静默忽略。
  useEffect(() => {
    let cancelled = false;
    const t = window.setTimeout(() => {
      if (cancelled) return;
      splashReady().catch(() => { /* 浏览器或重复调用 — 静默 */ });
    }, 220);
    return () => { cancelled = true; window.clearTimeout(t); };
  }, []);

  // ── Toast 工具 ────────────────────────────────────────────
  const showToast = (text: string, tone: "ok" | "err" = "ok") => {
    setToast({ text, tone });
    window.setTimeout(
      () => setToast((cur) => (cur?.text === text ? null : cur)),
      2400,
    );
  };
  const onRefreshWithToast = async () => {
    const n = await refreshPeers();
    showToast(n === 0 ? "未发现其他设备" : `发现 ${n} 台设备`);
  };

  // ── 桌面端快捷键套件 ──────────────────────────────────────
  useEffect(() => {
    if (layout === "mobile") return;
    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (!mod) return;
      const key = e.key.toLowerCase();
      if (key === "p" && e.shiftKey) { e.preventDefault(); setCmdOpen((p) => !p); return; }
      if (key === ",") { e.preventDefault(); setPrefsOpen((p) => !p); return; }
      if (key === "w") {
        e.preventDefault();
        if (prefsOpen) setPrefsOpen(false);
        else if (addSheetOpen) setAddSheetOpen(false);
        else if (request) { respond(request.session_id, false); clearRequest(); }
        return;
      }
      if (key === "1") { e.preventDefault(); setTab("devices"); return; }
      if (key === "2") { e.preventDefault(); setTab("transfers"); return; }
      if (key === "3") { e.preventDefault(); setTab("settings"); return; }
      if (key === "[") { e.preventDefault();
        setTab((t) => TAB_ORDER[Math.max(TAB_ORDER.indexOf(t) - 1, 0)]); return; }
      if (key === "]") { e.preventDefault();
        setTab((t) => TAB_ORDER[Math.min(TAB_ORDER.indexOf(t) + 1, TAB_ORDER.length - 1)]); return; }
      if (key === "r") { e.preventDefault(); onRefreshWithToast(); return; }
      if (key === "b") { e.preventDefault(); setSidebarCollapsed((c) => !c); return; }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [layout, prefsOpen, addSheetOpen, request, onRefreshWithToast, clearRequest]);

  // ── 命令面板列表 ──────────────────────────────────────────
  const commands: CommandItem[] = useMemo(() => [
    { id: "tab.devices", group: "导航", label: "切换到「设备」", hint: "查看局域网发现的设备",
      keywords: ["device", "设备", "tab"], shortcut: "⌘1", action: () => setTab("devices"),
      icon: <DevicesIcon size={14} /> },
    { id: "tab.transfers", group: "导航", label: "切换到「传输」", hint: "查看进行中 / 已完成的传输",
      keywords: ["transfer", "传输", "tab"], shortcut: "⌘2", action: () => setTab("transfers"),
      icon: <TransferIcon size={14} /> },
    { id: "tab.settings", group: "导航", label: "切换到「设置」", hint: "配置应用偏好",
      keywords: ["settings", "prefs", "设置", "tab"], shortcut: "⌘3", action: () => setTab("settings"),
      icon: <SettingsIcon size={14} /> },
    { id: "refresh", group: "设备", label: "刷新设备列表", hint: "重新 mDNS 扫描",
      keywords: ["refresh", "scan", "刷新", "扫描"], shortcut: "⌘R",
      action: () => { onRefreshWithToast(); }, icon: <RefreshIcon size={14} /> },
    { id: "add-device", group: "设备", label: "添加设备", hint: "手动输入 IP:port 或扫码",
      keywords: ["add", "device", "ip", "添加", "扫码"], action: () => setAddSheetOpen(true),
      icon: <PlusIcon size={14} /> },
    { id: "prefs", group: "应用", label: "偏好设置", hint: "打开偏好面板",
      keywords: ["prefs", "settings", "设置", "偏好"], shortcut: "⌘,",
      action: () => setPrefsOpen(true), icon: <SettingsIcon size={14} /> },
    { id: "theme.light", group: "主题", label: "切换为浅色",
      keywords: ["theme", "light", "主题", "浅色"], action: () => setTheme("light"),
      icon: <SunIcon size={14} /> },
    { id: "theme.dark", group: "主题", label: "切换为深色",
      keywords: ["theme", "dark", "主题", "深色"], action: () => setTheme("dark"),
      icon: <MoonIcon size={14} /> },
    { id: "theme.auto", group: "主题", label: "跟随系统",
      keywords: ["theme", "auto", "system", "主题", "自动", "系统"], action: () => setTheme("auto"),
      icon: <AutoIcon size={14} /> },
  ], [onRefreshWithToast]);

  // ── 业务操作 ──────────────────────────────────────────────
  const togglePeer = (p: Peer) =>
    setSelected((c) =>
      c.some((x) => x.device_id === p.device_id)
        ? c.filter((x) => x.device_id !== p.device_id)
        : [...c, p]
    );

  // ── 平台路由 ──────────────────────────────────────────────
  const platformMod = platformModules[platform] ?? platformModules.macos;
  const PlatformApp = platformMod.App;

  const platformProps: PlatformAppProps = {
    tab, setTab,
    theme, setTheme,
    platform,
    layout,
    peers, selected, togglePeer,
    refreshing, refresh: async () => { await refreshPeers(); return peers.length; },
    progress, history, request,
    prefsOpen, setPrefsOpen, addSheetOpen, setAddSheetOpen,
    commands, cmdOpen, setCmdOpen,
    sidebarCollapsed, setSidebarCollapsed,
    clearHistory, clearRequest,
    respond,
    toast,
  };

  return <PlatformApp {...platformProps} />;
}

// 兼容:某些组件仍 import `App` 命名导出(避免 breaking)
export { App };
