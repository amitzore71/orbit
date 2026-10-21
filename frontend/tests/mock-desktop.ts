import type { Page } from '@playwright/test';
export async function mockDesktop(page: Page, options: {light?: boolean; reduced?: boolean; contrast?: boolean; many?: boolean; blocked?: boolean} = {}) {
  await page.addInitScript((options) => {
    const handlers = new Map<string, Set<(v: unknown) => void>>();
    const emit = (name: string, value: unknown) => handlers.get(name)?.forEach(fn => fn(value));
    const data = {
      revision: 1, timer: {revision: 1, mode: 'Focus', running: false, started: false, remaining_ms: 1500000, total_ms: 1500000, cycle: 1, bound_task: null as string | null, selected_task: '1' as string | null},
      tasks: [
        {id: '1', title: 'Shape the next chapter of Orbit', estimate: 3, completed: 1, done: false, deleted: false, planned_for: '2026-10-20' as string | null, today_order: 1},
        {id: '2', title: 'Read a few pages, take a few notes', estimate: 2, completed: 0, done: false, deleted: false, planned_for: '2026-10-20', today_order: 2},
        {id: '3', title: 'Make space for a new idea', estimate: 1, completed: 0, done: false, deleted: false, planned_for: '2026-10-20', today_order: 3},
        {id: '4', title: 'Revisit the research notes', estimate: 2, completed: 0, done: false, deleted: false, planned_for: '2026-10-19', today_order: 4},
      ],
      settings: {focus_minutes: 25, short_minutes: 5, long_minutes: 15, long_interval: 4, auto_breaks: false, auto_focus: false, volume: 65, sound: 0, notifications: true, reduced_motion: false, mica: false, autostart: false, appearance: 0, hotkey_toggle: 'Ctrl+Alt+P', hotkey_show: 'Ctrl+Alt+O', hotkey_skip: ''},
      widgets: Array.from({length: 3}, () => ({enabled: false, x: null, y: null, layer: 1, click_through: false})),
      today: '2026-10-20', today_ms: 5040000, today_completed: 3, storage_blocked: !!options.blocked,
      data_location: 'C:\\Users\\You\\AppData\\Local\\Orbit\\orbit.db', last_backup: null, recovery_message: null,
    };
    if (options.many) data.tasks = Array.from({length: 5000}, (_, i) => ({...data.tasks[0], id: String(i + 1), title: `Research task ${i + 1}`, today_order: i}));
    const changed = (domains: string[]) => {data.revision++; data.timer.revision = data.revision; emit('orbit://changed', {revision: data.revision, domains}); emit('orbit://timer', structuredClone(data.timer));};
    const report = {title: '19 Oct 2026 — 25 Oct 2026', active_ms: 15_180_000, completed: 9, partial_ms: 1_680_000, days: [{label: 'Mon', active_ms: 6_300_000, completed: 4}, {label: 'Tue', active_ms: 5_040_000, completed: 3}, {label: 'Wed', active_ms: 3_840_000, completed: 2}, ...['Thu', 'Fri', 'Sat', 'Sun'].map(label => ({label, active_ms: 0, completed: 0}))], tasks: [{id: '1', title: data.tasks[0].title, active_ms: 10_800_000, completed: 7, deleted: false}, {id: null, title: 'Without a task', active_ms: 4_380_000, completed: 2, deleted: false}]};
    const testWindow = window as unknown as {__ORBIT_TEST__: unknown; __ORBIT_CALLS__: unknown[]; __ORBIT_LATENCY__: number; __ORBIT_EMIT__: typeof emit; __ORBIT_SAVE_ERROR__?: 'rejected' | 'accepted' | null};
    testWindow.__ORBIT_CALLS__ = []; testWindow.__ORBIT_LATENCY__ = 0; testWindow.__ORBIT_EMIT__ = emit;
    testWindow.__ORBIT_TEST__ = {
      listen: async (name: string, fn: (v: unknown) => void) => {if (!handlers.has(name)) handlers.set(name, new Set()); const subscription=(value: unknown)=>fn(value); handlers.get(name)!.add(subscription); return () => handlers.get(name)!.delete(subscription);},
      invoke: async (name: string, args: Record<string, any> = {}) => {
        testWindow.__ORBIT_CALLS__.push({name, args});
        if (name === 'get_appearance') return {dark: !options.light, reduced_motion: !!options.reduced, high_contrast: !!options.contrast};
        if (name === 'file_operation') return null;
        if (name !== 'orbit_command') return;
        const c = args.command;
        if (c.type === 'snapshot') return {type: 'snapshot', value: structuredClone(data)};
        if (c.type === 'report') {if (c.query.start && c.query.end < c.query.start) throw {message: 'Choose a valid date range.', accepted: false}; return {type: 'report', value: report};}
        if (c.type === 'session_details') return {type: 'details', value: ['20 Oct · 09:30 · 25m · Complete', '20 Oct · 10:15 · 18m · Partial']};
        if (c.type === 'create_task') {const id = String(data.tasks.length + 1); data.tasks.push({id, title: c.title, estimate: c.estimate, completed: 0, done: false, deleted: false, planned_for: c.today ? data.today : null, today_order: data.tasks.length + 1}); changed(['tasks']); return {type: 'created', value: id};}
        if (c.type === 'timer') {if (c.action === 'toggle') {data.timer.running = !data.timer.running; data.timer.started = true; data.timer.bound_task ||= data.timer.selected_task;} if (c.action === 'reset') {data.timer.running = false; data.timer.started = false; data.timer.remaining_ms = data.timer.total_ms;} changed(['timer']);}
        if (c.type === 'start_task') {data.timer.selected_task = c.id; if (!data.timer.started) {data.timer.bound_task = c.id; data.timer.running = true; data.timer.started = true;} changed(['timer']);}
        if (c.type === 'select_task') {data.timer.selected_task = c.id; changed(['timer']);}
        if (c.type === 'select_mode') {data.timer.mode = c.mode; data.timer.running = false; data.timer.started = false; data.timer.total_ms = data.timer.remaining_ms = c.mode === 'Focus' ? 1500000 : c.mode === 'ShortBreak' ? 300000 : 900000; changed(['timer']);}
        if (c.type === 'task') {const task = data.tasks.find(t => t.id === c.id)!; if (c.action === 'delete') task.deleted = true; if (c.action === 'restore') {task.deleted = false; task.planned_for = null;} if (c.action === 'toggle_done') task.done = !task.done; if (c.action === 'plan') task.planned_for = data.today; if (c.action === 'unplan') task.planned_for = null; changed(['tasks']);}
        if (c.type === 'edit_task') {Object.assign(data.tasks.find(t => t.id === c.id)!, {title: c.title, estimate: c.estimate}); changed(['tasks']);}
        if (c.type === 'save_settings') {
          const submitted = structuredClone(c.settings);
          await new Promise(r => setTimeout(r, testWindow.__ORBIT_LATENCY__));
          if (testWindow.__ORBIT_SAVE_ERROR__ === 'rejected') throw {message: 'Settings could not be saved. Try again.', accepted: false};
          data.settings = submitted;
          if (testWindow.__ORBIT_SAVE_ERROR__ === 'accepted') {data.storage_blocked = true; changed(['settings', 'storage']); throw {message: 'Your settings are waiting to be saved.', accepted: true};}
          changed(['settings']);
        }
        if (c.type === 'widget') {data.widgets[c.index] = c.preferences; changed(['widgets']);}
        if (c.type === 'retry_saving') {data.storage_blocked = false; changed(['storage']);}
        return {type: 'saved', value: data.revision};
      },
    };
  }, options);
}
