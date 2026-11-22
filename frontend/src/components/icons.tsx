import type { SVGProps } from 'react';

type IconProps = SVGProps<SVGSVGElement> & { size?: number; filled?: boolean };

function Svg({ size = 18, children, ...props }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.65"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      {...props}
    >
      {children}
    </svg>
  );
}

export function IconFocus({ size = 18, filled, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <circle cx="12" cy="12" r="8.5" strokeDasharray={filled ? undefined : '3.5 3.5'} />
      <circle cx="12" cy="12" r="2.75" fill={filled ? 'currentColor' : 'none'} />
    </Svg>
  );
}

export function IconTasks({ size = 18, filled, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M9.5 6.5h10M9.5 12h10M9.5 17.5h10" />
      <path d="m4 6.5 1.3 1.3L7.5 5.2" strokeWidth={filled ? '2.1' : '1.7'} />
      <path d="m4 12 1.3 1.3 2.2-2.6" strokeWidth={filled ? '2.1' : '1.7'} />
      <circle cx="5.5" cy="17.5" r="1.1" fill="currentColor" stroke="none" />
    </Svg>
  );
}

export function IconInsights({ size = 18, filled, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <rect x="4" y="13" width="3.5" height="7" rx="1" fill={filled ? 'currentColor' : 'none'} />
      <rect x="10.25" y="8.5" width="3.5" height="11.5" rx="1" fill={filled ? 'currentColor' : 'none'} />
      <rect x="16.5" y="4" width="3.5" height="16" rx="1" fill={filled ? 'currentColor' : 'none'} />
    </Svg>
  );
}

export function IconSettings({ size = 18, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M4 7h7m5 0h4M4 17h4m5 0h7" />
      <circle cx="13.5" cy="7" r="2.5" />
      <circle cx="10.5" cy="17" r="2.5" />
    </Svg>
  );
}

export function IconPlay({ size = 16, ...props }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" {...props}>
      <path d="M7.5 5.2a1.2 1.2 0 0 1 1.82-1.03l10.2 6.8a1.2 1.2 0 0 1 0 2.06l-10.2 6.8A1.2 1.2 0 0 1 7.5 18.8V5.2Z" />
    </svg>
  );
}

export function IconPause({ size = 16, ...props }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" {...props}>
      <rect x="6" y="4.5" width="4.4" height="15" rx="1.4" />
      <rect x="13.6" y="4.5" width="4.4" height="15" rx="1.4" />
    </svg>
  );
}

export function IconReset({ size = 18, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M4.5 10a7.5 7.5 0 1 1 1.7 7.1" />
      <path d="M4.5 4.8V10h5.2" />
    </Svg>
  );
}

export function IconSkip({ size = 18, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M5.5 6.2 14 12l-8.5 5.8V6.2Z" fill="currentColor" stroke="none" />
      <path d="M17.5 6v12" strokeWidth="1.9" />
    </Svg>
  );
}

export function IconPlus({ size = 18, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M12 5v14M5 12h14" />
    </Svg>
  );
}

export function IconMinus({ size = 15, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M5 12h14" />
    </Svg>
  );
}

export function IconMaximize({ size = 14, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <rect x="5" y="5" width="14" height="14" rx="1.8" />
    </Svg>
  );
}

export function IconRestore({ size = 14, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <rect x="4.5" y="7.5" width="11.5" height="11.5" rx="1.6" />
      <path d="M8.5 7.5V6a1.5 1.5 0 0 1 1.5-1.5h8A1.5 1.5 0 0 1 19.5 6v8a1.5 1.5 0 0 1-1.5 1.5H16" />
    </Svg>
  );
}

export function IconExpand({ size = 15, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M9 4.5H4.5V9M15 4.5h4.5V9M9 19.5H4.5V15M15 19.5h4.5V15" />
    </Svg>
  );
}

export function IconCompress({ size = 15, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M4.5 9H9V4.5M19.5 9H15V4.5M4.5 15H9v4.5M19.5 15H15v4.5" />
    </Svg>
  );
}

export function IconFlame({ size = 15, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M12.3 3.5c.8 3.2 4.2 4.9 4.2 9.1a4.5 4.5 0 1 1-9 0c0-2.3 1.1-4.1 2.4-5.5.2 1.6 1 2.4 1.9 2.6-.7-2-.4-4.2.5-6.2Z" />
    </Svg>
  );
}

export function IconWaves({ size = 15, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M3.5 10.5v3M7.5 7v10M12 4.5v15M16.5 8v8M20.5 10.5v3" />
    </Svg>
  );
}

