'use client';
import { lazy, Suspense, useEffect, useMemo, useRef, useState } from 'react';
import { adapter, normalizeError, windowAction } from '@/lib/bridge';
import { AmbientEngine } from '@/lib/ambient';
import {
  duration,
  motionPreference,
  normalizeSettings,
  type MotionLevel,
  type Settings,
  type WindowState,
} from '@/lib/protocol';
import { useOrbit, type OrbitState } from '@/lib/use-orbit';
import { ExperienceContext, WorkspaceContext } from '@/lib/experience';
import {
  IconAlert,
  IconArrowUpRight,
  IconCheckCircle,
  IconClose,
  IconCompress,
  IconExpand,
  IconFocus,
  IconInsights,
  IconLock,
  IconMaximize,
  IconMinus,
  IconReset,
  IconRestore,
  IconSearch,
  IconSettings,
  IconTasks,
} from './icons';
import { Brand, Button, IconButton } from './ui';
import { FocusTimer } from './timer';
import { TasksView } from './tasks';
import { CommandPalette } from './command-palette';

const ReportsView = lazy(() => import('./reports').then(m => ({ default: m.ReportsView })));
const SettingsView = lazy(() => import('./settings').then(m => ({ default: m.SettingsView })));

type Page = 'focus' | 'tasks' | 'reports' | 'settings';

const pageMeta: Record<Page, { crumb: string; label: string; title: string; subtitle: string }> = {
  focus: {
    crumb: 'Focus',
    label: 'Session workspace',
    title: 'One thing at a time.',
    subtitle: 'Pick a task, run the chronometer, and track what gets done today.',
  },
  tasks: {
    crumb: 'Tasks',
    label: 'Task planner',
    title: 'Work in order.',
    subtitle: 'Schedule today’s queue, set estimates, and carry unfinished work forward.',
  },
  reports: {
    crumb: 'Insights',
    label: 'Focus log',
    title: 'Time well spent.',
    subtitle: 'Daily, weekly, and custom reports from your local session history.',
  },
  settings: {
    crumb: 'Settings',
    label: 'Configuration',
    title: 'Workspace settings.',
    subtitle: 'Durations, theme, shortcuts, desktop widgets, and local backups.',
  },
};

