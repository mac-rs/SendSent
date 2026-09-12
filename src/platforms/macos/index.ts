// macOS 平台模块
// 布局:
//   ├─ Titlebar (44px,traffic lights + segmented + 右侧 icon 按钮)
//   ├─ Sidebar (240px,设备/收藏/历史导航,带 group label + badge)
//   └─ Main  (大标题 hero + stat strip + 内容区 + 右下 floating dock)

import type { PlatformModule } from "../types";
import { MacosApp } from "./MacosApp";

export const displayName = "macOS";
export const className = "macos";
export const App = MacosApp;

const mod: PlatformModule = { displayName, className, App };
export default mod;
