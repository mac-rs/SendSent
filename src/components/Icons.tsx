// 统一的 SVG 图标集。24x24 viewBox, 描边宽度 1.75, 圆角端点。
// 通过 currentColor 继承父元素 color,保持一致的视觉重量。

import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement> & { size?: number };

function base({ size = 20, ...rest }: IconProps) {
  return {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.75,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    ...rest,
  };
}

export const DevicesIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <rect x="3" y="5" width="14" height="10" rx="2" />
    <path d="M7 9h6M7 12h4" />
    <rect x="14" y="11" width="7" height="9" rx="1.5" />
    <path d="M16.5 18h2" />
  </svg>
);

export const TransferIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <path d="M4 12h12" />
    <path d="M12 7l5 5-5 5" />
    <path d="M20 4v16" opacity={0.4} />
  </svg>
);

export const SettingsIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" />
  </svg>
);

export const PlusIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <path d="M12 5v14M5 12h14" />
  </svg>
);

export const CheckIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <path d="M4 12l5 5L20 6" />
  </svg>
);

export const XIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <path d="M6 6l12 12M18 6L6 18" />
  </svg>
);

export const SunIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <circle cx="12" cy="12" r="4" />
    <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
  </svg>
);

export const MoonIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" />
  </svg>
);

export const FileIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" />
    <path d="M14 3v5h5" />
  </svg>
);

export const PhoneIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <rect x="7" y="2" width="10" height="20" rx="2.5" />
    <path d="M11 18h2" />
  </svg>
);

export const ShieldIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z" />
    <path d="M9 12l2 2 4-4" />
  </svg>
);

export const HashIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <path d="M5 9h14M5 15h14M10 3l-2 18M16 3l-2 18" />
  </svg>
);

export const SendIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <path d="M3 11.5L21 4l-7.5 18-2.5-7.5z" />
    <path d="M11 14.5L21 4" />
  </svg>
);

export const WifiIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <path d="M2 9a16 16 0 0 1 20 0" opacity={0.4} />
    <path d="M5 13a11 11 0 0 1 14 0" opacity={0.7} />
    <path d="M8.5 16.5a6 6 0 0 1 7 0" />
    <circle cx="12" cy="20" r="0.5" fill="currentColor" />
  </svg>
);

export const ChevronRightIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <path d="M9 6l6 6-6 6" />
  </svg>
);

export const ChevronDownIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <path d="M6 9l6 6 6-6" />
  </svg>
);

export const CopyIcon = (p: IconProps) => (
  <svg {...base(p)}>
    <rect x="8" y="8" width="12" height="12" rx="2" />
    <path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2" />
  </svg>
);

export const LogoIcon = (p: IconProps) => (
  <svg {...base({ ...p, strokeWidth: 1.5 })}>
    <path d="M4 12c0-4.4 3.6-8 8-8s8 3.6 8 8" />
    <path d="M4 12c0 4.4 3.6 8 8 8s8-3.6 8-8" />
    <path d="M8 12h8" />
    <circle cx="4" cy="12" r="1.4" fill="currentColor" stroke="none" />
    <circle cx="20" cy="12" r="1.4" fill="currentColor" stroke="none" />
  </svg>
);
