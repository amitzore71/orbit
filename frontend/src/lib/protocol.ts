/** Mirrors orbit-runtime/protocol.rs. IDs remain strings across the IPC boundary. */
export type Mode = 'Focus' | 'ShortBreak' | 'LongBreak';
export interface Settings {
  focus_minutes: number; short_minutes: number; long_minutes: number; long_interval: number;
  auto_breaks: boolean; auto_focus: boolean; volume: number; sound: number; notifications: boolean;
  reduced_motion: boolean; mica: boolean; autostart: boolean; appearance: number;
  hotkey_toggle: string; hotkey_show: string; hotkey_skip: string;
}
export interface WidgetPreferences {enabled: boolean; x: number | null; y: number | null; layer: number; click_through: boolean}
export interface TimerSnapshot {
  revision: number; mode: Mode; running: boolean; started: boolean; remaining_ms: number;
  total_ms: number; cycle: number; bound_task: string | null; selected_task: string | null;
}
export interface Task {
  id: string; title: string; estimate: number; completed: number; done: boolean; deleted: boolean;
  planned_for: string | null; today_order: number;
}
export interface AppSnapshot {
  revision: number; timer: TimerSnapshot; tasks: Task[]; settings: Settings; widgets: WidgetPreferences[];
  today: string; today_ms: number; today_completed: number; storage_blocked: boolean;
  data_location: string; last_backup: string | null; recovery_message: string | null;
}
export interface ReportQuery {period: 'day' | 'week' | 'month'; offset: number; start: string; end: string}
export interface Report {
  title: string; active_ms: number; completed: number; partial_ms: number;
  days: {label: string; active_ms: number; completed: number}[];
  tasks: {id: string | null; title: string; deleted: boolean; active_ms: number; completed: number}[];
}
export type TimerAction = 'start' | 'pause' | 'toggle' | 'skip' | 'reset' | 'checkpoint';
export type TaskAction = 'toggle_done' | 'delete' | 'restore' | 'plan' | 'unplan' | 'up' | 'down';
export type AppCommand =
  | {type: 'snapshot'} | {type: 'timer'; action: TimerAction} | {type: 'select_mode'; mode: Mode}
  | {type: 'select_task'; id: string | null} | {type: 'start_task'; id: string}
  | {type: 'create_task'; title: string; estimate: number; today: boolean; start: boolean}
  | {type: 'edit_task'; id: string; title: string; estimate: number}
  | {type: 'task'; id: string; action: TaskAction} | {type: 'save_settings'; settings: Settings}
  | {type: 'widget'; index: number; preferences: WidgetPreferences}
  | {type: 'report'; query: ReportQuery} | {type: 'session_details'; id: string | null; query: ReportQuery}
  | {type: 'retry_saving'};
export type Reply = {type: 'saved'; value: number} | {type: 'snapshot'; value: AppSnapshot}
  | {type: 'report'; value: Report} | {type: 'details'; value: string[]}
  | {type: 'created' | 'file'; value: string};
export interface RuntimeError {message: string; accepted: boolean}
export interface Appearance {dark: boolean; high_contrast: boolean; reduced_motion: boolean; mica_active?: boolean}
export interface Change {revision: number; domains: string[]}
export const defaultQuery: ReportQuery = {period: 'week', offset: 0, start: '', end: ''};
export function duration(ms: number) {
  const min = Math.floor(ms / 60000);
  return min >= 60 ? `${Math.floor(min / 60)}h ${String(min % 60).padStart(2, '0')}m` : ms > 0 && min === 0 ? '<1m' : `${min}m`;
}
export function timeLabel(ms: number) {const seconds = Math.ceil(ms / 1000); return `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;}
export function filterTasks(tasks: Task[], view: string, today: string, search = '') {
  const query = search.toLocaleLowerCase();
  return tasks.filter(t => (view === 'trash' ? t.deleted : view === 'completed' ? t.done && !t.deleted : !t.done && !t.deleted && (view === 'today' ? t.planned_for === today : view === 'carry' ? t.planned_for !== null && t.planned_for < today : true)) && t.title.toLocaleLowerCase().includes(query))
    .sort((a, b) => a.today_order - b.today_order || a.id.localeCompare(b.id, undefined, {numeric: true}));
}
