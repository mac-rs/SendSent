// iOS-style pull-to-refresh
// 当 main 元素滚动到顶时,下拉触发刷新回调
// 仅用于 mobile touch 设备

import { useEffect, useRef, useState } from "react";
import { RefreshIcon } from "./Icons";
import { haptic } from "../lib/haptic";

export function PullToRefresh({
  onRefresh,
  disabled = false,
}: {
  onRefresh: () => Promise<void> | void;
  disabled?: boolean;
}) {
  const [pull, setPull] = useState(0);
  const [refreshing, setRefreshing] = useState(false);
  const [active, setActive] = useState(false);

  // Keep the latest callback in a ref so the effect never has to re-register
  // listeners mid-gesture (that caused erratic scrolling on iOS).
  const onRefreshRef = useRef(onRefresh);
  onRefreshRef.current = onRefresh;

  useEffect(() => {
    if (disabled) return;
    const el = document.querySelector("main.main") as HTMLElement | null;
    if (!el) return;

    let startY = 0;
    let pulling = false;
    let refreshing = false;
    let curPull = 0;
    let lastHaptic = 0;

    const apply = (v: number) => { curPull = v; setPull(v); };

    const onTouchStart = (e: TouchEvent) => {
      if (el.scrollTop > 0 || refreshing) return;
      startY = e.touches[0].clientY;
      pulling = true;
      curPull = 0;
    };

    const onTouchMove = (e: TouchEvent) => {
      if (!pulling || refreshing) return;
      if (el.scrollTop > 0) { apply(0); setActive(false); return; }
      const dy = e.touches[0].clientY - startY;
      // Upward gesture: hand it back to native scrolling.
      if (dy <= 0) {
        if (curPull !== 0) { apply(0); setActive(false); }
        return;
      }
      const damped = Math.min(dy * 0.4, 100);
      apply(damped);
      const nowActive = damped > 60;
      setActive(nowActive);
      if (nowActive && Date.now() - lastHaptic > 200) {
        lastHaptic = Date.now();
        haptic("medium");
      }
      e.preventDefault();
    };

    const onTouchEnd = async () => {
      if (!pulling) return;
      pulling = false;
      if (curPull > 60 && !refreshing) {
        refreshing = true;
        setRefreshing(true);
        apply(60);
        haptic("light");
        try {
          await onRefreshRef.current();
          haptic("success");
        } catch {
          haptic("error");
        } finally {
          refreshing = false;
          setRefreshing(false);
          apply(0);
          setActive(false);
        }
      } else {
        apply(0);
        setActive(false);
      }
    };

    el.addEventListener("touchstart", onTouchStart, { passive: true });
    el.addEventListener("touchmove", onTouchMove, { passive: false });
    el.addEventListener("touchend", onTouchEnd, { passive: true });
    el.addEventListener("touchcancel", onTouchEnd, { passive: true });

    return () => {
      el.removeEventListener("touchstart", onTouchStart);
      el.removeEventListener("touchmove", onTouchMove);
      el.removeEventListener("touchend", onTouchEnd);
      el.removeEventListener("touchcancel", onTouchEnd);
    };
  }, [disabled]);

  if (disabled) return null;

  const rotation = (pull / 100) * 360;
  const ready = active && !refreshing;

  return (
    <div
      className={`ptr${refreshing ? " refreshing" : ""}${ready ? " ready" : ""}`}
      style={{
        transform: `translateY(${pull - 32}px)`,
        opacity: pull > 5 ? Math.min(pull / 60, 1) : 0,
      }}
      aria-hidden
    >
      <div
        className="ptr-spinner"
        style={{
          transform: refreshing
            ? "none"
            : `rotate(${rotation}deg) scale(${0.6 + Math.min(pull / 100, 1) * 0.4})`,
        }}
      >
        <RefreshIcon size={18} />
      </div>
      <span className="ptr-text">
        {refreshing ? "正在刷新…" : ready ? "松开刷新" : "下拉刷新"}
      </span>
    </div>
  );
}
