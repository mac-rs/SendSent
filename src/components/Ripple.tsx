// Material 3 ripple 效果包装器
// 点击时在 pointer 位置插入一个圆形 ripple 元素,scale + opacity 动画
// 完全不依赖 React Portal / Material 3 lib

import { useCallback, type CSSProperties, type ReactNode } from "react";

interface RippleProps {
  children: ReactNode;
  /** 自定义 ripple 颜色(默认 currentColor) */
  color?: string;
  className?: string;
  style?: CSSProperties;
  onClick?: (e: React.MouseEvent) => void;
  disabled?: boolean;
}

interface RippleData {
  x: number;
  y: number;
  size: number;
  key: number;
}

import { useState } from "react";

export function Ripple({ children, color, className, style, onClick, disabled }: RippleProps) {
  const [ripples, setRipples] = useState<RippleData[]>([]);

  const handle = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      onClick?.(e);
      if (disabled) return;
      const rect = e.currentTarget.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;
      const size = Math.max(rect.width, rect.height) * 2;
      const key = Date.now() + Math.random();
      setRipples((rs) => [...rs, { x, y, size, key }]);
      window.setTimeout(() => {
        setRipples((rs) => rs.filter((r) => r.key !== key));
      }, 600);
    },
    [onClick, disabled],
  );

  return (
    <div
      className={`ripple-host${className ? ` ${className}` : ""}`}
      style={{ position: "relative", overflow: "hidden", ...style }}
      onClick={handle}
    >
      {children}
      {ripples.map((r) => (
        <span
          key={r.key}
          className="ripple"
          style={{
            position: "absolute",
            borderRadius: "50%",
            background: color ?? "var(--ripple, currentColor)",
            opacity: 0.32,
            transform: "scale(0)",
            left: r.x - r.size / 2,
            top: r.y - r.size / 2,
            width: r.size,
            height: r.size,
            animation: "rippleEffect 0.6s var(--ease-standard) forwards",
            pointerEvents: "none",
          }}
        />
      ))}
    </div>
  );
}
