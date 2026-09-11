// 触觉反馈桥接 · 跨平台
// iOS: 调用 Tauri native haptic(后续通过 plugin-bridge 暴露)
// 其他: navigator.vibrate(Android Chrome 支持)
// 桌面: 静默(无设备)

export type HapticIntensity = "light" | "medium" | "heavy" | "success" | "warning" | "error";

const PATTERNS: Record<HapticIntensity, number | number[]> = {
  light: 10,
  medium: 20,
  heavy: 30,
  success: [10, 50, 20],
  warning: [20, 40, 20],
  error: [30, 60, 30],
};

let inIosWebview = false;
try {
  // Tauri iOS webview 暴露 window.__TAURI_INTERNALS__
  inIosWebview = !!(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
    && /iPhone|iPad|iPod/i.test(navigator.userAgent);
} catch {
  // ignore
}

let lastFireAt = 0;
const MIN_GAP = 50; // 50ms 节流,避免连续抖动

export function haptic(intensity: HapticIntensity = "light"): void {
  const now = Date.now();
  if (now - lastFireAt < MIN_GAP) return;
  lastFireAt = now;

  // iOS Tauri: 调用 native haptics(native 端需注册 command)
  if (inIosWebview && typeof (window as unknown as {
    __TAURI__?: { invoke?: (cmd: string, args?: unknown) => Promise<unknown> };
  }).__TAURI__?.invoke === "function") {
    (window as unknown as {
      __TAURI__: { invoke: (cmd: string, args?: unknown) => Promise<unknown> };
    }).__TAURI__.invoke("haptic", { intensity }).catch(() => fallback(intensity));
    return;
  }

  fallback(intensity);
}

function fallback(intensity: HapticIntensity): void {
  // Android Chrome / DevTools 模拟
  if (typeof navigator.vibrate === "function") {
    try { navigator.vibrate(PATTERNS[intensity]); } catch { /* ignore */ }
  }
}
