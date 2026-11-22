/** Mirrors orbit-runtime/protocol.rs. IDs remain strings across the IPC boundary. */
export type Mode = 'Focus' | 'ShortBreak' | 'LongBreak';
export type Accent = 'coral' | 'sage' | 'amber' | 'slate_blue';
export type Density = 'comfortable' | 'compact';
export type MotionLevel = 'standard' | 'reduced' | 'off';
export type AmbientSound = 'off' | 'brown_noise' | 'rain' | 'cafe' | 'drone';
export interface AmbientPreset {id: AmbientSound; name: string; hint: string}
export const AMBIENT_PRESETS: AmbientPreset[] = [
  {id: 'off', name: 'Off', hint: 'Silent workspace'},
  {id: 'brown_noise', name: 'Brown noise', hint: 'Deep warm focus rumble'},
  {id: 'rain', name: 'Soft rain', hint: 'Gentle steady rainfall'},
  {id: 'cafe', name: 'Warm cafe', hint: 'Cozy low room murmur'},
  {id: 'drone', name: 'Deep drone', hint: 'Calm harmonic hum'},
];
export const SUGGESTED_TAGS = ['Deep Work', 'Client', 'Study', 'Design', 'Admin'];
export interface Settings {
  focus_minutes: number; short_minutes: number; long_minutes: number; long_interval: number;
  daily_goal_sessions?: number;
  auto_breaks: boolean; auto_focus: boolean; volume: number; sound: number;
  ambient_sound?: AmbientSound; ambient_volume?: number; ambient_during_breaks?: boolean;
  notifications: boolean; taskbar_progress?: boolean;
  reduced_motion: boolean; mica: boolean; autostart: boolean; appearance: number;
  hotkey_toggle: string; hotkey_show: string; hotkey_skip: string;
  accent?: Accent; density?: Density; motion_level?: MotionLevel | null;
}
export interface WindowState {maximized: boolean}
export interface ActivityDay {date: string; active_ms: number; completed: number}
export interface ActivityHistory {days: ActivityDay[]; current_streak: number; best_streak: number; goal_days: number}
export interface FocusPreset {id: string; name: string; focus_minutes: number; short_minutes: number; long_minutes: number; hint: string}
export const FOCUS_PRESETS: FocusPreset[] = [
  {id: 'sprint', name: 'Sprint', focus_minutes: 15, short_minutes: 3, long_minutes: 10, hint: 'Quick momentum'},
  {id: 'classic', name: 'Classic', focus_minutes: 25, short_minutes: 5, long_minutes: 15, hint: 'Traditional Pomodoro'},
  {id: 'deep', name: 'Deep Work', focus_minutes: 50, short_minutes: 10, long_minutes: 20, hint: 'Sustained flow'},
  {id: 'ultradian', name: 'Ultradian', focus_minutes: 90, short_minutes: 20, long_minutes: 30, hint: 'Full energy cycle'},
];
export interface WidgetPreferences {enabled: boolean; x: number | null; y: number | null; layer: number; click_through: boolean}
export interface TimerSnapshot {
  revision: number; mode: Mode; running: boolean; started: boolean; remaining_ms: number;
  total_ms: number; cycle: number; bound_task: string | null; selected_task: string | null;
}
export interface Task {
  id: string; title: string; estimate: number; completed: number; done: boolean; deleted: boolean;
  planned_for: string | null; today_order: number;
  start_date?: string | null; end_date?: string | null;
  tag?: string;
}
export interface AppSnapshot {
  revision: number; timer: TimerSnapshot; tasks: Task[]; settings: Settings; widgets: WidgetPreferences[];
  today: string; today_ms: number; today_completed: number; storage_blocked: boolean;
  data_location: string; last_backup: string | null; recovery_message: string | null;
}
export interface ReportQuery {period: 'day' | 'week' | 'month'; offset: number; start: string; end: string}
export interface HourBucket {label: string; active_ms: number}
export interface TagReport {tag: string; active_ms: number; completed: number}
export interface Report {
  title: string; active_ms: number; completed: number; partial_ms: number;
  days: {label: string; active_ms: number; completed: number}[];
  tasks: {id: string | null; title: string; deleted: boolean; active_ms: number; completed: number; done?: boolean; start_date?: string | null; end_date?: string | null; tag?: string}[];
  hours?: HourBucket[];
  tags?: TagReport[];
}
export type TimerAction = 'start' | 'pause' | 'toggle' | 'skip' | 'reset' | 'checkpoint';
export type TaskAction = 'toggle_done' | 'delete' | 'restore' | 'plan' | 'unplan' | 'up' | 'down';
export type AppCommand =
  | {type: 'snapshot'} | {type: 'timer'; action: TimerAction} | {type: 'extend_timer'; minutes: number} | {type: 'select_mode'; mode: Mode}
  | {type: 'select_task'; id: string | null} | {type: 'start_task'; id: string}
  | {type: 'create_task'; title: string; estimate: number; today: boolean; start: boolean; start_date?: string | null; end_date?: string | null; tag?: string}
  | {type: 'edit_task'; id: string; title: string; estimate: number; start_date?: string | null; end_date?: string | null; tag?: string}
  | {type: 'task'; id: string; action: TaskAction} | {type: 'save_settings'; settings: Settings}
  | {type: 'widget'; index: number; preferences: WidgetPreferences}
  | {type: 'report'; query: ReportQuery} | {type: 'activity_history'; days: number} | {type: 'session_details'; id: string | null; query: ReportQuery}
  | {type: 'retry_saving'};
