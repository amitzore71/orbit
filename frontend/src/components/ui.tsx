'use client';
import type { ButtonHTMLAttributes, ReactNode } from 'react';
import { useDismissable } from '@/lib/use-dismissable';
import { IconClose } from './icons';

export function Button({
  children,
  primary,
  className = '',
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { primary?: boolean }) {
  return (
    <button
      className={`button ${primary ? 'primary' : ''} ${className}`.trim()}
      {...props}
    >
      {children}
    </button>
  );
}

export function IconButton({
  children,
  label,
  className = '',
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { label: string }) {
  return (
    <button
      className={`icon-button ${className}`.trim()}
      aria-label={label}
      title={label}
      {...props}
    >
      {children}
    </button>
  );
}

export function PanelHeading({
  eyebrow,
  title,
  action,
}: {
  eyebrow?: string;
  title: string;
  action?: ReactNode;
}) {
  return (
    <div className="panel-heading">
      <div>
        {eyebrow && <span className="eyebrow">{eyebrow}</span>}
        <h2>{title}</h2>
      </div>
      {action}
    </div>
  );
}

export function Empty({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="empty-state">
      <span className="empty-orbit" aria-hidden="true">
        <i />
        <i />
      </span>
      <h3>{title}</h3>
      <p>{children}</p>
    </div>
  );
}

export function Sheet({
  title,
  children,
  close,
}: {
  title: string;
  children: ReactNode;
  close: () => void;
}) {
  const root = useDismissable(true, close);
  return (
    <>
      <div className="sheet-backdrop" aria-hidden="true" onClick={close} />
      <section
        ref={root}
        role="dialog"
        tabIndex={-1}
        className="sheet"
        aria-label={title}
      >
        <div className="panel-heading">
          <h2>{title}</h2>
          <IconButton label="Close details" onClick={close}>
            <IconClose size={16} />
          </IconButton>
        </div>
        {children}
      </section>
    </>
  );
}

export function Brand({ small = false }: { small?: boolean }) {
  return (
    <span className={`brand ${small ? 'small' : ''}`}>
      <svg width={small ? 20 : 26} height={small ? 20 : 26} viewBox="0 0 32 32" fill="none" aria-hidden="true">
        <circle cx="16" cy="16" r="7.5" stroke="currentColor" strokeWidth="1.75" />
        <ellipse
          cx="16"
          cy="16"
          rx="14.5"
          ry="5.8"
          transform="rotate(-34 16 16)"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeOpacity="0.88"
        />
        <circle cx="26.1" cy="9.2" r="2.6" fill="currentColor" />
      </svg>
      {!small && (
        <span className="brand-word">
          orbit<span className="brand-dot">.</span>
        </span>
      )}
    </span>
  );
}