export function IconTag({ size = 14, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M12.5 4H6a2 2 0 0 0-2 2v6.5a2 2 0 0 0 .6 1.4l6.5 6.5a2 2 0 0 0 2.8 0l5.5-5.5a2 2 0 0 0 0-2.8l-6.5-6.5A2 2 0 0 0 12.5 4Z" />
      <circle cx="8.5" cy="8.5" r="1.1" fill="currentColor" stroke="none" />
    </Svg>
  );
}

export function IconClock({ size = 15, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 7.5V12l3 1.8" />
    </Svg>
  );
}

export function IconClose({ size = 15, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M6.5 6.5 17.5 17.5M17.5 6.5l-11 11" />
    </Svg>
  );
}

export function IconCheck({ size = 14, ...props }: IconProps) {
  return (
    <Svg size={size} strokeWidth="2.1" {...props}>
      <path d="m5 12.5 4.5 4.5 9.5-10" />
    </Svg>
  );
}

export function IconChevronDown({ size = 14, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="m6.5 9.5 5.5 5.5 5.5-5.5" />
    </Svg>
  );
}

export function IconArrowRight({ size = 15, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M5 12h14M13 6l6 6-6 6" />
    </Svg>
  );
}

export function IconArrowLeft({ size = 15, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M19 12H5M11 6l-6 6 6 6" />
    </Svg>
  );
}

export function IconArrowUp({ size = 15, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M12 19V5M6 11l6-6 6 6" />
    </Svg>
  );
}

export function IconArrowDown({ size = 15, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M12 5v14M6 13l6 6 6-6" />
    </Svg>
  );
}

export function IconArrowUpRight({ size = 14, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M7 17 17 7M8.5 7H17v8.5" />
    </Svg>
  );
}

export function IconSearch({ size = 15, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <circle cx="11" cy="11" r="6.5" />
      <path d="m16 16 4 4" />
    </Svg>
  );
}

export function IconDots({ size = 18, ...props }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" {...props}>
      <circle cx="6.5" cy="12" r="1.5" />
      <circle cx="12" cy="12" r="1.5" />
      <circle cx="17.5" cy="12" r="1.5" />
    </svg>
  );
}

export function IconCalendar({ size = 15, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <rect x="4" y="5.5" width="16" height="14.5" rx="2.5" />
      <path d="M8 3.5v3.5M16 3.5v3.5M4 10h16" />
    </Svg>
  );
}

export function IconEdit({ size = 15, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="m15.2 5.3 3.5 3.5-9.8 9.8H5.4v-3.5l9.8-9.8Z" />
    </Svg>
  );
}

export function IconTrash({ size = 15, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M4.5 7h15M9.5 7V5a1.5 1.5 0 0 1 1.5-1.5h2A1.5 1.5 0 0 1 14.5 5v2M7 7l.8 12a1.5 1.5 0 0 0 1.5 1.5h5.4a1.5 1.5 0 0 0 1.5-1.5L17 7" />
    </Svg>
  );
}

export function IconLock({ size = 13, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <rect x="5" y="10.5" width="14" height="9.5" rx="2" />
      <path d="M8.5 10.5V7.5a3.5 3.5 0 0 1 7 0v3" />
    </Svg>
  );
}

export function IconAlert({ size = 18, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 8v5" />
      <circle cx="12" cy="16.2" r=".9" fill="currentColor" stroke="none" />
    </Svg>
  );
}

export function IconCheckCircle({ size = 18, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <circle cx="12" cy="12" r="9" />
      <path d="m8.5 12.2 2.4 2.4 4.8-5.2" />
    </Svg>
  );
}

export function IconSpeaker({ size = 16, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M5 9.5h3.5L13 5.5v13l-4.5-4H5v-5Z" />
      <path d="M16.5 9a4.5 4.5 0 0 1 0 6M19 6.8a7.8 7.8 0 0 1 0 10.4" />
    </Svg>
  );
}

export function IconKeyboard({ size = 16, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <rect x="3" y="6" width="18" height="12" rx="2.5" />
      <path d="M7 10h.01M10.5 10h.01M14 10h.01M17 10h.01M8 14h8" />
    </Svg>
  );
}

export function IconFolder({ size = 16, ...props }: IconProps) {
  return (
    <Svg size={size} {...props}>
      <path d="M3.5 7.5a2 2 0 0 1 2-2h4.2l2 2.3H18.5a2 2 0 0 1 2 2v7.2a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2v-9.5Z" />
    </Svg>
  );
}
