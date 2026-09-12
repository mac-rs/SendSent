// iOS 平台模块
// 布局:
//   ├─ Status bar (9:41 + signal/wifi/battery)
//   ├─ Navbar (inline title · 大标题,滚动收缩)
//   ├─ Content (iOS Inset Grouped List · Liquid Glass 传输卡片)
//   ├─ Bottom: 玻璃 tab bar + 中央 FAB + home indicator
//   └─ Sheets (half-sheet 转全屏) + Alert (request)
//
// 设计参考 docs/design-mocks/ios.html

import type { PlatformModule } from "../types";
import { IosApp } from "./IosApp";

export const displayName = "iOS";
export const className = "ios";
export const App = IosApp;

const mod: PlatformModule = { displayName, className, App };
export default mod;
