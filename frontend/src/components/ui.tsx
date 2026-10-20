'use client';
import { motion } from 'motion/react';
import type { ButtonHTMLAttributes, ReactNode } from 'react';
import { X } from '@phosphor-icons/react';
export function Button({children, primary, className = '', ...props}: ButtonHTMLAttributes<HTMLButtonElement> & {primary?: boolean}) {
  return <motion.button className={`button ${primary ? 'primary' : ''} ${className}`} whileTap={{scale: .975}} {...props as object}>{children}</motion.button>;
}
export function IconButton({children, label, ...props}: ButtonHTMLAttributes<HTMLButtonElement> & {label: string}) {
  return <button className="icon-button" aria-label={label} title={label} {...props}>{children}</button>;
}
export function PanelHeading({eyebrow, title, action}: {eyebrow?: string; title: string; action?: ReactNode}) {
  return <div className="panel-heading"><div>{eyebrow && <span className="eyebrow">{eyebrow}</span>}<h2>{title}</h2></div>{action}</div>;
}
export function Empty({title, children}: {title: string; children: ReactNode}) {
  return <div className="empty-state"><span className="empty-orbit" aria-hidden="true"><i /><i /></span><h3>{title}</h3><p>{children}</p></div>;
}
export function Sheet({title, children, close}: {title: string; children: ReactNode; close: () => void}) {
  return <motion.section className="sheet" aria-label={title} initial={{opacity: 0, x: 24}} animate={{opacity: 1, x: 0}} exit={{opacity: 0, x: 24}} transition={{duration: .22}}><div className="panel-heading"><h2>{title}</h2><IconButton label="Close details" onClick={close}><X /></IconButton></div>{children}</motion.section>;
}
export function Brand({small = false}: {small?: boolean}) {
  return <span className={`brand ${small ? 'small' : ''}`}><svg width="30" height="30" viewBox="0 0 32 32" fill="none" aria-hidden="true"><circle cx="16" cy="16" r="8" stroke="currentColor" strokeWidth="1.5" /><ellipse cx="16" cy="16" rx="15" ry="6" transform="rotate(-38 16 16)" stroke="currentColor" strokeWidth="1.5"/><circle cx="25.8" cy="8.2" r="2.5" fill="currentColor"/></svg>{!small && <span>orbit<span className="brand-dot">.</span></span>}</span>;
}
