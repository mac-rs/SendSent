// iOS 风格 swipe-back 拖影指示器
// 模拟 iOS 系统返回手势:屏幕左缘出现一个白色弧形边缘,
// 跟随手指向右拖出,弧度 progress 越大越明显
// progress = 0..1 表示右滑进度(返回)

export function SwipeIndicator({ progress }: { progress: number }) {
  // 右滑(从左缘)显示左缘拖影
  // 左滑(从屏幕中部)显示右侧 chevron(返回/前进,跟 chrome tab 一样)
  const abs = Math.abs(progress);
  if (abs < 0.01) return null;

  if (progress > 0) {
    // 左缘白色弧形拖影
    return <LeftEdgeShadow progress={progress} />;
  }

  // 左滑(进入下一个 tab)—— 屏幕右缘 chevron
  return <RightEdgeChevron progress={-progress} />;
}

function LeftEdgeShadow({ progress }: { progress: number }) {
  // 1px 宽的竖条,opacity + width 跟 progress 增长
  // 同时显示一个半圆弧度效果
  const width = 8 + progress * 32; // 8~40px
  return (
    <div className="swipe-edge-shadow" aria-hidden>
      <div
        className="swipe-edge-bar"
        style={{
          width: `${width}px`,
          opacity: 0.4 + progress * 0.5,
        }}
      />
      <div
        className="swipe-edge-glow"
        style={{ opacity: progress * 0.6 }}
      />
    </div>
  );
}

function RightEdgeChevron({ progress }: { progress: number }) {
  const x = Math.min(progress * 200, 80);
  return (
    <div
      className="swipe-indicator swipe-indicator-right"
      style={{
        transform: `translateX(calc(-1 * (${x - 32}px))) scale(${0.6 + progress * 0.4})`,
        opacity: progress,
      }}
      aria-hidden
    >
      <div className="swipe-indicator-bg" />
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round">
        <path d="M9 18l6-6-6-6" />
      </svg>
    </div>
  );
}
