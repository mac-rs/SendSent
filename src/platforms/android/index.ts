// Android 平台模块
// 复用 iOS 移动布局,但替换:
//   · Status bar → 实际系统状态(由 webview 渲染)
//   · Tab bar → Material 3 NavigationBar(无 FAB,顶栏放 + 按钮)
//   · List → Material List (rounded corners, ripple)
//   · Card → Material 3 elevated

import type { PlatformModule } from "../types";
import { AndroidApp } from "./AndroidApp";

export const displayName = "Android";
export const className = "android";
export const App = AndroidApp;

const mod: PlatformModule = { displayName, className, App };
export default mod;
