// Linux 平台模块
// 大多数 Linux 桌面 (GNOME / KDE / Hyprland) 都会模拟 macOS-like traffic lights
// 但为了和 macOS 区分:
//   · titlebar 左侧是 app icon + app name(类似 Windows,但用 Adwaita accent)
//   · 副色:GNOME blue (#3584e4) 或 Adwaita default
// 整体上接近 Adwaita 风格 + 一点 KDE Breeze 元素

import type { PlatformModule } from "../types";
import { LinuxApp } from "./LinuxApp";

export const displayName = "Linux";
export const className = "linux";
export const App = LinuxApp;

const mod: PlatformModule = { displayName, className, App };
export default mod;
