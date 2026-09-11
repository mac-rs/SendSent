// 简化的水平 swipe 手势 · 用于在 tab 之间左右滑动
// 仅用于 iOS mobile
// 返回 progress 表示滑动进度 [-1, 1],可用于显示边缘拖影指示器
//   > 0: 右滑(从屏幕左缘开始,准备返回上一个 tab)
//   < 0: 左滑(从屏幕右缘开始,准备进入下一个 tab)

import { useEffect, useRef, useState } from "react";

export function useSwipeTabs({
  enabled,
  onLeft,   // 手指向左滑 → 进入下一个 tab
  onRight,  // 手指向右滑 → 返回上一个 tab
  threshold = 80,
  edgeWidth = 30,    // 屏幕左缘 30px 才允许 right swipe-back
  maxProgress = 200, // progress 达到该像素值时为 100%
}: {
  enabled: boolean;
  onLeft: () => void;
  onRight: () => void;
  threshold?: number;
  edgeWidth?: number;
  maxProgress?: number;
}) {
  const [progress, setProgress] = useState(0); // -1..1

  // Latest config in a ref: keeps the effect stable so listeners are not
  // re-bound during a gesture (which broke native scrolling on iOS).
  const cfg = useRef({ onLeft, onRight, threshold, edgeWidth, maxProgress });
  cfg.current = { onLeft, onRight, threshold, edgeWidth, maxProgress };

  useEffect(() => {
    if (!enabled) {
      setProgress(0);
      return;
    }
    const el = document.querySelector("main.main") as HTMLElement | null;
    if (!el) return;

    let mode: "undecided" | "horizontal" | "vertical" = "undecided";
    let startX = 0;
    let startY = 0;
    let startAt = 0;
    let fromEdge = false;

    const reset = () => {
      mode = "undecided";
      setProgress(0);
    };

    const onStart = (e: TouchEvent) => {
      const t = e.touches[0];
      startX = t.clientX;
      startY = t.clientY;
      startAt = Date.now();
      mode = "undecided";
      fromEdge = t.clientX <= cfg.current.edgeWidth;
    };

    const onMove = (e: TouchEvent) => {
      const t = e.touches[0];
      const dx = t.clientX - startX;
      const dy = t.clientY - startY;

      // Lock direction once the finger has moved enough. Vertical intent never
      // gets intercepted, so normal scrolling always works.
      if (mode === "undecided") {
        if (Math.abs(dx) < 8 && Math.abs(dy) < 8) return;
        mode = Math.abs(dx) > Math.abs(dy) ? "horizontal" : "vertical";
      }
      if (mode === "vertical") return;

      let p = 0;
      if (dx > 0 && fromEdge) {
        p = Math.min(dx / cfg.current.maxProgress, 1);
      } else if (dx < 0) {
        p = -Math.min(-dx / cfg.current.maxProgress, 1);
      }
      setProgress(p);

      if (dx < -10) e.preventDefault();
    };

    const onEnd = (e: TouchEvent) => {
      const t = e.changedTouches[0];
      const dx = t.clientX - startX;
      const dy = t.clientY - startY;
      const dt = Date.now() - startAt;
      const wasHorizontal = mode === "horizontal";
      reset();
      if (!wasHorizontal) return;     // vertical scroll: do nothing
      if (dt > 500) return;           // too slow
      if (Math.abs(dy) > Math.abs(dx)) return;
      if (Math.abs(dx) < cfg.current.threshold) return;
      if (dx < 0) cfg.current.onLeft();
      else if (dx > 0 && fromEdge) cfg.current.onRight();
    };

    el.addEventListener("touchstart", onStart, { passive: true });
    el.addEventListener("touchmove", onMove, { passive: false });
    el.addEventListener("touchend", onEnd, { passive: true });
    el.addEventListener("touchcancel", onEnd, { passive: true });

    return () => {
      el.removeEventListener("touchstart", onStart);
      el.removeEventListener("touchmove", onMove);
      el.removeEventListener("touchend", onEnd);
      el.removeEventListener("touchcancel", onEnd);
    };
  }, [enabled]);

  return { progress };
}
