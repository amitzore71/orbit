'use client';
import { AnimatePresence, motion } from 'motion/react';
import { Plus, DotsThree, Check, MagnifyingGlass, ArrowUp, ArrowDown, Trash, ArrowCounterClockwise, CalendarBlank, PencilSimple, ListChecks, ArrowRight, X } from '@phosphor-icons/react';
import { useContext, useEffect, useMemo, useRef, useState } from 'react';
import type { OrbitState } from '@/lib/use-orbit';
import { duration, filterTasks, type Task, type TaskAction } from '@/lib/protocol';
import { Button, Empty, IconButton, PanelHeading, Sheet } from './ui';
import { useWorkspaceState } from '@/lib/use-workspace-state';
import { useExperience, WorkspaceContext } from '@/lib/experience';
import { TaskFocusAction } from './timer';
export function TasksView({state, compact = false, onViewAll}: {state: OrbitState; compact?: boolean; onViewAll?: () => void}) {
  const data = state.snapshot!;
  const [view, setView] = useWorkspaceState('tasks.view', 'today');
  const [search, setSearch] = useWorkspaceState('tasks.search', '');
  const [title, setTitle] = useState('');
  const [estimate, setEstimate] = useState(1);
  const [adding, setAdding] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [undo, setUndo] = useState<string | null>(null);
  const [acceptedTitle, setAcceptedTitle] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const {motion: motionLevel} = useExperience();
  const rows = useMemo(() => filterTasks(data.tasks, compact ? 'today' : view, data.today, compact ? '' : search), [data.tasks, compact, view, data.today, search]);
  const titleError = [...title.trim()].length > 240 ? 'Use 240 characters or fewer.' : '';
  const estimateError = !Number.isInteger(estimate) || estimate < 1 || estimate > 99 ? 'Enter 1–99 sessions.' : '';
  const completed = data.tasks.filter(t => !t.deleted && t.done && t.planned_for === data.today).length;
  const carried = filterTasks(data.tasks, 'carry', data.today).length;
  const inspected = data.tasks.find(t => t.id === selected);
  useEffect(() => {if (!undo) return; const id = setTimeout(() => setUndo(null), 10000); return () => clearTimeout(id);}, [undo]);
  useEffect(() => {if (acceptedTitle !== null && !data.storage_blocked) {setTitle(prev => prev === acceptedTitle ? '' : prev); setAcceptedTitle(null);}}, [acceptedTitle, data.storage_blocked]);
  async function add(start: boolean) {
    if (!title.trim() || [...title.trim()].length > 240 || adding || estimateError) return;
    setAdding(true);
    const submitted = title;
    try {await state.run({type: 'create_task', title, estimate, today: compact || view === 'today', start}); setTitle(prev => prev === submitted ? '' : prev); setEstimate(1); input.current?.focus();}
    catch (e) {if (e && typeof e === 'object' && 'accepted' in e && e.accepted) setAcceptedTitle(submitted);}
    finally {setAdding(false);}
  }
  async function action(task: Task, action: TaskAction) {
    try {await state.run({type: 'task', id: task.id, action}); if (action === 'delete') {setUndo(task.id); setSelected(null);}}
    catch { /* The shared error surface owns recovery. */ }
  }
  return <section className={`tasks-surface ${compact ? 'today-panel' : 'tasks-page'}`} aria-label={compact ? 'Today’s plan' : 'Task planning'}>
    {compact ? <PanelHeading eyebrow="A LITTLE INTENTION" title="Today’s plan" action={<IconButton label="Add a task" onClick={() => input.current?.focus()}><Plus size={19}/></IconButton>}/> : <div className="task-toolbar"><div className="filter-tabs" aria-label="Task views">{[['today', 'Today'], ['all', 'All active'], ['completed', 'Completed'], ['trash', 'Trash']].map(([id, label]) => <button key={id} className={view === id ? 'active' : ''} aria-pressed={view === id} onClick={() => setView(id)}>{label}{id === 'today' && <span>{filterTasks(data.tasks, 'today', data.today).length}</span>}</button>)}</div><label className="search-field"><MagnifyingGlass size={16}/><input aria-label="Search tasks" placeholder="Find a task…" value={search} onChange={e => setSearch(e.target.value)}/>{search && <IconButton label="Clear search" onClick={() => setSearch('')}><X/></IconButton>}</label></div>}
    {compact && <div className="today-summary"><span>{rows.length} planned <i>·</i> {completed} completed</span><span className="day-marker">{new Date(`${data.today}T12:00:00`).toLocaleDateString('en', {weekday: 'short', day: 'numeric'})}</span></div>}
    {!compact && carried > 0 && <button className={`carry-forward ${view === 'carry' ? 'active' : ''}`} onClick={() => setView(view === 'carry' ? 'today' : 'carry')}><CalendarBlank size={18}/><span>{carried} unfinished {carried === 1 ? 'task' : 'tasks'} from earlier days</span><span>Review <ArrowRight size={15}/></span></button>}
    {(!compact && view === 'trash') || view === 'completed' ? null : <form className={`quick-add ${compact ? 'compact-add' : ''}`} onSubmit={e => {e.preventDefault(); void add(false);}}><Plus size={18}/><input ref={input} aria-label="Task title" placeholder={compact ? 'Something worth focusing on…' : 'What would you like to work on?'} value={title} onChange={e => setTitle(e.target.value)} maxLength={480} disabled={adding || data.storage_blocked}/>{!compact && <label className="estimate-input"><input aria-label="Estimated sessions" type="number" min={1} max={99} value={estimate} onChange={e => setEstimate(Number(e.target.value))}/><span>sessions</span></label>}<button type="submit" className="add-submit" aria-label="Add task" disabled={!title.trim() || !!titleError || !!estimateError || adding || data.storage_blocked}>{adding ? 'Saving…' : compact ? <ArrowRight/> : 'Add task'}</button>{!compact && <button type="button" className="add-and-focus" disabled={!title.trim() || !!titleError || !!estimateError || adding || data.storage_blocked} onClick={() => void add(true)}>Add & focus</button>}</form>}
    {(titleError || estimateError) && <p className="field-error" role="alert">{titleError || estimateError}</p>}
    {rows.length > 0 ? <VirtualTasks memoryKey={compact ? "today.scroll" : "tasks.scroll"} rows={rows} render={(task, index) => <motion.div layout={motionLevel === "standard" ? "position" : false} initial={{opacity: 0, y: motionLevel === "standard" ? 7 : 0}} animate={{opacity: 1, y: 0}} transition={{duration: motionLevel === "off" ? 0 : motionLevel === "reduced" ? .1 : .2}} className={`task-row ${task.id === state.timer?.selected_task ? 'is-selected' : ''} ${task.done ? 'is-done' : ''}`}>
      <button className={`task-check ${task.done ? 'checked' : ''}`} aria-label={`${task.done ? 'Reopen' : 'Complete'} ${task.title}`} disabled={data.storage_blocked || !!state.pending || task.deleted} onClick={() => void action(task, 'toggle_done')}>{task.done ? <Check size={13} weight="bold"/> : <span className="task-number">{String(index + 1).padStart(2, '0')}</span>}</button>
      <button className="task-description" onClick={() => setSelected(task.id)} title={task.title}><span>{task.title}</span><small>{task.completed} / {task.estimate} sessions{task.planned_for && task.planned_for !== data.today ? ` · ${task.planned_for}` : ''}<span className="task-progress-dots" aria-hidden="true">{Array.from({length: Math.min(task.estimate, 5)}, (_, i) => <i key={i} className={i < task.completed ? 'filled' : ''}/>)}</span></small></button>
      {!task.done && !task.deleted && <TaskFocusAction state={state} task={task}/>}
      {task.deleted ? <IconButton label={`Restore ${task.title}`} disabled={data.storage_blocked || !!state.pending} onClick={() => void action(task, 'restore')}><ArrowCounterClockwise size={18}/></IconButton> : <IconButton label={`Edit ${task.title}`} onClick={() => setSelected(task.id)}><DotsThree size={23}/></IconButton>}
    </motion.div>}/> : <Empty title={search ? 'No matching tasks' : view === 'trash' ? 'Nothing lost.' : view === 'completed' ? 'Good things take a little time.' : view === 'carry' ? 'You’re all caught up.' : compact ? 'A fresh page for today.' : 'Make room for your next idea.'}>{search ? 'Try a different word or clear your search.' : view === 'trash' ? 'Deleted tasks stay here, ready to restore.' : view === 'completed' ? 'Completed tasks will appear here. Small steps count.' : 'Add one thing you want to give your attention to.'}</Empty>}
    {undo && <div className="undo-notice" role="status"><span>Task moved to Trash</span><button onClick={() => {const task = data.tasks.find(t => t.id === undo); if (task) void action(task, 'restore'); setUndo(null);}}>Undo</button></div>}
    {compact && <><button className="view-all" onClick={onViewAll}>See all your tasks <ArrowUpRightIcon/></button><div className="today-totals"><div><span className="eyebrow">TIME GIVEN TODAY</span><strong>{duration(data.today_ms)}</strong></div><div><span className="eyebrow">SESSIONS FINISHED</span><strong>{String(data.today_completed).padStart(2, '0')}<span className="total-suffix"> / today</span></strong></div></div><p className="plan-note"><ListChecks size={17}/>A short list. A clear head.</p></>}
    <AnimatePresence>{inspected && <TaskInspector key={inspected.id} task={inspected} state={state} action={action} close={() => setSelected(null)}/>}</AnimatePresence>
  </section>;
}
function ArrowUpRightIcon() {return <svg width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M4 12 12 4M4 4h8v8" stroke="currentColor" strokeWidth="1.5"/></svg>;}
function VirtualTasks({rows, render, memoryKey}: {rows: Task[]; render: (task: Task, index: number) => React.ReactNode; memoryKey: string}) {
  const memory = useContext(WorkspaceContext);
  const {density} = useExperience();
  const rowHeight = density === 'compact' ? 68 : 84;
  const [scroll, setScroll] = useState(() => Number(memory.values.get(memoryKey) ?? 0));
  const root = useRef<HTMLDivElement>(null);
  const [focusTarget, setFocusTarget] = useState<number | null>(null);
  useEffect(() => {
    if (!root.current) return;
    const maximum = Math.max(0, rows.length * rowHeight - root.current.clientHeight);
    const restored = Math.min(Number(memory.values.get(memoryKey) ?? 0), maximum);
    root.current.scrollTop = restored; setScroll(restored);
  }, [rows.length, rowHeight, memory, memoryKey]);
  useEffect(() => {
    if (focusTarget !== null) root.current?.querySelector<HTMLElement>(`[data-row="${focusTarget}"] .task-description`)?.focus();
  }, [focusTarget, scroll]);
  const start = Math.max(0, Math.floor(scroll / rowHeight) - 3);
  const end = Math.min(rows.length, start + 22);
  return <div className="task-list" ref={root} tabIndex={0} onKeyDown={e => {
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(e.key)) return;
    const current = Number((e.target as HTMLElement).closest('[data-row]')?.getAttribute('data-row') ?? -1);
    const next = e.key === 'Home' ? 0 : e.key === 'End' ? rows.length - 1 : Math.max(0, Math.min(rows.length - 1, current + (e.key === 'ArrowDown' ? 1 : -1)));
    e.preventDefault();
    if (root.current) {
      const top = next * rowHeight;
      if (top < root.current.scrollTop || top + rowHeight > root.current.scrollTop + root.current.clientHeight) root.current.scrollTop = top;
      setScroll(root.current.scrollTop);
    }
    setFocusTarget(next);
  }} onScroll={e => {const value = e.currentTarget.scrollTop; memory.values.set(memoryKey, value); setScroll(value);}} role="list" aria-label="Tasks"><div style={{height: rows.length * rowHeight, position: 'relative'}}>{rows.slice(start, end).map((task, i) => <div role="listitem" data-row={start + i} aria-posinset={start + i + 1} aria-setsize={rows.length} key={task.id} style={{position: 'absolute', top: (start + i) * rowHeight, width: '100%', height: rowHeight}}>{render(task, start + i)}</div>)}</div></div>;
}

