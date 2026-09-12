// 平台模块注册表
// 5 个平台各自一个 PlatformModule,PlatformRoute 按当前平台挑一个。

import type { PlatformModule } from "./types";
import * as macos from "./macos";
import * as ios from "./ios";
import * as android from "./android";
import * as windows from "./windows";
import * as linux from "./linux";

export const platformModules: Record<string, PlatformModule> = {
  macos,
  ios,
  android,
  windows,
  linux,
  unknown: macos, // 桌面 unknown fallback 到 macOS 布局
};

export type { PlatformModule, PlatformAppProps, AppTab, ThemePref } from "./types";
