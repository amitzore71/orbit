'use client';
import { useCallback, useEffect, useRef, useState } from 'react';
import { adapter, command, normalizeError } from './bridge';
import type { AppCommand, AppSnapshot, Appearance, Change, RuntimeError, TimerSnapshot } from './protocol';

export function useOrbit() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [timer, setTimer] = useState<TimerSnapshot | null>(null);
  const [appearance, setAppearance] = useState<Appearance>({dark: true, high_contrast: false, reduced_motion: false});
  const [error, setError] = useState<RuntimeError | null>(null);
  const [notice, setNotice] = useState('');
  const [pending, setPending] = useState(0);
  const [reportRevision, setReportRevision] = useState(0);
  const mounted = useRef(true);
  const loading = useRef(false);
  const reload = useRef(false);
  const refresh = useCallback(async () => {
    if (loading.current) {reload.current = true; return;}
    loading.current = true;
    try {
      do {
        reload.current = false;
        const reply = await command({type: 'snapshot'});
        if (reply.type === 'snapshot' && mounted.current) {
          const value = reply.value;
          setSnapshot(prev => !prev || value.revision >= prev.revision ? value : prev);
          setTimer(prev => !prev || value.timer.revision >= prev.revision ? value.timer : prev);
        }
      } while (reload.current && mounted.current);
    } catch (e) {if (mounted.current) setError(normalizeError(e));}
    finally {loading.current = false;}
  }, []);
  useEffect(() => {
    mounted.current = true;
    let disposed = false; const cleanups: (() => void)[] = [];
    const register = async <T,>(name: string, handler: (value: T) => void) => {
      const cleanup = await adapter().listen(name, handler);
      if (disposed) cleanup(); else cleanups.push(cleanup);
    };
    (async () => {
      try {
        await register<TimerSnapshot>('orbit://timer', value => setTimer(prev => !prev || value.revision >= prev.revision ? value : prev));
        await register<Change>('orbit://changed', value => {
          if (value.domains.some(d => d !== 'timer')) void refresh();
          if (value.domains.includes('reports')) setReportRevision(value.revision);
        });
        await register<RuntimeError>('orbit://error', setError);
        await register<Appearance>('orbit://appearance', setAppearance);
        await register<boolean>('orbit://visibility', visible => {document.documentElement.dataset.hidden = String(!visible);});
        await register('orbit://resync', () => void refresh());
        await register('orbit://completed', () => {setNotice('Session complete. Take a breath.');});
        const appearance = await adapter().invoke<Appearance>('get_appearance');
        if (!disposed) setAppearance(appearance);
        await refresh();
      } catch (e) {if (!disposed) setError(normalizeError(e));}
    })();
    return () => {disposed = true; mounted.current = false; cleanups.forEach(fn => fn());};
  }, [refresh]);
  useEffect(() => {if (!notice) return; const id = setTimeout(() => setNotice(''), 5000); return () => clearTimeout(id);}, [notice]);
  const run = useCallback(async (action: AppCommand) => {
    setPending(n => n + 1);
    try {const reply = await command(action); setError(null); await refresh(); return reply;}
    catch (e) {setError(normalizeError(e)); await refresh(); throw e;}
    finally {setPending(n => n - 1);}
  }, [refresh]);
  const act = useCallback((action: AppCommand) => {void run(action).catch(() => {});}, [run]);
  return {snapshot, timer, appearance, error, setError, notice, setNotice, pending, run, act, refresh, reportRevision};
}
export type OrbitState = ReturnType<typeof useOrbit>;
