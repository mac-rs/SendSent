// 平台层共享类型
// 每个平台模块(macos/ios/android/windows/linux)导出一个 PlatformApp
// 组件 + 配套的 CSS import。PlatformApp 接收共享状态 + 渲染函数,
// 由 src/App.tsx 中的 PlatformRoute 统一调度。

import type { ReactNode } from "react";
import type { Peer } from "../lib/types";
import type { ProgressView, RequestView } from "../hooks/useTransfer";
import type { HistoryRecord } from "../lib/types";
import type { CommandItem } from "../components/CommandPalette";

export type ThemePref = "auto" | "light" | "dark";
export type AppTab = "devices" | "transfers" | "profile" | "settings";

// 平台 App 接收的全部状态(由 App.tsx 集中管理,平台层只读)
export interface PlatformAppProps {
  // 路由状态
  tab: AppTab;
  setTab: (t: AppTab) => void;
  theme: ThemePref;
  setTheme: (t: ThemePref) => void;
  // 平台信息
  platform: "macos" | "windows" | "linux" | "ios" | "android" | "unknown";
  layout: "desktop" | "mobile";
  // 业务状态
  peers: Peer[];
  selected: Peer[];
  togglePeer: (p: Peer) => void;
  selectOnly: (p: Peer) => void;
  clearSelection: () => void;
  refreshing: boolean;
  refresh: () => Promise<number>;
  progress: Record<string, ProgressView>;
  history: HistoryRecord[];
  request: RequestView | null;
  // 命令面板
  commands: CommandItem[];
  cmdOpen: boolean;
  setCmdOpen: (b: boolean) => void;
  // 模态/面板
  prefsOpen: boolean;
  setPrefsOpen: (b: boolean) => void;
  addSheetOpen: boolean;
  setAddSheetOpen: (b: boolean) => void;
  // macOS 专属:sidebar 折叠
  sidebarCollapsed: boolean;
  setSidebarCollapsed: (b: boolean) => void;
  // 操作
  clearHistory: () => void;
  clearRequest: () => void;
  respond: (sessionId: string, accept: boolean) => void;
  // 文案快讯
  toast: { text: string; tone?: "ok" | "err" } | null;
}

// 平台模块形状
export interface PlatformModule {
  // 显示名(中文,用于 about 页等)
  displayName: string;
  // 平台 CSS 类名(挂到 .app 上)
  className: string;
  // 平台 App 组件
  App: (props: PlatformAppProps) => ReactNode;
  // 平台专属 CSS 入口(由 main.tsx 引入,或在模块内 import)
}

// 平台检测 + 路由
import { detectPlatform } from "../lib/platform";

export function pickPlatformModule(
  mods: Record<string, PlatformModule>,
): PlatformModule {
  const p = detectPlatform();
  return mods[p] ?? mods.unknown ?? mods.macos;
}
