// @vitest-environment jsdom
import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { useEffect } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { AppSnapshot } from '../src/lib/protocol';

const mocks = vi.hoisted(() => ({handlers: new Map<string, (value: any) => void>(), invoke: vi.fn(), command: vi.fn()}));
vi.mock('../src/lib/bridge', () => ({adapter: () => ({invoke: mocks.invoke, listen: async (name: string, handler: (value: any) => void) => {mocks.handlers.set(name, handler); return () => mocks.handlers.delete(name);}}), command: mocks.command, normalizeError: (e: unknown) => ({message: String(e), accepted: false})}));
import { useOrbit, useLiveTimer } from '../src/lib/use-orbit';

function fixture(): AppSnapshot {
  return {revision: 1, timer: {revision: 1, mode: 'Focus', running: true, started: true, remaining_ms: 1500000, total_ms: 1500000, cycle: 0, bound_task: null, selected_task: null},
    settings: {focus_minutes: 25, short_minutes: 5, long_minutes: 15, long_interval: 4, auto_breaks: false, auto_focus: false, volume: 65, sound: 0, notifications: true, reduced_motion: false, mica: false, autostart: false, appearance: 0, hotkey_toggle: '', hotkey_show: '', hotkey_skip: ''},
    tasks: [], widgets: [], today: '2026-10-21', today_ms: 0, today_completed: 0, storage_blocked: false, data_location: '', last_backup: null, recovery_message: null};
}
afterEach(() => {cleanup(); mocks.handlers.clear(); vi.clearAllMocks();});
describe('live timer subscriptions', () => {
  it('updates countdown consumers without committing the workspace on ticks; rejects old revisions', async () => {
    const data = fixture(); let workspaceCommits = 0;
    mocks.invoke.mockResolvedValue({dark: true, reduced_motion: false, high_contrast: false});
    mocks.command.mockResolvedValue({type: 'snapshot', value: data});
    const orbit = renderHook(() => {const state = useOrbit(); useEffect(() => {workspaceCommits++;}); return state;});
    await waitFor(() => expect(orbit.result.current.snapshot).not.toBeNull());
    const countdown = renderHook(() => useLiveTimer(orbit.result.current));
    const baseline = workspaceCommits;
    act(() => {for (let i = 1; i <= 5; i++) mocks.handlers.get('orbit://timer')!({...data.timer, remaining_ms: 1500000 - i * 1000});});
    expect(countdown.result.current?.remaining_ms).toBe(1495000);
    expect(workspaceCommits).toBe(baseline);
    act(() => mocks.handlers.get('orbit://timer')!({...data.timer, revision: 0, remaining_ms: 1}));
    expect(countdown.result.current?.remaining_ms).toBe(1495000);
    act(() => mocks.handlers.get('orbit://timer')!({...data.timer, revision: 2, running: false}));
    expect(orbit.result.current.timer?.running).toBe(false);
  });
  it('coalesces a mutation acknowledgement and its changed event into one snapshot', async () => {
    const data = fixture();
    mocks.invoke.mockResolvedValue({dark: true, reduced_motion: false, high_contrast: false});
    mocks.command.mockImplementation(async (action: {type: string}) => {
      if (action.type === 'snapshot') return {type: 'snapshot', value: structuredClone(data)};
      data.revision++;
      mocks.handlers.get('orbit://changed')!({revision: data.revision, domains: ['settings']});
      return {type: 'saved', value: data.revision};
    });
    const orbit = renderHook(() => useOrbit());
    await waitFor(() => expect(orbit.result.current.snapshot).not.toBeNull());
    mocks.command.mockClear();
    await act(async () => {await orbit.result.current.run({type: 'retry_saving'});});
    expect(mocks.command.mock.calls.filter(([command]) => command.type === 'snapshot')).toHaveLength(1);
    expect(orbit.result.current.snapshot?.revision).toBe(2);
  });
});
