'use client';
import { useEffect, useRef, useState } from 'react';
import type { OrbitState } from '@/lib/use-orbit';
import { motionPreference, normalizeSettings, type Settings } from '@/lib/protocol';
import { adapter, command, chooseFile, normalizeError, windowAction } from '@/lib/bridge';
import { useWorkspaceState } from '@/lib/use-workspace-state';
import {
  IconArrowDown,
  IconArrowUp,
  IconCheck,
  IconFolder,
  IconKeyboard,
  IconReset,
  IconSpeaker,
} from './icons';
import { Brand, Button } from './ui';

const groups = ['Sessions', 'Sound', 'Appearance', 'Shortcuts', 'Widgets', 'Data', 'About'];

const groupDescriptions: Record<string, string> = {
  Sessions: 'Set focus lengths, break intervals, and automatic handoffs.',
  Sound: 'Choose the completion chime, volume, and Windows alerts.',
  Appearance: 'Tune theme, accent pigment, density, and motion level.',
  Shortcuts: 'Control the timer from any Windows app.',
  Widgets: 'Pin lightweight timer or task panels to your desktop.',
  Data: 'Back up, restore, or locate your local SQLite database.',
  About: 'Local-first desktop workspace built with Rust and Tauri 2.',
};

export function SettingsView({
  state,
  draft,
  setDraft,
}: {
  state: OrbitState;
  draft: Settings;
  setDraft: (value: Settings) => void;
}) {
  const data = state.snapshot!;
  const [group, setGroup] = useWorkspaceState('settings.category', 'Sessions');
  const [restore, setRestore] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [capture, setCapture] = useState<keyof Settings | null>(null);
  const draftRef = useRef(draft);
  draftRef.current = draft;

  const dirty = JSON.stringify(draft) !== JSON.stringify(data.settings);
  const set = <K extends keyof Settings>(key: K, value: Settings[K]) =>
    setDraft({ ...draft, [key]: value });

  useEffect(() => {
    if (!capture) return;
    let cancelled = false;
    void adapter()
      .invoke('capture_shortcuts', { capturing: true })
      .catch(e => {
        state.setError(normalizeError(e));
        setCapture(null);
      });
    const record = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === 'Escape') {
        setCapture(null);
        return;
      }
      if (['Control', 'Alt', 'Shift', 'Meta'].includes(e.key) || e.repeat) return;
      const key = e.key === ' ' ? 'Space' : e.key.length === 1 ? e.key.toUpperCase() : e.key;
      const value = [
        e.ctrlKey && 'Ctrl',
        e.altKey && 'Alt',
        e.shiftKey && 'Shift',
        e.metaKey && 'Win',
        key,
      ]
        .filter(Boolean)
        .join('+');
      if (!cancelled) {
        setDraft({ ...draftRef.current, [capture]: value });
        setCapture(null);
      }
    };
    window.addEventListener('keydown', record, true);
    return () => {
      cancelled = true;
      window.removeEventListener('keydown', record, true);
      void adapter()
        .invoke('capture_shortcuts', { capturing: false })
        .catch(e => state.setError(normalizeError(e)));
    };
    // Draft is read via ref so recording stays active across keystrokes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [capture]);

  const invalidDurations = [draft.focus_minutes, draft.short_minutes, draft.long_minutes].some(
    v => !Number.isInteger(v) || v < 1 || v > 180,
  );
  const invalidInterval =
    !Number.isInteger(draft.long_interval) || draft.long_interval < 1 || draft.long_interval > 12;

  async function save() {
    if (invalidDurations || invalidInterval) return;
    const submitted = normalizeSettings(draft);
    try {
      await state.run({ type: 'save_settings', settings: submitted });
      state.setNotice('Settings saved. New durations apply to your next session.');
    } catch {
      /* Retain draft on rejected saves. */
    }
  }

  async function native(action: string) {
    try {
      await windowAction(action);
    } catch (e) {
      state.setError(normalizeError(e));
    }
  }

  async function file(kind: number) {
    setBusy(true);
    try {
      const path = await chooseFile(kind);
      if (path) {
        if (kind === 1) setRestore(path);
        else state.setNotice('Backup created. The timer is paused.');
      }
      await state.refresh();
    } catch (e) {
      state.setError(normalizeError(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="settings-page">
      <nav className="settings-nav" aria-label="Settings categories">
        {groups.map(g => (
          <button
            key={g}
            className={group === g ? 'active' : ''}
            onClick={() => {
              setCapture(null);
              setGroup(g);
            }}
          >
            {g}
          </button>
        ))}
      </nav>

      <div className="settings-content">
        <div className="settings-intro">
          <span className="eyebrow">Preferences</span>
          <h2>{group}</h2>
          <p>{groupDescriptions[group]}</p>
        </div>

        {group === 'Sessions' && (
          <>
            <div className="duration-settings">
              {(
                [
                  ['focus_minutes', 'Focus', 'Primary deep-work block'],
                  ['short_minutes', 'Short break', 'Reset between sessions'],
                  ['long_minutes', 'Long break', 'Recharge after a full cycle'],
                ] as const
              ).map(([key, label, hint]) => (
                <label key={key}>
                  <span>{label}</span>
                  <div>
                    <input
                      aria-label={`${label} minutes`}
                      type="number"
                      min={1}
                      max={180}
                      value={draft[key]}
                      onChange={e => set(key, Number(e.target.value))}
                    />
                    <span>min</span>
                  </div>
                  <small>{hint}</small>
                  {(!Number.isInteger(draft[key]) || draft[key] < 1 || draft[key] > 180) && (
                    <span className="field-error" role="alert">
                      Enter 1–180 whole minutes.
                    </span>
                  )}
                </label>
              ))}
            </div>
            <SettingRow
              title="Long break interval"
              description="Completed focus sessions before a long break."
            >
              <input
                className="small-number"
                aria-label="Long break interval"
                type="number"
                min={1}
                max={12}
                value={draft.long_interval}
                onChange={e => set('long_interval', Number(e.target.value))}
              />
            </SettingRow>
            {invalidInterval && (
              <p className="field-error" role="alert">
                Choose 1–12 sessions before a long break.
              </p>
            )}
            <SettingRow
              title="Automatically start breaks"
              description="Begin a break immediately when a focus session finishes."
            >
              <Toggle
                label="Automatically start breaks"
                value={draft.auto_breaks}
                change={v => set('auto_breaks', v)}
              />
            </SettingRow>
            <SettingRow
              title="Automatically start focus"
              description="Start the next focus session as soon as a break ends."
            >
              <Toggle
                label="Automatically start focus"
                value={draft.auto_focus}
                change={v => set('auto_focus', v)}
              />
            </SettingRow>
          </>
        )}

        {group === 'Sound' && (
          <>
            <SettingRow
              title="Session sound"
              description="Tone played when a focus block or break ends."
            >
              <select
                aria-label="Session sound"
                value={draft.sound}
                onChange={e => set('sound', Number(e.target.value))}
              >
                <option value={0}>Soft bell</option>
                <option value={1}>Warm chime</option>
                <option value={2}>Gentle pulse</option>
              </select>
            </SettingRow>
            <SettingRow title="Volume" description={`${draft.volume}% output level`}>
              <input
                aria-label="Alarm volume"
                type="range"
                min={0}
                max={100}
                value={draft.volume}
                onChange={e => set('volume', Number(e.target.value))}
              />
            </SettingRow>
            <Button onClick={() => void native('preview_sound')}>
              <IconSpeaker size={15} />
              Preview saved sound
            </Button>
            <SettingRow
              title="Windows notifications"
              description="Send a native Windows toast when a session finishes."
            >
              <Toggle
                label="Windows notifications"
                value={draft.notifications}
                change={v => set('notifications', v)}
              />
            </SettingRow>
            <Button onClick={() => void native('register_notifications')}>
              Register notifications for this executable
            </Button>
            <p className="muted">
              Register again if you move the portable folder. Save changes before previewing sound.
            </p>
          </>
        )}

        {group === 'Appearance' && (
          <>
            <div className="theme-options">
              {(
                [
                  ['System', 0],
                  ['Light', 1],
                  ['Dark', 2],
                ] as const
              ).map(([label, value]) => (
                <button
                  key={label}
                  aria-pressed={draft.appearance === value}
                  className={`theme-choice ${label.toLowerCase()} ${draft.appearance === value ? 'selected' : ''}`}
                  onClick={() => set('appearance', value)}
                >
                  <span className="theme-preview">
                    <i />
                    <span>
                      <i />
                      <i />
                      <i />
                    </span>
                  </span>
                  <span>
                    {label}
                    {draft.appearance === value && <IconCheck size={13} />}
                  </span>
                </button>
              ))}
            </div>
            <SettingRow
              title="Accent color"
              description="Applied to the timer arc, primary controls, and focus indicators."
            >
              <div className="accent-options" role="group" aria-label="Accent color">
                {(
                  [
                    ['coral', 'Coral'],
                    ['sage', 'Sage'],
                    ['amber', 'Amber'],
                    ['slate_blue', 'Slate Blue'],
                  ] as const
                ).map(([value, label]) => (
                  <button
                    key={value}
                    className={`accent-choice accent-${value}`}
                    aria-label={label}
                    aria-pressed={(draft.accent ?? 'coral') === value}
                    onClick={() => set('accent', value)}
                  >
                    <i />
                    {label}
                    {(draft.accent ?? 'coral') === value && <IconCheck size={13} />}
                  </button>
                ))}
              </div>
            </SettingRow>
            <SettingRow
              title="Density"
              description="Adjust task row height and control spacing."
            >
              <select
                aria-label="Interface density"
                value={draft.density ?? 'comfortable'}
                onChange={e => set('density', e.target.value as Settings['density'])}
              >
                <option value="comfortable">Comfortable</option>
                <option value="compact">Compact</option>
              </select>
            </SettingRow>
            <SettingRow
              title="Animations"
              description="Standard uses crisp instrument transitions. Reduced keeps brief feedback. Off switches states immediately."
            >
              <select
                aria-label="Animation level"
                value={motionPreference(draft)}
                onChange={e =>
                  setDraft({
                    ...draft,
                    motion_level: e.target.value as Settings['motion_level'],
                    reduced_motion: e.target.value !== 'standard',
                  })
                }
              >
                <option value="standard">Standard</option>
                <option value="reduced">Reduced</option>
                <option value="off">Off</option>
              </select>
            </SettingRow>
            <SettingRow
              title="Windows backdrop"
              description="Use Windows Mica behind the main window when supported."
            >
              <Toggle
                label="Windows backdrop"
                value={draft.mica}
                change={v => set('mica', v)}
              />
            </SettingRow>
            <p className="preview-note">
              Previewing in this window. Save to apply to desktop widgets. Windows accessibility
              settings take precedence.
            </p>
            <Button
              onClick={() =>
                setDraft({
                  ...draft,
                  appearance: 0,
                  accent: 'coral',
                  density: 'comfortable',
                  motion_level: 'standard',
                  reduced_motion: false,
                  mica: false,
                })
              }
            >
              <IconReset size={15} />
              Reset appearance
            </Button>
          </>
        )}

        {group === 'Shortcuts' && (
          <>
            <SettingRow
              title="Open Orbit at sign-in"
              description="Launch silently in the tray with zero browser memory until opened."
            >
              <Toggle
                label="Open Orbit at sign-in"
                value={draft.autostart}
                change={v => set('autostart', v)}
              />
            </SettingRow>
            {(
              [
                ['hotkey_toggle', 'Start or pause'],
                ['hotkey_show', 'Open Orbit'],
                ['hotkey_skip', 'Skip session'],
              ] as const
            ).map(([key, label]) => (
              <SettingRow
                key={key}
                title={label}
                description={
                  key === 'hotkey_skip'
                    ? 'Optional. Disabled by default to prevent accidental skips.'
                    : 'Active globally across Windows.'
                }
              >
                <div className="shortcut-controls">
                  <button
                    className={`shortcut-recorder ${capture === key ? 'recording' : ''}`}
                    onClick={() => setCapture(capture === key ? null : key)}
                  >
                    <IconKeyboard size={15} />
                    {capture === key ? 'Press a shortcut…' : draft[key] || 'Record shortcut'}
                  </button>
                  {draft[key] && (
                    <button className="text-button" onClick={() => set(key, '')}>
                      Disable
                    </button>
                  )}
                </div>
              </SettingRow>
            ))}
            <p className="muted">Press Escape to cancel recording. Changes apply after Save.</p>
          </>
        )}

        {group === 'Widgets' && (
          <div className="widget-options">
            {data.widgets.map((preferences, index) => (
              <div className="widget-option" key={index}>
                <div className={`widget-preview preview-${index}`} aria-hidden="true">
                  {index === 0 ? (
                    <span className="mini-ring">25:00</span>
                  ) : index === 1 ? (
                    <span className="mini-task-lines">
                      <i />
                      <i />
                      <i />
                    </span>
                  ) : (
                    <span className="mini-timer">
                      25:00 <span>▶</span>
                    </span>
                  )}
                </div>
                <SettingRow
                  title={['Timer ring', 'Today’s tasks', 'Mini timer'][index]}
                  description={
                    [
                      'Compact circular chronometer with start/pause controls.',
                      'Your planned list for today pinned to the desktop.',
                      'Low-profile bar showing remaining time and play/pause.',
                    ][index]
                  }
                >
                  <Toggle
                    label={`Show ${['timer ring', 'today tasks', 'mini timer'][index]}`}
                    value={preferences.enabled}
                    change={enabled =>
                      state.act({
                        type: 'widget',
                        index,
                        preferences: { ...preferences, enabled },
                      })
                    }
                  />
                </SettingRow>
                <details>
                  <summary>Advanced controls</summary>
                  <SettingRow title="Window layer">
                    <select
                      aria-label={`${index} window layer`}
                      value={preferences.layer}
                      onChange={e =>
                        state.act({
                          type: 'widget',
                          index,
                          preferences: { ...preferences, layer: Number(e.target.value) },
                        })
                      }
                    >
                      <option value={0}>Desktop background</option>
                      <option value={1}>Normal</option>
                      <option value={2}>Always on top</option>
                    </select>
                  </SettingRow>
                  <SettingRow
                    title="Click-through"
                    description="Use Unlock all widgets in the tray to regain interaction."
                  >
                    <Toggle
                      label={`${index} click-through`}
                      value={preferences.click_through}
                      change={click_through =>
                        state.act({
                          type: 'widget',
                          index,
                          preferences: { ...preferences, click_through },
                        })
                      }
                    />
                  </SettingRow>
                  <Button
                    onClick={() =>
                      state.act({
                        type: 'widget',
                        index,
                        preferences: { ...preferences, x: null, y: null, click_through: false },
                      })
                    }
                  >
                    <IconReset size={14} />
                    Reset position &amp; unlock
                  </Button>
                </details>
              </div>
            ))}
          </div>
        )}

        {group === 'Data' && (
          <>
            <div className="data-location">
              <span className="eyebrow">Database location</span>
              <code>{data.data_location}</code>
              <Button onClick={() => void native('data_folder')}>
                <IconFolder size={15} />
                Open data folder
              </Button>
            </div>
            <SettingRow
              title="Create a backup"
              description="Saves a consistent copy of tasks, history, settings, and paused timer state."
            >
              <Button disabled={busy || data.storage_blocked} onClick={() => void file(0)}>
                <IconArrowDown size={15} />
                Back up
              </Button>
            </SettingRow>
            <SettingRow
              title="Restore a backup"
              description="Replace local data with a verified backup. A recovery copy is created first."
            >
              <Button disabled={busy || data.storage_blocked} onClick={() => void file(1)}>
                <IconArrowUp size={15} />
                Choose backup
              </Button>
            </SettingRow>
            {data.last_backup && (
              <p className="backup-path">
                Last backup <code>{data.last_backup}</code>
              </p>
            )}
            {restore && (
              <div className="restore-confirm" role="alert">
                <h3>Replace your local data?</h3>
                <p>
                  Tasks, session history, settings, and timer state will be replaced. Your current
                  database is saved as a recovery copy first.
                </p>
                <code>{restore}</code>
                <div className="button-pair">
                  <Button
                    primary
                    disabled={busy}
                    onClick={() => {
                      setBusy(true);
                      void adapter()
                        .invoke('confirm_restore', { confirm: true })
                        .then(async () => {
                          setRestore(null);
                          state.setNotice('Backup restored. Your timer is paused.');
                          await state.refresh();
                          const fresh = await command({ type: 'snapshot' });
                          if (fresh.type === 'snapshot') setDraft({ ...fresh.value.settings });
                        })
                        .catch(e => {
                          setRestore(null);
                          state.setError(normalizeError(e));
                        })
                        .finally(() => setBusy(false));
                    }}
                  >
                    Restore backup
                  </Button>
                  <Button
                    disabled={busy}
                    onClick={() => {
                      void adapter().invoke('confirm_restore', { confirm: false });
                      setRestore(null);
                    }}
                  >
                    Cancel
                  </Button>
                </div>
              </div>
            )}
            <p className="muted">
              CSV exports are reports, not restorable backups. Uninstalling Orbit keeps your data.
            </p>
          </>
        )}

        {group === 'About' && (
          <div className="about-content">
            <Brand />
            <p className="about-version">Version 0.3.1</p>
            <h3>Offline focus timer and task planner for Windows.</h3>
            <p>
              Built with Rust, SQLite, Tauri 2, and static Next.js. No account, no network calls,
              and no background service.
            </p>
            <p>
              Orbit is MIT licensed. Dependency and font notices are shipped in
              THIRD-PARTY-NOTICES.txt beside the executable.
            </p>
            <Button onClick={() => void native('data_folder')}>
              <IconFolder size={15} />
              Open your data folder
            </Button>
          </div>
        )}
      </div>

      {!['Widgets', 'Data', 'About'].includes(group) && (
        <div className="settings-save">
          <span className={dirty ? 'draft-label' : 'muted'}>
            {dirty ? 'You have unsaved changes' : 'All changes saved'}
          </span>
          <div className="button-pair">
            <Button
              disabled={!dirty || !!state.pending}
              onClick={() => setDraft({ ...data.settings })}
            >
              Discard
            </Button>
            <Button
              primary
              disabled={
                !dirty || !!state.pending || data.storage_blocked || invalidDurations || invalidInterval
              }
              onClick={() => void save()}
            >
              {state.pending ? 'Saving…' : 'Save changes'}
            </Button>
          </div>
        </div>
      )}
    </section>
  );
}

function SettingRow({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="setting-row">
      <div>
        <h3>{title}</h3>
        {description && <p>{description}</p>}
      </div>
      <div className="setting-control">{children}</div>
    </div>
  );
}

function Toggle({
  label,
  value,
  change,
}: {
  label: string;
  value: boolean;
  change: (value: boolean) => void;
}) {
  return (
    <button
      className={`toggle ${value ? 'on' : ''}`}
      role="switch"
      aria-label={label}
      aria-checked={value}
      onClick={() => change(!value)}
    >
      <span />
    </button>
  );
}