export type Reply = {type: 'saved'; value: number} | {type: 'snapshot'; value: AppSnapshot}
  | {type: 'report'; value: Report} | {type: 'activity_history'; value: ActivityHistory} | {type: 'details'; value: string[]}
  | {type: 'created' | 'file'; value: string};
export interface RuntimeError {message: string; accepted: boolean}
export interface Appearance {
  dark: boolean; high_contrast: boolean; reduced_motion: boolean; mica_active?: boolean;
  accent?: Accent; density?: Density; motion_level?: MotionLevel;
  system_dark?: boolean; system_reduced_motion?: boolean;
}
export function motionPreference(settings: Settings): MotionLevel {
  return settings.motion_level ?? (settings.reduced_motion ? 'reduced' : 'standard');
}
export function normalizeSettings(settings: Settings): Settings {
  const motion_level = motionPreference(settings);
  return {
    ...settings,
    daily_goal_sessions: settings.daily_goal_sessions ?? 4,
    ambient_sound: settings.ambient_sound ?? 'off',
    ambient_volume: settings.ambient_volume ?? 35,
    ambient_during_breaks: settings.ambient_during_breaks ?? false,
    taskbar_progress: settings.taskbar_progress ?? true,
    accent: settings.accent ?? 'coral',
    density: settings.density ?? 'comfortable',
    motion_level,
    reduced_motion: motion_level !== 'standard',
  };
}
export interface Change {revision: number; domains: string[]}
export const defaultQuery: ReportQuery = {period: 'week', offset: 0, start: '', end: ''};
export function duration(ms: number) {
  const min = Math.floor(ms / 60000);
  return min >= 60 ? `${Math.floor(min / 60)}h ${String(min % 60).padStart(2, '0')}m` : ms > 0 && min === 0 ? '<1m' : `${min}m`;
}
export function timeLabel(ms: number) {const seconds = Math.ceil(ms / 1000); return `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;}
export function tagTone(tag: string): number {
  let hash = 0;
  for (let i = 0; i < tag.length; i++) hash = (hash * 31 + tag.charCodeAt(i)) | 0;
  return Math.abs(hash) % 5;
}
export function filterTasks(tasks: Task[], view: string, today: string, search = '', tag = '') {
  const query = search.toLocaleLowerCase();
  return tasks.filter(t =>
    (view === 'trash' ? t.deleted : view === 'completed' ? t.done && !t.deleted : view === 'carry' ? !t.done && !t.deleted && t.planned_for !== null && t.planned_for < today : !t.deleted && (view === 'today' ? t.planned_for === today : true))
    && (!tag || (t.tag ?? '') === tag)
    && (!query || t.title.toLocaleLowerCase().includes(query) || (t.tag ?? '').toLocaleLowerCase().includes(query))
  ).sort((a, b) => a.today_order - b.today_order || a.id.localeCompare(b.id, undefined, {numeric: true}));
}
