'use client';
import { lazy, Suspense, useEffect, useMemo, useState } from 'react';
import { windowAction } from '@/lib/bridge';
import { motionPreference, type MotionLevel } from '@/lib/protocol';
import { useOrbit, type OrbitState } from '@/lib/use-orbit';
import { ExperienceContext, WorkspaceContext } from '@/lib/experience';
import { IconAlert, IconCheckCircle, IconClose } from './icons';
import { Brand, IconButton } from './ui';
import { MiniTime, TimerControls, TimerRing } from './timer';

// Only the Today's Tasks widget (kind === 1) loads TasksView — Timer Ring & Mini Timer stay featherweight.
const TasksView = lazy(() => import('./tasks').then(m => ({ default: m.TasksView })));

export function WidgetApp() {
  const source = useOrbit();
  const memory = useMemo(() => ({ values: new Map<string, unknown>() }), []);
  const [browserReduced, setBrowserReduced] = useState(false);
  const [kind, setKind] = useState(0);

  useEffect(() => {
    setKind(Number(new URLSearchParams(window.location.search).get('kind') || 0));
    const media = matchMedia('(prefers-reduced-motion: reduce)');
    const update = () => setBrowserReduced(media.matches);
    update();
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, []);

  const settings = source.snapshot?.settings;
  const systemReduced =
    browserReduced ||
    source.appearance.high_contrast ||
    (source.appearance.system_reduced_motion ?? source.appearance.reduced_motion);
  const requestedMotion = settings ? motionPreference(settings) : 'standard';
  const motionLevel: MotionLevel =
    requestedMotion === 'off' ? 'off' : systemReduced ? 'reduced' : requestedMotion;

  const state: OrbitState = {
    ...source,
    appearance: {
      ...source.appearance,
      dark:
        settings?.appearance === 1
          ? false
          : settings?.appearance === 2
            ? true
            : (source.appearance.system_dark ?? source.appearance.dark),
      accent: settings?.accent ?? 'coral',
      density: settings?.density ?? 'comfortable',
      motion_level: motionLevel,
      reduced_motion: motionLevel !== 'standard',
    },
  };

  const experience = useMemo(
    () => ({ motion: motionLevel, density: state.appearance.density!, visible: state.visible }),
    [motionLevel, state.appearance.density, state.visible],
  );

  const theme = state.appearance.high_contrast
    ? 'contrast'
    : state.appearance.dark
      ? 'dark'
      : 'light';

  return (
    <WorkspaceContext.Provider value={memory}>
      <ExperienceContext.Provider value={experience}>
        <div
          className="orbit-app widget-app"
          data-theme={theme}
          data-accent={state.appearance.accent}
          data-density={state.appearance.density}
          data-motion={motionLevel}
          data-reduced-motion={state.appearance.reduced_motion}
        >
          {!state.snapshot || !state.timer ? (
            <div className="desktop-widget widget-loading">
              <Brand small />
            </div>
          ) : (
            <div className={`desktop-widget widget-kind-${kind}`}>
              <header data-tauri-drag-region>
                <Brand small />
                <span>
                  {kind === 1
                    ? 'Today’s plan'
                    : state.timer.mode === 'Focus'
                      ? 'Focus session'
                      : state.timer.mode === 'ShortBreak'
                        ? 'Short break'
                        : 'Long break'}
                </span>
                <IconButton
                  label="Close widget"
                  onClick={() => {
                    void windowAction('close');
                  }}
                >
                  <IconClose size={13} />
                </IconButton>
              </header>

              {kind === 1 ? (
                <Suspense fallback={<div className="screen-loading">Loading tasks…</div>}>
                  <TasksView
                    state={state}
                    compact
                    onViewAll={() => {
                      void windowAction('show');
                    }}
                  />
                </Suspense>
              ) : kind === 0 ? (
                <>
                  <TimerRing state={state} mini />
                  <TimerControls state={state} small />
                </>
              ) : (
                <div className="mini-widget-content">
                  <div>
                    <MiniTime state={state} />
                    <span>
                      {state.timer.running
                        ? 'In flow'
                        : state.timer.started
                          ? 'Paused'
                          : 'Ready'}
                    </span>
                  </div>
                  <TimerControls state={state} small />
                </div>
              )}
            </div>
          )}

          {state.error && state.snapshot && (
            <div className="toast error-toast" role="alert">
              <IconAlert size={16} />
              <div>{state.error.message}</div>
              <IconButton label="Dismiss error" onClick={() => state.setError(null)}>
                <IconClose size={13} />
              </IconButton>
            </div>
          )}

          {state.notice && (
            <div className="toast" role="status">
              <IconCheckCircle size={16} />
              <span>{state.notice}</span>
              <IconButton label="Dismiss notification" onClick={() => state.setNotice('')}>
                <IconClose size={13} />
              </IconButton>
            </div>
          )}
        </div>
      </ExperienceContext.Provider>
    </WorkspaceContext.Provider>
  );
}
