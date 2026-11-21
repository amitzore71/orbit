'use client';
import { memo, useState } from 'react';
import type { OrbitState } from '@/lib/use-orbit';
import type { Mode, Task } from '@/lib/protocol';
import { useLiveTimer } from '@/lib/use-orbit';
import { useExperience } from '@/lib/experience';
import { useDismissable } from '@/lib/use-dismissable';
import { timeLabel } from '@/lib/protocol';
import {
  IconArrowUpRight,
  IconCheck,
  IconChevronDown,
  IconPause,
  IconPlay,
  IconReset,
  IconSkip,
} from './icons';
import { Button, IconButton } from './ui';

const modes: [Mode, string][] = [
  ['Focus', 'Focus'],
  ['ShortBreak', 'Short break'],
  ['LongBreak', 'Long break'],
];

const TICKS = Array.from({ length: 60 }, (_, i) => ({
  index: i,
  cardinal: i % 15 === 0,
  major: i % 5 === 0,
  angle: i * 6,
}));

/** Static machined dial bezel — isolated in its own SVG layer so 1-second arc ticks never re-rasterize 65 shapes. */
const DialFrame = memo(function DialFrame() {
  return (
    <svg className="orbit-dial orbit-dial-bg" viewBox="0 0 400 400" aria-hidden="true">
      <defs>
        <radialGradient id="orbit-well-grad" cx="50%" cy="35%" r="65%">
          <stop offset="0%" stopColor="var(--dial-well-hi)" />
          <stop offset="100%" stopColor="var(--dial-well-lo)" />
        </radialGradient>
      </defs>
      <circle className="dial-well" cx="200" cy="200" r="186" fill="url(#orbit-well-grad)" />
      <circle className="dial-outer" cx="200" cy="200" r="186" />
      <circle className="dial-bezel" cx="200" cy="200" r="177" />
      {TICKS.map(t => (
        <line
          key={t.index}
          x1="200"
          y1={t.cardinal ? 23 : t.major ? 25 : 28}
          x2="200"
          y2={t.cardinal ? 36 : t.major ? 34 : 32}
          transform={`rotate(${t.angle} 200 200)`}
          className={t.cardinal ? 'dial-tick cardinal' : t.major ? 'dial-tick major' : 'dial-tick'}
        />
      ))}
      <circle className="dial-inner" cx="200" cy="200" r="144" />
      <circle className="dial-track" cx="200" cy="200" r="158" />
    </svg>
  );
});

export function TimerRing({ state, mini = false }: { state: OrbitState; mini?: boolean }) {
  const timer = useLiveTimer(state)!;
  const { motion: motionLevel, visible } = useExperience();
  const smoothTick = motionLevel === 'standard' && visible && timer.running;
  const progress = Math.max(0, Math.min(1, timer.remaining_ms / Math.max(timer.total_ms, 1)));
  const [minutes, seconds] = timeLabel(timer.remaining_ms).split(':');
  const modeLabel =
    timer.mode === 'Focus'
      ? 'Focus session'
      : timer.mode === 'ShortBreak'
        ? 'Short break'
        : 'Long break';
  const statusLabel = state.snapshot?.storage_blocked
    ? 'Saving paused'
    : timer.running
      ? 'In flow'
      : timer.started
        ? 'Paused'
        : 'Ready';

  return (
    <div
      className={`timer-object ${timer.running && visible ? 'is-running' : ''} ${mini ? 'widget-ring' : ''} ${state.notice.startsWith('Session complete') ? 'session-complete' : ''}`}
      data-mode={timer.mode}
    >
      <div className="timer-aura" aria-hidden="true" />
      <DialFrame />
      <svg className="orbit-dial orbit-dial-arc" viewBox="0 0 400 400" aria-hidden="true">
        <defs>
          <linearGradient id="orbit-arc-grad" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor="var(--accent-soft)" />
            <stop offset="100%" stopColor="var(--accent)" />
          </linearGradient>
        </defs>
        <circle
          className="dial-progress"
          cx="200"
          cy="200"
          r="158"
          stroke="url(#orbit-arc-grad)"
          pathLength="1"
          strokeDasharray="1"
          strokeDashoffset={1 - progress}
          transform="rotate(-90 200 200)"
        />
      </svg>
      <span
        className={`progress-orbit ${smoothTick ? 'is-ticking' : ''}`}
        aria-hidden="true"
        style={{ transform: `rotate(${progress * 360}deg)` }}
      >
        <i className="progress-dot" />
      </span>
      <div className="timer-face">
        <span className="eyebrow">{modeLabel}</span>
        <div
          className={`time-display ${minutes.length > 2 ? 'long-time' : ''}`}
          role="timer"
          aria-label={`${minutes} minutes ${seconds} seconds remaining`}
        >
          <span>{minutes}</span>
          <span className="time-colon">:</span>
          <span>{seconds}</span>
        </div>
        <span className="timer-state">
          <i className={timer.running ? 'live-dot' : ''} />
          {statusLabel}
        </span>
      </div>
    </div>
  );
}

export function TimerControls({ state, small = false }: { state: OrbitState; small?: boolean }) {
  const timer = state.timer!;
  const label = timer.running
    ? 'Pause session'
    : timer.started
      ? 'Resume session'
      : timer.mode === 'Focus'
        ? 'Start focusing'
        : 'Start break';
  return (
    <div className={`timer-controls ${small ? 'small-controls' : ''}`}>
      <IconButton
        label="Reset session"
        disabled={state.snapshot?.storage_blocked || !!state.pending}
        onClick={() => state.act({ type: 'timer', action: 'reset' })}
      >
        <IconReset size={18} />
      </IconButton>
      <Button
        primary
        aria-label={label}
        disabled={state.snapshot?.storage_blocked || !!state.pending}
        onClick={() => state.act({ type: 'timer', action: 'toggle' })}
      >
        {timer.running ? <IconPause size={15} /> : <IconPlay size={15} />}
        <span>{label}</span>
        {!small && state.snapshot?.settings.hotkey_toggle && (
          <kbd>{state.snapshot.settings.hotkey_toggle}</kbd>
        )}
      </Button>
      <IconButton
        label="Skip session"
        disabled={state.snapshot?.storage_blocked || !!state.pending}
        onClick={() => state.act({ type: 'timer', action: 'skip' })}
      >
        <IconSkip size={18} />
      </IconButton>
    </div>
  );
}

