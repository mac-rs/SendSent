// iOS 风格半屏 sheet(类似 UISheetPresentationController .medium / .large)
// · 顶 drag handle(可下拉关闭)
// · 三档高度:small (28%) / medium (60%) / large (94%)
// · 玻璃背景 + spring 动画
// · 点击 backdrop 关闭

import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

export type SheetSize = "small" | "medium" | "large";

export interface SheetProps {
  open: boolean;
  onClose: () => void;
  /** 默认展开高度 */
  size?: SheetSize;
  /** 是否显示 drag handle(默认 true) */
  handle?: boolean;
  /** 锁定高度(不能拖拽改变) */
  locked?: boolean;
  children: ReactNode;
}

const SIZE_HEIGHT: Record<SheetSize, string> = {
  small: "28vh",
  medium: "60vh",
  large: "94vh",
};

export function Sheet({
  open, onClose, size = "medium", handle = true, locked = false, children,
}: SheetProps) {
  const [dragY, setDragY] = useState(0);
  const [mounted, setMounted] = useState(false);
  const [dragging, setDragging] = useState(false);
  const sheetRef = useRef<HTMLDivElement | null>(null);
  const startY = useRef(0);
  const startHeight = useRef(0);

  useEffect(() => {
    if (open) setMounted(true);
    else {
      const t = window.setTimeout(() => setMounted(false), 360);
      return () => window.clearTimeout(t);
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!mounted) return null;

  const onDragStart = (e: React.PointerEvent) => {
    if (locked) return;
    setDragging(true);
    startY.current = e.clientY;
    startHeight.current = sheetRef.current?.getBoundingClientRect().height ?? 0;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  };
  const onDragMove = (e: React.PointerEvent) => {
    if (!dragging) return;
    const dy = e.clientY - startY.current;
    setDragY(Math.max(0, dy));
  };
  const onDragEnd = () => {
    if (!dragging) return;
    setDragging(false);
    if (dragY > 80) onClose();
    setDragY(0);
  };

  const heightStr = SIZE_HEIGHT[size];
  const translateY = dragging ? `translateY(${dragY}px)` : undefined;
  const opacity = dragging ? Math.max(0, 1 - dragY / 300) : 1;

  return (
    <div
      className={`sheet-backdrop${open ? " open" : ""}`}
      onClick={onClose}
      aria-hidden={!open}
    >
      <div
        ref={sheetRef}
        className={`sheet ${size} glass ${open ? "open" : "closing"} ${dragging ? "dragging" : ""}`}
        style={{
          height: heightStr,
          transform: translateY,
          opacity,
          transition: dragging
            ? "none"
            : "transform 0.42s var(--ease-ios-spring), opacity 0.32s ease",
        }}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        onPointerDown={onDragStart}
        onPointerMove={onDragMove}
        onPointerUp={onDragEnd}
        onPointerCancel={onDragEnd}
      >
        {handle && (
          <div className="sheet-handle">
            <div className="sheet-handle-bar" />
          </div>
        )}
        <div className="sheet-content">
          {children}
        </div>
      </div>
    </div>
  );
}
