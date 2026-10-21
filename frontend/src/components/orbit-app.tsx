'use client';
import { AnimatePresence, MotionConfig, motion } from 'motion/react';
import { CircleDashed, ListChecks, ChartBar, SlidersHorizontal, ArrowUpRight, Minus, X, LockSimple, WarningCircle, ArrowCounterClockwise, CheckCircle } from '@phosphor-icons/react';
import { lazy, Suspense, useEffect, useMemo, useRef, useState } from 'react';
import { adapter, normalizeError, windowAction } from '@/lib/bridge';
import { motionPreference, type MotionLevel, type Settings } from '@/lib/protocol';
import { useOrbit, type OrbitState } from '@/lib/use-orbit';
import { Brand, Button, IconButton } from './ui';
import { FocusTimer, MiniTime, TimerControls, TimerRing } from './timer';
import { TasksView } from './tasks';
import { ExperienceContext, WorkspaceContext } from '@/lib/experience';
const ReportsView = lazy(() => import('./reports').then(m => ({default: m.ReportsView})));
const SettingsView = lazy(() => import('./settings').then(m => ({default: m.SettingsView})));
type Page = 'focus' | 'tasks' | 'reports' | 'settings';
export function OrbitApp({widget = false}: {widget?: boolean}) {
  const source = useOrbit();
  const memory = useMemo(() => ({values: new Map<string, unknown>()}), []);
  const [browserReduced, setBrowserReduced] = useState(false);
  useEffect(() => {const media = matchMedia("(prefers-reduced-motion: reduce)"); const update = () => setBrowserReduced(media.matches); update(); media.addEventListener("change", update); return () => media.removeEventListener("change", update);}, []);
  const [page, setPage] = useState<Page>('focus');
  const [draft, setDraft] = useState<Settings | null>(null);
  const [confirmQuit, setConfirmQuit] = useState(false);
  const preview = !widget ? draft ?? source.snapshot?.settings : source.snapshot?.settings;
  const systemReduced = browserReduced || source.appearance.high_contrast || (source.appearance.system_reduced_motion ?? source.appearance.reduced_motion);
  const requestedMotion = preview ? motionPreference(preview) : 'standard';
  const motionLevel: MotionLevel = requestedMotion === 'off' ? 'off' : systemReduced ? 'reduced' : requestedMotion;
  const state: OrbitState = {...source, appearance: {...source.appearance,
    dark: preview?.appearance === 1 ? false : preview?.appearance === 2 ? true : source.appearance.system_dark ?? source.appearance.dark,
    accent: preview?.accent ?? 'coral', density: preview?.density ?? 'comfortable',
    motion_level: motionLevel, reduced_motion: motionLevel !== 'standard',
  }};
  const experience = useMemo(() => ({motion: motionLevel, density: state.appearance.density!, visible: state.visible}), [motionLevel, state.appearance.density, state.visible]);
  const baseline = useRef('');
  const draftRef = useRef<Settings | null>(null); draftRef.current = draft;
  const native = (action: string) => {void windowAction(action).catch(e => state.setError(normalizeError(e)));};
  useEffect(() => {if (!widget && state.snapshot) void adapter().invoke('frontend_ready').catch(() => {});}, [widget, !!state.snapshot]);
  useEffect(() => {if (!widget && state.snapshot) {void adapter().invoke('set_timer_visibility', {visible: page === 'focus'}).catch(() => {}); if (page === 'focus') void state.refresh();}}, [page, widget, !!state.snapshot, state.refresh]);
  useEffect(() => {
    const settings = state.snapshot?.settings;
    if (!settings) return;
    const saved = JSON.stringify(settings);
    if (saved !== baseline.current) {
      // A durable acknowledgement advances the saved baseline without erasing a newer draft.
      if (!draftRef.current || JSON.stringify(draftRef.current) === baseline.current) setDraft({...settings});
      baseline.current = saved;
    }
  }, [state.snapshot?.settings]);
  const dirty = !!draft && !!state.snapshot && JSON.stringify(draft) !== JSON.stringify(state.snapshot.settings);
  useEffect(() => {if (!widget && state.snapshot) void adapter().invoke('set_settings_dirty', {dirty}).catch(e => state.setError(normalizeError(e)));}, [dirty, widget, !!state.snapshot]);
  useEffect(() => {
    if (widget) return;
    let cleanup: (() => void) | undefined; let ended = false;
    void adapter().listen('orbit://confirm-quit', () => setConfirmQuit(true)).then(fn => {if (ended) fn(); else cleanup = fn;}).catch(() => {});
    return () => {ended = true; cleanup?.();};
  }, [widget]);
  const theme = state.appearance.high_contrast ? 'contrast' : state.appearance.dark ? 'dark' : 'light';
  return <WorkspaceContext.Provider value={memory}><ExperienceContext.Provider value={experience}><MotionConfig reducedMotion={state.appearance.reduced_motion ? 'always' : 'user'} skipAnimations={motionLevel === 'off' || !state.visible} transition={{duration: motionLevel === 'reduced' ? .1 : .22, ease: [.22, 1, .36, 1]}}><div className={`orbit-app ${widget ? 'widget-app' : ''}`} data-theme={theme} data-accent={state.appearance.accent} data-density={state.appearance.density} data-motion={motionLevel} data-mica={!!state.appearance.mica_active} data-reduced-motion={state.appearance.reduced_motion}>
    {!state.snapshot || !state.timer ? <div className="connection-screen"><Brand/><div className="loading-orbit" aria-hidden="true"/><h1>{state.error ? 'Your workspace is taking a moment.' : 'A little room for focus.'}</h1><p>{state.error ? 'Open the desktop application to connect to your local workspace.' : 'Opening your private workspace…'}</p>{state.error && <><p className="connection-error">{state.error.message}</p><Button onClick={() => void state.refresh()}>Try again</Button></>}</div> : widget ? <Widget state={state}/> : <>
      <a className="skip-link" href="#main">Skip to workspace</a>
      <aside className="sidebar"><div className="sidebar-brand" data-tauri-drag-region><Brand/></div><div className="workspace-label eyebrow">YOUR SPACE</div><nav aria-label="Main navigation">{([['focus', CircleDashed, 'Focus'], ['tasks', ListChecks, 'Tasks'], ['reports', ChartBar, 'Insights']] as const).map(([id, Icon, label]) => <button key={id} title={label} className={page === id ? 'active' : ''} aria-current={page === id ? 'page' : undefined} onClick={() => setPage(id)}>{page === id && <motion.span className="nav-selection" layoutId="navigation" transition={{type: 'spring', stiffness: 350, damping: 35}}/>}<Icon size={20} weight={page === id ? 'duotone' : 'regular'}/><span>{label}</span>{id === 'tasks' && <small>{state.snapshot!.tasks.filter(t => !t.deleted && !t.done && t.planned_for === state.snapshot!.today).length}</small>}</button>)}</nav><div className="sidebar-bottom"><div className="sidebar-thought"><span className="little-star">✳</span><p>Less noise.<br/>More meaning.</p></div><button title="Settings" className={`settings-link ${page === 'settings' ? 'active' : ''}`} onClick={() => setPage('settings')}><SlidersHorizontal size={19}/><span>Settings</span>{dirty && <i className="draft-dot"/>}</button><div className="local-status"><span className="status-dot"/><span>Only on this device</span><LockSimple size={12}/></div></div></aside>
      <div className="app-body"><header className="titlebar" data-tauri-drag-region><div className="breadcrumbs"><span>Workspace</span><span>/</span><strong>{page === 'focus' ? 'Focus' : page === 'tasks' ? 'Tasks' : page === 'reports' ? 'Insights' : 'Settings'}</strong></div><div className="titlebar-right"><span className="saved-status"><span className={state.pending ? 'pending-dot' : 'status-dot'}/>{state.snapshot.storage_blocked ? 'Saving paused' : state.pending ? 'Saving…' : 'Everything’s local'}</span><div className="window-controls"><IconButton label="Minimize Orbit" onClick={() => native('minimize')}><Minus size={15}/></IconButton><IconButton label="Hide Orbit to tray" onClick={() => native('hide')}><X size={15}/></IconButton></div></div></header>
      <main id="main" tabIndex={-1}><div className="page-heading"><div><span className="eyebrow">{page === 'focus' ? 'A LITTLE INTENTION GOES A LONG WAY' : page === 'tasks' ? 'SMALL STEPS. REAL PROGRESS.' : page === 'reports' ? 'YOUR EFFORT, IN PERSPECTIVE' : 'YOUR ORBIT, YOUR WAY'}</span><h1>{page === 'focus' ? 'One thing at a time.' : page === 'tasks' ? 'Your work, in order.' : page === 'reports' ? 'Time well spent.' : 'Make yourself at home.'}</h1><p>{page === 'focus' ? 'Choose your work. Give it your full attention.' : page === 'tasks' ? 'A place for what matters today—and what can wait.' : page === 'reports' ? 'See the quiet progress that adds up.' : 'A few thoughtful details, just the way you like them.'}</p></div><span className="today-date">{new Date(`${state.snapshot.today}T12:00:00`).toLocaleDateString('en', {month: 'short', day: 'numeric'})}<span>{new Date(`${state.snapshot.today}T12:00:00`).toLocaleDateString('en', {weekday: 'long'})}</span></span></div>
      {state.snapshot.storage_blocked && <div className="storage-banner" role="alert"><WarningCircle size={21}/><div><strong>Your work is waiting to be saved.</strong><span>The timer is paused. Accepted changes are retained for retry.</span></div><Button onClick={() => state.act({type: 'retry_saving'})} disabled={!!state.pending}><ArrowCounterClockwise/>Retry saving</Button></div>}
      {state.snapshot.recovery_message && <details className="recovery-notice"><summary>Recovered session</summary><p>{state.snapshot.recovery_message}</p></details>}
      <motion.div key={page} className="page-content" initial={{opacity: 0, y: motionLevel === 'standard' ? 6 : 0}} animate={{opacity: 1, y: 0}} transition={{duration: motionLevel === 'off' ? 0 : motionLevel === 'reduced' ? .1 : .2}}><Suspense fallback={<div className="screen-loading" role="status">Opening workspace…</div>}>{page === 'focus' ? <div className="focus-workspace"><FocusTimer state={state}/><TasksView state={state} compact onViewAll={() => setPage('tasks')}/></div> : page === 'tasks' ? <TasksView state={state}/> : page === 'reports' ? <ReportsView state={state}/> : draft && <SettingsView state={state} draft={draft} setDraft={setDraft}/>}</Suspense></motion.div>
      <footer className="app-footer"><span>One session. One small step forward.</span><button onClick={() => native('hide')}>Keep focusing in the background <ArrowUpRight size={13}/></button></footer></main></div>
      {confirmQuit && <div className="dialog-scrim"><section role="alertdialog" aria-modal="true" aria-labelledby="quit-title" className="quit-dialog" onKeyDown={e => {
        if (e.key === 'Escape') {e.preventDefault(); setConfirmQuit(false); return;}
        if (e.key !== 'Tab') return;
        const buttons = e.currentTarget.querySelectorAll<HTMLButtonElement>('button');
        const first = buttons[0]; const last = buttons[buttons.length - 1];
        if (e.shiftKey && document.activeElement === first) {e.preventDefault(); last.focus();}
        else if (!e.shiftKey && document.activeElement === last) {e.preventDefault(); first.focus();}
      }}><h2 id="quit-title">Keep your changes?</h2><p>Your settings have unsaved changes. You can return to them or discard them before exiting.</p><div className="button-pair"><Button autoFocus primary onClick={() => {setConfirmQuit(false); setPage('settings');}}>Return to settings</Button><Button onClick={() => native('discard_and_quit')}>Discard & quit</Button></div></section></div>}
    </>}
    <AnimatePresence>{state.error && state.snapshot && <motion.div className="toast error-toast" role="alert" initial={{opacity: 0, x: '-50%', y: 12}} animate={{opacity: 1, x: '-50%', y: 0}} exit={{opacity: 0, x: '-50%'}}><WarningCircle size={20}/><div>{state.error.message}{state.error.accepted && <small>This change was accepted. Use Retry saving rather than submitting it again.</small>}</div><IconButton label="Dismiss error" onClick={() => state.setError(null)}><X/></IconButton></motion.div>}{state.notice && <motion.div className="toast" role="status" initial={{opacity: 0, x: '-50%', y: 12}} animate={{opacity: 1, x: '-50%', y: 0}} exit={{opacity: 0, x: '-50%'}}><CheckCircle size={20}/><span>{state.notice}</span><IconButton label="Dismiss notification" onClick={() => state.setNotice('')}><X/></IconButton></motion.div>}</AnimatePresence>
  </div></MotionConfig></ExperienceContext.Provider></WorkspaceContext.Provider>;
}
function Widget({state}: {state: OrbitState}) {
  const [kind, setKind] = useState(0);
  useEffect(() => {setKind(Number(new URLSearchParams(window.location.search).get('kind') || 0));}, []);
  return <div className={`desktop-widget widget-kind-${kind}`}><header data-tauri-drag-region><Brand small/><span>{kind === 1 ? 'Today’s plan' : state.timer?.mode === 'Focus' ? 'A little focus' : 'A little breathing room'}</span><IconButton label="Close widget" onClick={() => {void windowAction('close');}}><X size={14}/></IconButton></header>{kind === 1 ? <TasksView state={state} compact onViewAll={() => {void windowAction('show');}}/> : kind === 0 ? <><TimerRing state={state} mini/><TimerControls state={state} small/></> : <div className="mini-widget-content"><div><MiniTime state={state}/><span>{state.timer?.running ? 'In your flow' : state.timer?.started ? 'Paused' : 'Ready when you are'}</span></div><TimerControls state={state} small/></div>}</div>;
}
