'use client';
import { useEffect, useRef } from 'react';

export function useDismissable(open: boolean, close: () => void) {
  const root = useRef<HTMLElement>(null);
  const closeRef = useRef(close); closeRef.current = close;
  useEffect(() => {
    if (!open) return;
    const origin = document.activeElement as HTMLElement | null;
    const frame = requestAnimationFrame(() => root.current?.querySelector<HTMLElement>('input,textarea,button,[tabindex="0"]')?.focus());
    const key = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {event.preventDefault(); event.stopPropagation(); closeRef.current();}
    };
    document.addEventListener('keydown', key);
    return () => {cancelAnimationFrame(frame); document.removeEventListener('keydown', key); if (origin?.isConnected) origin.focus();};
  }, [open]);
  return root;
}
