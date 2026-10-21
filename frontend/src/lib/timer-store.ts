import type { TimerSnapshot } from './protocol';

export function createTimerStore() {
  let current: TimerSnapshot | null = null;
  const listeners = new Set<() => void>();
  return {
    getSnapshot: () => current,
    subscribe: (listener: () => void) => {listeners.add(listener); return () => {listeners.delete(listener);};},
    update: (next: TimerSnapshot) => {
      if (current && next.revision < current.revision) return;
      // Equal revisions can legitimately carry a newer countdown.
      current = next;
      listeners.forEach(listener => listener());
    },
  };
}
export function sameTimerState(a: TimerSnapshot | null, b: TimerSnapshot) {
  return !!a && a.mode === b.mode && a.running === b.running && a.started === b.started
    && a.total_ms === b.total_ms && a.cycle === b.cycle && a.bound_task === b.bound_task && a.selected_task === b.selected_task;
}
