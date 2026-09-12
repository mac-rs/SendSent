// iOS NavigationStack 大标题滚动跟踪
// · 用 rAF 平滑插值 scrollTop → collapse(0~1),手感比直接 setState 跟手
// · scroll 期间禁用 transition(直接跟手),停止后恢复 spring(overshoot 收回)
// · 返回 [collapse, scrollTopRef] 给组件用

import { useEffect, useRef, useState } from "react";

const COLLAPSE_START = 28;
const COLLAPSE_END = 112;

export function useScrollCollapse(scopeSelector = ".content"): {
  collapse: number;
  scrollTop: number;
  scrolling: boolean;
} {
  const [collapse, setCollapse] = useState(0);
  const [scrollTop, setScrollTop] = useState(0);
  const [scrolling, setScrolling] = useState(false);
  const targetRef = useRef(0);
  const currentRef = useRef(0);
  const rafRef = useRef<number | null>(null);
  const stopTimerRef = useRef<number | null>(null);

  useEffect(() => {
    const el = document.querySelector(scopeSelector) as HTMLElement | null;
    if (!el) return;

    const onScroll = () => {
      const y = el.scrollTop;
      targetRef.current = y;
      setScrollTop(y);
      setScrolling(true);
      if (stopTimerRef.current) window.clearTimeout(stopTimerRef.current);
      stopTimerRef.current = window.setTimeout(() => setScrolling(false), 140);
    };

    el.addEventListener("scroll", onScroll, { passive: true });

    const tick = () => {
      const target = targetRef.current;
      const cur = currentRef.current;
      // scroll 中:直接跟手(disable spring);停止后:用 spring 拉回
      const next = scrolling
        ? target
        : cur + (target - cur) * 0.18;
      currentRef.current = next;
      const ratio = Math.min(
        1,
        Math.max(0, (next - COLLAPSE_START) / (COLLAPSE_END - COLLAPSE_START)),
      );
      setCollapse(ratio);
      rafRef.current = requestAnimationFrame(tick);
    };
    rafRef.current = requestAnimationFrame(tick);

    return () => {
      el.removeEventListener("scroll", onScroll);
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
      if (stopTimerRef.current) window.clearTimeout(stopTimerRef.current);
    };
  }, [scopeSelector, scrolling]);

  return { collapse, scrollTop, scrolling };
}
