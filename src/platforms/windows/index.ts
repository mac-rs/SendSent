// Windows 平台模块
// 视觉借鉴:
//   · Mica 半透明 titlebar(类似 macOS,但用 Segoe UI Variable)
//   · 标题栏左侧是 app icon + 名称,右侧最小/最大/关闭
//   · 副色:Windows 11 accent (#0078D4)
// 布局基本沿用 macOS,但去掉 traffic lights,换成 Windows caption 按钮

import type { PlatformModule } from "../types";
import { WindowsApp } from "./WindowsApp";

export const displayName = "Windows";
export const className = "windows";
export const App = WindowsApp;

const mod: PlatformModule = { displayName, className, App };
export default mod;
