'use client';
import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { adapter, command, normalizeError } from './bridge';
import { normalizeSettings } from './protocol';
import { createTimerStore, sameTimerState } from './timer-store';
import type { AppCommand, AppSnapshot, Appearance, Change, RuntimeError, TimerSnapshot } from './protocol';

export function useOrbit() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [timer, setTimer] = useState<TimerSnapshot | null>(null);
  const [appearance, setAppearance] = useState<Appearance>({dark: true, high_contrast: false, reduced_motion: false});
  const [error, setError] = useState<RuntimeError | null>(null);
  const [notice, setNotice] = useState('');
  const [visible, setVisible] = useState(true);
  const [timerStore] = useState(createTimerStore);
  const snapshotRevision = useRef(-1);
  const requiredRevision = useRef(0);
  const inFlight = useRef<Promise<void> | null>(null);
  const publishTimer = useCallback((value: TimerSnapshot) => {
    const previous = timerStore.getSnapshot();
    if (previous && value.revision < previous.revision) return;
    timerStore.update(value);
    setTimer(prev => sameTimerState(prev, value) ? prev : value);
  }, [timerStore]);
  const [pending, setPending] = useState(0);
  const [reportRevision, setReportRevision] = useState(0);
  const mounted = useRef(true);
  const refresh = useCallback(async (minimumRevision = 0): Promise<void> => {
    requiredRevision.current = Math.max(requiredRevision.current, minimumRevision);
    if (minimumRevision > 0 && snapshotRevision.current >= minimumRevision) return;
    if (inFlight.current) return inFlight.current;
    const request = (async () => {
      try {
        do {
          const reply = await command({type: 'snapshot'});
          if (reply.type !== 'snapshot' || !mounted.current) break;
          const value = {...reply.value, settings: normalizeSettings(reply.value.settings)};
          if (value.revision >= snapshotRevision.current) {
            snapshotRevision.current = value.revision;
            setSnapshot(value);
            publishTimer(value.timer);
          }
        } while (snapshotRevision.current < requiredRevision.current && mounted.current);
      } catch (e) {if (mounted.current) setError(normalizeError(e));}
    })();
    inFlight.current = request;
    try {await request;} finally {inFlight.current = null;}
  }, [publishTimer]);
  useEffect(() => {
    mounted.current = true;
    let disposed = false; const cleanups: (() => void)[] = [];
    const register = async <T,>(name: string, handler: (value: T) => void) => {
      const cleanup = await adapter().listen(name, handler);
      if (disposed) cleanup(); else cleanups.push(cleanup);
    };
    (async () => {
      try {
        await register<TimerSnapshot>('orbit://timer', publishTimer);
        await register<Change>('orbit://changed', value => {
          if (value.domains.some(d => d !== 'timer')) void refresh(value.revision);
          if (value.domains.includes('reports')) setReportRevision(value.revision);
        });
        await register<RuntimeError>('orbit://error', setError);
        await register<Appearance>('orbit://appearance', setAppearance);
        await register<boolean>('orbit://visibility', visible => {document.documentElement.dataset.hidden = String(!visible); setVisible(visible); if (visible) void refresh();});
        await register('orbit://resync', () => void refresh());
        await register('orbit://completed', () => {setNotice('Session complete. Take a breath.');});
        const appearance = await adapter().invoke<Appearance>('get_appearance');
        if (!disposed) setAppearance(appearance);
        await refresh();
      } catch (e) {if (!disposed) setError(normalizeError(e));}
    })();
    return () => {disposed = true; mounted.current = false; cleanups.forEach(fn => fn());};
  }, [refresh, publishTimer]);
  useEffect(() => {if (!notice) return; const id = setTimeout(() => setNotice(''), 5000); return () => clearTimeout(id);}, [notice]);
  const run = useCallback(async (action: AppCommand) => {
    setPending(n => n + 1);
    try {const reply = await command(action); setError(null); await refresh(reply.type === 'saved' ? reply.value : 0); return reply;}
    catch (e) {setError(normalizeError(e)); await refresh(); throw e;}
    finally {setPending(n => n - 1);}
  }, [refresh]);
  const act = useCallback((action: AppCommand) => {void run(action).catch(() => {});}, [run]);
  return {snapshot, timer, appearance, error, setError, notice, setNotice, pending, run, act, refresh, reportRevision, timerStore, visible};
}
export type OrbitState = ReturnType<typeof useOrbit>;

/** Only countdown consumers subscribe to one-second display updates. */
export function useLiveTimer(state: OrbitState) {
  return useSyncExternalStore(state.timerStore.subscribe, state.timerStore.getSnapshot, state.timerStore.getSnapshot) ?? state.timer;
}