export function FocusTimer({ state }: { state: OrbitState }) {
  const { snapshot: data, timer } = state;
  const [picker, setPicker] = useState(false);
  const [search, setSearch] = useState('');
  const pickerRoot = useDismissable(picker, () => setPicker(false));
  if (!data || !timer) return null;

  const currentId = timer.started ? timer.bound_task : timer.selected_task;
  const task = data.tasks.find(t => t.id === currentId);
  const next =
    timer.started && timer.selected_task !== timer.bound_task
      ? data.tasks.find(t => t.id === timer.selected_task)
      : undefined;
  const tasks = data.tasks.filter(
    t => !t.done && !t.deleted && t.title.toLocaleLowerCase().includes(search.toLocaleLowerCase()),
  );
  const cycle = timer.cycle % data.settings.long_interval;
  const completedInCycle =
    timer.mode === 'LongBreak' && timer.cycle > 0 ? data.settings.long_interval : cycle;

  return (
    <section className="focus-stage" aria-label="Focus timer">
      <div className="mode-tabs" role="group" aria-label="Session mode">
        {modes.map(([mode, label]) => (
          <button
            key={mode}
            className={timer.mode === mode ? 'selected' : ''}
            aria-pressed={timer.mode === mode}
            disabled={!!state.pending || data.storage_blocked}
            onClick={() => state.act({ type: 'select_mode', mode })}
          >
            <span>{label}</span>
          </button>
        ))}
      </div>

      <TimerRing state={state} />

      <div className="task-context">
        <span className="eyebrow">{timer.started ? 'Active task' : 'Focus target'}</span>
        <button
          className="task-picker-trigger"
          aria-expanded={picker}
          onClick={() => setPicker(!picker)}
        >
          <span className="task-picker-dot" aria-hidden="true" />
          <span className="task-picker-label">
            {task?.title || 'Focus without a task'}
            {task?.deleted ? ' · Deleted' : ''}
          </span>
          <IconChevronDown size={14} />
        </button>
        {timer.started && timer.selected_task !== timer.bound_task && (
          <span className="next-task">Next session · {next?.title || 'Without a task'}</span>
        )}
        {picker && (
          <div
            ref={pickerRoot as React.RefObject<HTMLDivElement>}
            className="task-picker"
            role="dialog"
            aria-label="Choose a focus task"
          >
            <p className="picker-hint">
              {timer.started
                ? 'Your running session keeps its task. Pick what comes next.'
                : 'Attach a task or run an open session.'}
            </p>
            <input
              aria-label="Find a focus task"
              placeholder="Filter tasks…"
              value={search}
              onChange={e => setSearch(e.target.value)}
            />
            <button
              onClick={() => {
                state.act({ type: 'select_task', id: null });
                setPicker(false);
              }}
            >
              <span>Without a task</span>
              {timer.selected_task === null && <IconCheck size={14} />}
            </button>
            <div className="picker-options">
              {tasks.length === 0 && (
                <p className="picker-hint">No matching tasks. Try another word.</p>
              )}
              {tasks.slice(0, 100).map(t => (
                <button
                  key={t.id}
                  onClick={() => {
                    state.act({ type: 'select_task', id: t.id });
                    setPicker(false);
                  }}
                >
                  <span>{t.title}</span>
                  {timer.selected_task === t.id && <IconCheck size={14} />}
                </button>
              ))}
            </div>
            <button className="picker-close" onClick={() => setPicker(false)}>
              Close picker
            </button>
          </div>
        )}
      </div>

      <TimerControls state={state} />

      <div className="cycle-progress">
        <div className="cycle-bars" aria-hidden="true">
          {Array.from({ length: data.settings.long_interval }, (_, i) => (
            <i key={i} className={i < completedInCycle ? 'complete' : ''} />
          ))}
        </div>
        <span>
          {completedInCycle} of {data.settings.long_interval} sessions before a long break
        </span>
      </div>

      <div className="stage-footer">
        <span className="mono-meta">
          {data.settings.focus_minutes}m focus · {data.settings.short_minutes}m break
        </span>
        <span>Sleep never counts as focus</span>
      </div>
    </section>
  );
}

export function TaskFocusAction({ state, task }: { state: OrbitState; task: Task }) {
  const fresh = !state.timer?.started && state.timer?.mode === 'Focus';
  return (
    <button
      className="focus-task-action"
      title={fresh ? 'Start focus' : 'Use next'}
      aria-label={`${fresh ? 'Start focus on' : 'Use next:'} ${task.title}`}
      disabled={!!state.pending || state.snapshot?.storage_blocked}
      onClick={() => {
        state.act({ type: 'start_task', id: task.id });
      }}
    >
      {fresh ? <IconPlay size={12} /> : <IconArrowUpRight size={14} />}
      <span>{fresh ? 'Focus' : 'Next'}</span>
    </button>
  );
}

export function MiniTime({ state }: { state: OrbitState }) {
  const timer = useLiveTimer(state);
  return (
    <strong role="timer" aria-label="Time remaining" className="mini-time-readout">
      {timer ? timeLabel(timer.remaining_ms) : '00:00'}
    </strong>
  );
}
