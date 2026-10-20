'use client';
import { motion } from 'motion/react';
import { Pause, Play, ArrowCounterClockwise, SkipForward, ArrowUpRight, CaretDown, Check } from '@phosphor-icons/react';
import { useEffect, useRef, useState } from 'react';
import type { OrbitState } from '@/lib/use-orbit';
import type { Mode, Task } from '@/lib/protocol';
import { duration, timeLabel } from '@/lib/protocol';
import { Button, IconButton } from './ui';
const modes: [Mode, string][] = [['Focus', 'Focus'], ['ShortBreak', 'Short break'], ['LongBreak', 'Long break']];

export function TimerRing({state, mini = false}: {state: OrbitState; mini?: boolean}) {
  const timer = state.timer!;
  const reduced = state.appearance.reduced_motion;
  const progress = Math.max(0, Math.min(1, timer.remaining_ms / timer.total_ms));
  const [minutes, seconds] = timeLabel(timer.remaining_ms).split(':');
  const root = useRef<HTMLDivElement>(null);
  const [diameter, setDiameter] = useState(270);
  useEffect(() => {if (!root.current) return; const observer = new ResizeObserver(([entry]) => setDiameter(entry.contentRect.width)); observer.observe(root.current); return () => observer.disconnect();}, []);
  const angle = progress * Math.PI * 2;
  const radius = diameter * .4025;
  return <div ref={root} className={`timer-object ${timer.running ? 'is-running' : ''} ${mini ? 'widget-ring' : ''}`} data-mode={timer.mode}>
    <motion.div className="timer-aura" aria-hidden="true" initial={false} animate={{opacity: timer.running ? 1 : .5}} transition={{duration: reduced ? 0 : .4}}/>
    <svg className="orbit-dial" viewBox="0 0 400 400" aria-hidden="true">
      <defs><linearGradient id="ring-light" x1="0" y1="0" x2="1" y2="1"><stop offset="0%" stopColor="var(--accent-soft)"/><stop offset="100%" stopColor="var(--accent)"/></linearGradient></defs>
      <circle className="dial-outer" cx="200" cy="200" r="183" />
      {Array.from({length: 60}, (_, i) => <line key={i} x1="200" y1={i % 5 === 0 ? 22 : 25} x2="200" y2={i % 5 === 0 ? 32 : 29} transform={`rotate(${i * 6} 200 200)`} className={i % 5 === 0 ? 'dial-tick major' : 'dial-tick'} />)}
      <circle className="dial-inner" cx="200" cy="200" r="150" />
      <circle className="dial-track" cx="200" cy="200" r="161" />
      <circle className="dial-progress" cx="200" cy="200" r="161" stroke="url(#ring-light)" pathLength="1" strokeDasharray="1" strokeDashoffset={1 - progress} transform="rotate(-90 200 200)" />
    </svg>
    <motion.span className="progress-dot" aria-hidden="true" initial={false} animate={{transform: `translate(${Math.sin(angle) * radius - 3}px, ${-Math.cos(angle) * radius - 3}px)`}} transition={{duration: reduced ? 0 : timer.running ? .95 : .4, ease: 'linear'}}/>
    <div className="timer-face"><span className="eyebrow">{timer.mode === 'Focus' ? 'FOCUS SESSION' : timer.mode === 'ShortBreak' ? 'A MOMENT TO RESET' : 'ROOM TO RECHARGE'}</span><div className={`time-display ${minutes.length > 2 ? 'long-time' : ''}`} role="timer" aria-label={`${minutes} minutes ${seconds} seconds remaining`}><span>{minutes}</span><span className="time-colon">:</span><span>{seconds}</span></div><span className="timer-state"><i className={timer.running ? 'live-dot' : ''}/>{state.snapshot?.storage_blocked ? 'Saving paused' : timer.running ? 'In your flow' : timer.started ? 'Take your time' : 'Ready when you are'}</span></div>
  </div>;
}
export function TimerControls({state, small = false}: {state: OrbitState; small?: boolean}) {
  const timer = state.timer!;
  return <div className={`timer-controls ${small ? 'small-controls' : ''}`}>
    <IconButton label="Reset session" disabled={state.snapshot?.storage_blocked || !!state.pending} onClick={() => state.act({type: 'timer', action: 'reset'})}><ArrowCounterClockwise size={19}/></IconButton>
    <Button primary aria-label={timer.running ? 'Pause session' : timer.started ? 'Resume session' : timer.mode === 'Focus' ? 'Start focusing' : 'Start break'} disabled={state.snapshot?.storage_blocked || !!state.pending} onClick={() => state.act({type: 'timer', action: 'toggle'})}>{timer.running ? <Pause weight="fill" size={17}/> : <Play weight="fill" size={17}/>}<span>{timer.running ? 'Pause session' : timer.started ? 'Resume session' : timer.mode === 'Focus' ? 'Start focusing' : 'Start break'}</span>{!small && <kbd>⌃ ⌥ P</kbd>}</Button>
    <IconButton label="Skip session" disabled={state.snapshot?.storage_blocked || !!state.pending} onClick={() => state.act({type: 'timer', action: 'skip'})}><SkipForward size={19}/></IconButton>
  </div>;
}
export function FocusTimer({state}: {state: OrbitState}) {
  const {snapshot: data, timer} = state;
  const [picker, setPicker] = useState(false);
  const [search, setSearch] = useState('');
  if (!data || !timer) return null;
  const currentId = timer.started ? timer.bound_task : timer.selected_task;
  const task = data.tasks.find(t => t.id === currentId);
  const next = timer.started && timer.selected_task !== timer.bound_task ? data.tasks.find(t => t.id === timer.selected_task) : undefined;
  const tasks = data.tasks.filter(t => !t.done && !t.deleted && t.title.toLocaleLowerCase().includes(search.toLocaleLowerCase()));
  const cycle = timer.cycle % data.settings.long_interval;
  return <section className="focus-stage" aria-label="Focus timer">
    <div className="mode-tabs" role="group" aria-label="Session mode">{modes.map(([mode, label]) => <button key={mode} className={timer.mode === mode ? 'selected' : ''} aria-pressed={timer.mode === mode} disabled={!!state.pending || data.storage_blocked} onClick={() => state.act({type: 'select_mode', mode})}>{timer.mode === mode && <motion.span className="mode-indicator" layoutId="mode" transition={{type: 'spring', stiffness: 350, damping: 32}}/>}<span>{label}</span></button>)}</div>
    <TimerRing state={state}/>
    <div className="task-context"><span className="eyebrow">{timer.started ? 'FOCUSING ON' : 'MAKE SPACE FOR'}</span><button className="task-picker-trigger" aria-expanded={picker} onClick={() => setPicker(!picker)}><span>{task?.title || 'Focus without a task'}{task?.deleted ? ' · Deleted' : ''}</span><CaretDown size={14}/></button>
      {timer.started && timer.selected_task !== timer.bound_task && <span className="next-task">Next focus · {next?.title || 'Without a task'}</span>}
      {picker && <div className="task-picker"><input autoFocus aria-label="Find a focus task" placeholder="Find your next focus…" value={search} onChange={e => setSearch(e.target.value)}/><button onClick={() => {state.act({type: 'select_task', id: null}); setPicker(false);}}>Without a task {timer.selected_task === null && <Check/>}</button><div className="picker-options">{tasks.slice(0, 100).map(t => <button key={t.id} onClick={() => {state.act({type: 'select_task', id: t.id}); setPicker(false);}}>{t.title}{timer.selected_task === t.id && <Check/>}</button>)}</div><button className="picker-close" onClick={() => setPicker(false)}>Close picker</button></div>}
    </div>
    <TimerControls state={state}/>
    <div className="cycle-progress"><div className="cycle-dots" aria-hidden="true">{Array.from({length: data.settings.long_interval}, (_, i) => <i key={i} className={i < cycle || (timer.mode === 'LongBreak' && timer.cycle > 0) ? 'complete' : ''}/>)}</div><span>{timer.mode === 'LongBreak' && timer.cycle > 0 ? data.settings.long_interval : cycle} of {data.settings.long_interval} sessions before a long break</span></div>
    <div className="stage-footer"><span><span className="tiny-orbit"/>Stay with this moment.</span><span>Sleep never counts as focus</span></div>
  </section>;
}
export function TaskFocusAction({state, task}: {state: OrbitState; task: Task}) {
  const fresh = !state.timer?.started && state.timer?.mode === 'Focus';
  return <button className="focus-task-action" title={fresh ? 'Start focus' : 'Use next'} aria-label={`${fresh ? 'Start focus on' : 'Use next:'} ${task.title}`} disabled={!!state.pending || state.snapshot?.storage_blocked} onClick={() => {state.act({type: 'start_task', id: task.id});}}>{fresh ? <Play weight="fill" size={13}/> : <ArrowUpRight size={16}/>}<span>{fresh ? 'Focus' : 'Next'}</span></button>;
}
