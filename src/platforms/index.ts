// 平台模块注册表
// 桌面 + Android 各一个 PlatformModule,PlatformRoute 按当前平台挑一个。
// (iOS 已改为原生 SwiftUI,不再走 Web)

import type { PlatformModule } from "./types";
import * as macos from "./macos";
import * as android from "./android";
import * as windows from "./windows";
import * as linux from "./linux";

export const platformModules: Record<string, PlatformModule> = {
  macos,
  android,
  windows,
  linux,
  unknown: macos, // 桌面 unknown fallback 到 macOS 布局
};

export type { PlatformModule, PlatformAppProps, AppTab, ThemePref } from "./types";
