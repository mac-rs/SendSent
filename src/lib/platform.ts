// 平台 + 布局模式检测
// 桌面: macOS / Windows / Linux → sidebar 布局
// 移动: iOS / Android → bottom tab bar 布局
// 当 UA 检测不可用时回退到视口宽度(< 720 视为移动)

import { useEffect, useState } from "react";

export type Platform = "macos" | "windows" | "linux" | "ios" | "android" | "unknown";
export type LayoutMode = "desktop" | "mobile";

export function detectPlatform(): Platform {
  if (typeof navigator === "undefined") return "unknown";
  const ua = navigator.userAgent;
  if (/iPhone|iPad|iPod/.test(ua)) return "ios";
  if (/Android/.test(ua)) return "android";
  if (/Windows/.test(ua)) return "windows";
  if (/Macintosh|Mac OS X/.test(ua) && !/iPhone|iPad/.test(ua)) return "macos";
  if (/Linux/.test(ua) && !/Android/.test(ua)) return "linux";
  return "unknown";
}

export function detectLayout(platform: Platform, width: number): LayoutMode {
  if (platform === "ios" || platform === "android") return "mobile";
  return width < 720 ? "mobile" : "desktop";
}

export function usePlatform() {
  const [platform, setPlatform] = useState<Platform>(() => detectPlatform());
  const [layout, setLayout] = useState<LayoutMode>(() => {
    if (typeof window === "undefined") return "desktop";
    return detectLayout(detectPlatform(), window.innerWidth);
  });

  useEffect(() => {
    const onResize = () => setLayout(detectLayout(detectPlatform(), window.innerWidth));
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  useEffect(() => setPlatform(detectPlatform()), []);

  return { platform, layout };
}