export function OrbitApp() {
  const source = useOrbit();
  const memory = useMemo(() => ({ values: new Map<string, unknown>() }), []);
  const [browserReduced, setBrowserReduced] = useState(false);

  useEffect(() => {
    const media = matchMedia('(prefers-reduced-motion: reduce)');
    const update = () => setBrowserReduced(media.matches);
    update();
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, []);

  const [page, setPage] = useState<Page>('focus');
  const [draft, setDraft] = useState<Settings | null>(null);
  const [confirmQuit, setConfirmQuit] = useState(false);
  const [maximized, setMaximized] = useState(false);
  const [zen, setZen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);

  const preview = draft ?? source.snapshot?.settings;
  const systemReduced =
    browserReduced ||
    source.appearance.high_contrast ||
    (source.appearance.system_reduced_motion ?? source.appearance.reduced_motion);
  const requestedMotion = preview ? motionPreference(preview) : 'standard';
  const motionLevel: MotionLevel =
    requestedMotion === 'off' ? 'off' : systemReduced ? 'reduced' : requestedMotion;

  const state: OrbitState = {
    ...source,
    appearance: {
      ...source.appearance,
      dark:
        preview?.appearance === 1
          ? false
          : preview?.appearance === 2
            ? true
            : (source.appearance.system_dark ?? source.appearance.dark),
      accent: preview?.accent ?? 'coral',
      density: preview?.density ?? 'comfortable',
      motion_level: motionLevel,
      reduced_motion: motionLevel !== 'standard',
    },
  };

  const experience = useMemo(
    () => ({ motion: motionLevel, density: state.appearance.density!, visible: state.visible }),
    [motionLevel, state.appearance.density, state.visible],
  );

  const baseline = useRef('');
  const draftRef = useRef<Settings | null>(null);
  draftRef.current = draft;
  const ambientRef = useRef<AmbientEngine | null>(null);

  useEffect(() => {
    const engine = new AmbientEngine();
    ambientRef.current = engine;
    return () => {
      engine.dispose();
      ambientRef.current = null;
    };
  }, []);

  useEffect(() => {
    const settings = preview;
    const timer = state.timer;
    if (!ambientRef.current || !settings || !timer) return;
    const preset = settings.ambient_sound ?? 'off';
    const volume = settings.ambient_volume ?? 35;
    const duringBreaks = settings.ambient_during_breaks ?? false;
    const shouldPlay =
      timer.running && (timer.mode === 'Focus' || duringBreaks) && preset !== 'off';
    ambientRef.current.sync(preset, volume, shouldPlay);
  }, [
    preview?.ambient_sound,
    preview?.ambient_volume,
    preview?.ambient_during_breaks,
    state.timer?.running,
    state.timer?.mode,
  ]);

  const native = (action: string) => {
    void windowAction(action).catch(e => state.setError(normalizeError(e)));
  };

  useEffect(() => {
    if (state.snapshot) void adapter().invoke('frontend_ready').catch(() => {});
  }, [!!state.snapshot]);

  useEffect(() => {
    if (state.snapshot) {
      void adapter()
        .invoke('set_timer_visibility', { visible: page === 'focus' })
        .catch(() => {});
      if (page === 'focus') void state.refresh();
    }
  }, [page, !!state.snapshot, state.refresh]);

  useEffect(() => {
    const settings = state.snapshot?.settings;
    if (!settings) return;
    const saved = JSON.stringify(settings);
    if (saved !== baseline.current) {
      if (
        !draftRef.current ||
        JSON.stringify(draftRef.current) === baseline.current ||
        JSON.stringify(normalizeSettings(draftRef.current)) ===
          JSON.stringify(normalizeSettings(settings))
      ) {
        setDraft({ ...settings });
      }
      baseline.current = saved;
    }
  }, [state.snapshot?.settings]);

  const dirty =
    !!draft &&
    !!state.snapshot &&
    JSON.stringify(draft) !== JSON.stringify(state.snapshot.settings);

  useEffect(() => {
    if (state.snapshot) {
      void adapter()
        .invoke('set_settings_dirty', { dirty })
        .catch(e => state.setError(normalizeError(e)));
    }
  }, [dirty, !!state.snapshot]);

  useEffect(() => {
    let cleanup: (() => void) | undefined;
    let ended = false;
    void adapter()
      .listen('orbit://confirm-quit', () => setConfirmQuit(true))
      .then(fn => {
        if (ended) fn();
        else cleanup = fn;
      })
      .catch(() => {});
    return () => {
      ended = true;
      cleanup?.();
    };
  }, []);

  useEffect(() => {
    let cleanup: (() => void) | undefined;
    let ended = false;
    void adapter()
      .invoke<WindowState>('get_window_state')
      .then(s => {
        if (!ended && s && typeof s.maximized === 'boolean') setMaximized(s.maximized);
      })
      .catch(() => {});
    void adapter()
      .listen<WindowState>('orbit://window-state', s => {
        if (!ended && s && typeof s.maximized === 'boolean') setMaximized(s.maximized);
      })
      .then(fn => {
        if (ended) fn();
        else cleanup = fn;
      })
      .catch(() => {});
    return () => {
      ended = true;
      cleanup?.();
    };
  }, []);

  const toggleZen = () => {
    setZen(prev => {
      const next = !prev;
      if (next) setPage('focus');
      return next;
    });
  };

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setPaletteOpen(v => !v);
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && !e.altKey && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        toggleZen();
        return;
      }
      if (e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
        if (e.key === '1') {
          e.preventDefault();
          setZen(false);
          setPage('focus');
        } else if (e.key === '2') {
          e.preventDefault();
          setZen(false);
          setPage('tasks');
        } else if (e.key === '3') {
          e.preventDefault();
          setZen(false);
          setPage('reports');
        } else if (e.key === '4') {
          e.preventDefault();
          setZen(false);
          setPage('settings');
        }
        return;
      }
      if (
        e.key === 'Escape' &&
        zen &&
        !paletteOpen &&
        !confirmQuit &&
        !document.querySelector('[role="dialog"], [role="alertdialog"]')
      ) {
        e.preventDefault();
        setZen(false);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [zen, paletteOpen, confirmQuit]);

  const theme = state.appearance.high_contrast
    ? 'contrast'
    : state.appearance.dark
      ? 'dark'
      : 'light';

  const todayCount = useMemo(
    () =>
      state.snapshot
        ? state.snapshot.tasks.filter(
            t => !t.deleted && t.planned_for === state.snapshot!.today,
          ).length
        : 0,
    [state.snapshot],
  );

  const dailyGoal = state.snapshot?.settings.daily_goal_sessions ?? 4;
  const goalMet = !!state.snapshot && state.snapshot.today_completed >= dailyGoal;
  const meta = pageMeta[page];

  return (
    <WorkspaceContext.Provider value={memory}>
      <ExperienceContext.Provider value={experience}>
        <div
          className="orbit-app"
          data-theme={theme}
          data-accent={state.appearance.accent}
          data-density={state.appearance.density}
          data-motion={motionLevel}
          data-mica={!!state.appearance.mica_active}
          data-reduced-motion={state.appearance.reduced_motion}
          data-maximized={maximized}
          data-zen={zen}
        >
          {!state.snapshot || !state.timer ? (
            <div className="connection-screen">
              <Brand />
              <div className="loading-orbit" aria-hidden="true" />
              <h1>
                {state.error ? 'Workspace connection interrupted.' : 'Opening your workspace.'}
              </h1>
              <p>
                {state.error
                  ? 'Launch the Orbit desktop app to connect to your local SQLite database.'
                  : 'Loading your tasks, settings, and timer checkpoint…'}
              </p>
              {state.error && (
                <>
                  <p className="connection-error">{state.error.message}</p>
                  <Button onClick={() => void state.refresh()}>Try again</Button>
                </>
              )}
            </div>
          ) : (
            <>
              <a className="skip-link" href="#main">
                Skip to workspace
              </a>
              <aside className="sidebar">
                <div className="sidebar-brand" data-tauri-drag-region>
                  <Brand />
                </div>
                <div className="workspace-label">Workspace</div>
                <nav aria-label="Main navigation">
                  {(
                    [
                      ['focus', IconFocus, 'Focus'],
                      ['tasks', IconTasks, 'Tasks'],
                      ['reports', IconInsights, 'Insights'],
                    ] as const
                  ).map(([id, Icon, label]) => (
                    <button
                      key={id}
                      title={label}
                      className={page === id ? 'active' : ''}
                      aria-current={page === id ? 'page' : undefined}
                      onClick={() => setPage(id)}
                    >
                      <Icon size={18} filled={page === id} />
                      <span>{label}</span>
                      {id === 'tasks' && <small>{todayCount}</small>}
                    </button>
                  ))}
                </nav>

                <div className="sidebar-bottom">
                  <div
                    className={`sidebar-pulse ${goalMet ? 'goal-met' : ''}`}
                    aria-label="Today summary"
                  >
                    <div className="pulse-row">
                      <span>Today</span>
                      <strong className="mono-meta">{duration(state.snapshot.today_ms)}</strong>
                    </div>
                    <div className="pulse-bar" aria-hidden="true">
                      <i
                        style={{
                          transform: `scaleX(${Math.min(
                            1,
                            state.snapshot.today_completed / Math.max(dailyGoal, 1),
                          )})`,
                        }}
                      />
                    </div>
                    <span className="pulse-meta">
                      {state.snapshot.today_completed} / {dailyGoal} daily goal
                      {goalMet ? ' · Reached' : ''}
                    </span>
                  </div>

                  <button
                    title="Settings"
                    className={`settings-link ${page === 'settings' ? 'active' : ''}`}
                    onClick={() => setPage('settings')}
                  >
                    <IconSettings size={18} />
                    <span>Settings</span>
                    {dirty && <i className="draft-dot" />}
                  </button>

                  <div className="local-status">
                    <span className="status-dot" />
                    <span>Local SQLite</span>
                    <IconLock size={12} />
                  </div>
                </div>
              </aside>

              <div className="app-body">
                <header
                  className="titlebar"
                  data-tauri-drag-region
                  onDoubleClick={e => {
                    if ((e.target as HTMLElement).closest('button, a, input, [role="button"]')) {
                      return;
                    }
                    native('toggle_maximize');
                  }}
                >
                  <div className="breadcrumbs">
                    <span>Orbit</span>
                    <span className="crumb-sep">/</span>
                    <strong>{zen ? 'Zen Focus' : meta.crumb}</strong>
                  </div>
                  <div className="titlebar-right">
                    <button
                      type="button"
                      className="palette-trigger"
                      aria-label="Open command palette"
                      onClick={() => setPaletteOpen(true)}
                    >
                      <IconSearch size={13} />
                      <span>Quick actions</span>
                      <kbd>Ctrl+K</kbd>
                    </button>
                    <span className="saved-status">
                      <span className={state.pending ? 'pending-dot' : 'status-dot'} />
                      {state.snapshot.storage_blocked
                        ? 'Saving paused'
                        : state.pending
                          ? 'Saving…'
                          : 'Stored on device'}
                    </span>
                    <div className="window-controls">
                      <IconButton
                        label={zen ? 'Exit Zen mode' : 'Zen focus mode'}
                        className={zen ? 'active' : ''}
                        onClick={toggleZen}
                      >
                        {zen ? <IconCompress size={14} /> : <IconExpand size={14} />}
                      </IconButton>
                      <IconButton label="Minimize Orbit" onClick={() => native('minimize')}>
                        <IconMinus size={14} />
                      </IconButton>
                      <IconButton
                        label={maximized ? 'Restore Orbit window' : 'Maximize Orbit'}
                        onClick={() => native('toggle_maximize')}
                      >
                        {maximized ? <IconRestore size={13} /> : <IconMaximize size={13} />}
                      </IconButton>
                      <IconButton
                        label="Hide Orbit to tray"
                        className="window-close"
                        onClick={() => native('hide')}
                      >
                        <IconClose size={14} />
                      </IconButton>
                    </div>
                  </div>
                </header>

                <main id="main" tabIndex={-1}>
                  <div className="page-heading">
                    <div>
                      <span className="eyebrow">{meta.label}</span>
                      <h1>{meta.title}</h1>
                      <p>{meta.subtitle}</p>
                    </div>
                    <span className="today-date">
                      {new Date(`${state.snapshot.today}T12:00:00`).toLocaleDateString('en', {
                        month: 'short',
                        day: 'numeric',
                      })}
                      <span>
                        {new Date(`${state.snapshot.today}T12:00:00`).toLocaleDateString('en', {
                          weekday: 'long',
                        })}
                      </span>
                    </span>
                  </div>

                  {state.snapshot.storage_blocked && (
                    <div className="storage-banner" role="alert">
                      <IconAlert size={20} />
                      <div>
                        <strong>Your work is waiting to be saved.</strong>
                        <span>The timer is paused. Accepted changes are retained for retry.</span>
                      </div>
                      <Button
                        onClick={() => state.act({ type: 'retry_saving' })}
                        disabled={!!state.pending}
                      >
                        <IconReset size={15} />
                        Retry saving
                      </Button>
                    </div>
                  )}

                  {state.snapshot.recovery_message && (
                    <details className="recovery-notice">
                      <summary>Recovered session</summary>
                      <p>{state.snapshot.recovery_message}</p>
                    </details>
                  )}

                  <div key={page} className="page-content">
                    <Suspense
                      fallback={
                        <div className="screen-loading" role="status">
                          Loading view…
                        </div>
                      }
                    >
                      {page === 'focus' ? (
                        <div className="focus-workspace">
                          <FocusTimer state={state} />
                          <TasksView state={state} compact onViewAll={() => setPage('tasks')} />
                        </div>
                      ) : page === 'tasks' ? (
                        <TasksView state={state} />
                      ) : page === 'reports' ? (
                        <ReportsView state={state} />
                      ) : (
                        draft && <SettingsView state={state} draft={draft} setDraft={setDraft} />
                      )}
                    </Suspense>
                  </div>

                  <footer className="app-footer">
                    <span>Offline · No account or telemetry</span>
                    <button onClick={() => native('hide')}>
                      Run in system tray <IconArrowUpRight size={13} />
                    </button>
                  </footer>
                </main>
              </div>

              {confirmQuit && (
                <div className="dialog-scrim">
                  <section
                    role="alertdialog"
                    aria-modal="true"
                    aria-labelledby="quit-title"
                    className="quit-dialog"
                    onKeyDown={e => {
                      if (e.key === 'Escape') {
                        e.preventDefault();
                        setConfirmQuit(false);
                        return;
                      }
                      if (e.key !== 'Tab') return;
                      const buttons = e.currentTarget.querySelectorAll<HTMLButtonElement>('button');
                      const first = buttons[0];
                      const last = buttons[buttons.length - 1];
                      if (e.shiftKey && document.activeElement === first) {
                        e.preventDefault();
                        last.focus();
                      } else if (!e.shiftKey && document.activeElement === last) {
                        e.preventDefault();
                        first.focus();
                      }
                    }}
                  >
                    <h2 id="quit-title">Keep your changes?</h2>
                    <p>
                      Your settings have unsaved changes. Return to save them or discard them before
                      exiting.
                    </p>
                    <div className="button-pair">
                      <Button
                        autoFocus
                        primary
                        onClick={() => {
                          setConfirmQuit(false);
                          setPage('settings');
                        }}
                      >
                        Return to settings
                      </Button>
                      <Button onClick={() => native('discard_and_quit')}>Discard &amp; quit</Button>
                    </div>
                  </section>
                </div>
              )}
              <CommandPalette
                open={paletteOpen}
                close={() => setPaletteOpen(false)}
                state={state}
                page={page}
                setPage={p => {
                  setZen(false);
                  setPage(p);
                }}
                zen={zen}
                toggleZen={toggleZen}
                maximized={maximized}
                toggleMaximize={() => native('toggle_maximize')}
              />
            </>
          )}

          {state.error && state.snapshot && (
            <div className="toast error-toast" role="alert">
              <IconAlert size={18} />
              <div>
                {state.error.message}
                {state.error.accepted && (
                  <small>
                    This change was accepted. Use Retry saving rather than submitting it again.
                  </small>
                )}
              </div>
              <IconButton label="Dismiss error" onClick={() => state.setError(null)}>
                <IconClose size={14} />
              </IconButton>
            </div>
          )}

          {state.notice && (
            <div className="toast" role="status">
              <IconCheckCircle size={18} />
              <span>{state.notice}</span>
              <IconButton label="Dismiss notification" onClick={() => state.setNotice('')}>
                <IconClose size={14} />
              </IconButton>
            </div>
          )}
        </div>
      </ExperienceContext.Provider>
    </WorkspaceContext.Provider>
  );
}