function TaskInspector({task, state, action, close}: {task: Task; state: OrbitState; action: (task: Task, action: TaskAction) => Promise<void>; close: () => void}) {
  const [title, setTitle] = useState(task.title);
  const [estimate, setEstimate] = useState(task.estimate);
  return <Sheet title="A closer look" close={close}><span className="eyebrow">TASK DETAILS</span><form className="detail-form" onSubmit={e => {e.preventDefault(); void state.run({type: 'edit_task', id: task.id, title, estimate}).then(() => state.setNotice('Task updated.')).catch(() => {});}}><label>Title<textarea value={title} maxLength={480} onChange={e => setTitle(e.target.value)} required/></label><label>Estimated focus sessions<input type="number" min={1} max={99} value={estimate} onChange={e => setEstimate(Number(e.target.value))}/></label>{([...title.trim()].length > 240 || !title.trim()) && <p className="field-error" role="alert">Enter a title of 1–240 characters.</p>}{(!Number.isInteger(estimate) || estimate < 1 || estimate > 99) && <p className="field-error" role="alert">Enter 1–99 sessions.</p>}<p className="muted">{task.completed} completed sessions. Your session history stays with this task.</p><Button primary disabled={state.snapshot?.storage_blocked || !!state.pending || !title.trim() || [...title.trim()].length > 240 || !Number.isInteger(estimate) || estimate < 1 || estimate > 99}><PencilSimple/>Save changes</Button></form><div className="detail-actions">{!task.deleted && <><Button onClick={() => void action(task, task.planned_for === state.snapshot?.today ? 'unplan' : 'plan')}><CalendarBlank/>{task.planned_for === state.snapshot?.today ? 'Remove from today' : 'Plan for today'}</Button><div className="button-pair"><Button onClick={() => void action(task, 'up')}><ArrowUp/>Move up</Button><Button onClick={() => void action(task, 'down')}><ArrowDown/>Move down</Button></div><Button onClick={() => void action(task, 'toggle_done')}><Check/>{task.done ? 'Reopen task' : 'Mark complete'}</Button></>}<Button className={task.deleted ? '' : 'danger'} onClick={() => void action(task, task.deleted ? 'restore' : 'delete')}>{task.deleted ? <ArrowCounterClockwise/> : <Trash/>}{task.deleted ? 'Restore task' : 'Move to Trash'}</Button></div></Sheet>;
}
