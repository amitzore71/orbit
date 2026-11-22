'use client';
import { useEffect, useMemo, useRef, useState } from 'react';
import {
  AMBIENT_PRESETS,
  FOCUS_PRESETS,
  normalizeSettings,
  type AmbientSound,
  type FocusPreset,
  type Mode,
} from '@/lib/protocol';
import type { OrbitState } from '@/lib/use-orbit';
import {
  IconExpand,
  IconFocus,
  IconInsights,
  IconMaximize,
  IconPause,
  IconPlay,
  IconPlus,
  IconReset,
  IconSearch,
  IconSettings,
  IconSkip,
  IconTasks,
  IconWaves,
} from './icons';

type Page = 'focus' | 'tasks' | 'reports' | 'settings';

interface PaletteItem {
  id: string;
  group: 'Timer' | 'Tasks' | 'Workspace' | 'Presets' | 'Soundscapes' | 'Widgets';
  label: string;
  hint?: string;
  shortcut?: string;
  icon:
    | 'play'
    | 'pause'
    | 'plus'
    | 'skip'
    | 'reset'
    | 'focus'
    | 'tasks'
    | 'insights'
    | 'settings'
    | 'zen'
    | 'maximize'
    | 'waves';
  run: () => void | Promise<void>;
}

const widgetNames = ['Timer ring', "Today's tasks", 'Mini timer'] as const;

export function CommandPalette({
  open,
  close,
  state,
  page,
  setPage,
  zen,
  toggleZen,
  maximized,
  toggleMaximize,
}: {
  open: boolean;
  close: () => void;
  state: OrbitState;
  page: Page;
  setPage: (p: Page) => void;
  zen: boolean;
  toggleZen: () => void;
  maximized: boolean;
  toggleMaximize: () => void;
}) {
  const [query, setQuery] = useState('');
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const data = state.snapshot;
  const timer = state.timer;

  useEffect(() => {
    if (!open) return;
    setQuery('');
    setActiveIndex(0);
    const id = requestAnimationFrame(() => inputRef.current?.focus());
    return () => cancelAnimationFrame(id);
  }, [open]);

  const items = useMemo<PaletteItem[]>(() => {
    if (!data || !timer) return [];
    const list: PaletteItem[] = [];
    const blocked = data.storage_blocked;

    const applyPreset = (preset: FocusPreset) => {
      void state
        .run({
          type: 'save_settings',
          settings: normalizeSettings({
            ...data.settings,
            focus_minutes: preset.focus_minutes,
            short_minutes: preset.short_minutes,
            long_minutes: preset.long_minutes,
          }),
        })
        .then(() =>
          state.setNotice(
            `Applied ${preset.name} (${preset.focus_minutes}m / ${preset.short_minutes}m).`,
          ),
        )
        .catch(() => {});
    };

    if (!blocked) {
      list.push({
        id: 'timer-toggle',
        group: 'Timer',
        label: timer.running
          ? 'Pause session'
          : timer.started
            ? 'Resume session'
            : 'Start focusing',
        shortcut: data.settings.hotkey_toggle || undefined,
        icon: timer.running ? 'pause' : 'play',
        run: () => state.act({ type: 'timer', action: 'toggle' }),
      });

      if (timer.started && timer.total_ms < 180 * 60_000) {
        list.push({
          id: 'timer-extend-5',
          group: 'Timer',
          label: 'Extend active session by 5 minutes',
          hint: '+5m',
          icon: 'plus',
          run: () => state.act({ type: 'extend_timer', minutes: 5 }),
        });
      }

      list.push(
        {
          id: 'timer-skip',
          group: 'Timer',
          label: 'Skip current session',
          icon: 'skip',
          run: () => state.act({ type: 'timer', action: 'skip' }),
        },
        {
          id: 'timer-reset',
          group: 'Timer',
          label: 'Reset current session',
          icon: 'reset',
          run: () => state.act({ type: 'timer', action: 'reset' }),
        },
      );

      (
        [
          ['Focus', 'Switch to Focus mode'],
          ['ShortBreak', 'Switch to Short break'],
          ['LongBreak', 'Switch to Long break'],
        ] as [Mode, string][]
      ).forEach(([mode, label]) => {
        if (timer.mode !== mode) {
          list.push({
            id: `mode-${mode}`,
            group: 'Timer',
            label,
            icon: 'focus',
            run: () => {
              state.act({ type: 'select_mode', mode });
              setPage('focus');
            },
          });
        }
      });
    }

    list.push(
      {
        id: 'zen-toggle',
        group: 'Workspace',
        label: zen ? 'Exit Zen focus mode' : 'Enter Zen focus mode',
        shortcut: 'Ctrl+Shift+F',
        icon: 'zen',
        run: toggleZen,
      },
      {
        id: 'window-maximize',
        group: 'Workspace',
        label: maximized ? 'Restore window size' : 'Maximize window',
        icon: 'maximize',
        run: toggleMaximize,
      },
      ...(
        [
          ['focus', 'Go to Focus', 'Alt+1', 'focus'],
          ['tasks', 'Go to Tasks', 'Alt+2', 'tasks'],
          ['reports', 'Go to Insights', 'Alt+3', 'insights'],
          ['settings', 'Go to Settings', 'Alt+4', 'settings'],
        ] as const
      )
        .filter(([id]) => id !== page)
        .map(([id, label, shortcut, icon]) => ({
          id: `nav-${id}`,
          group: 'Workspace' as const,
          label,
          shortcut,
          icon,
          run: () => setPage(id),
        })),
    );

    if (!blocked) {
      for (const preset of FOCUS_PRESETS) {
        list.push({
          id: `preset-${preset.id}`,
          group: 'Presets',
          label: `Preset: ${preset.name}`,
          hint: `${preset.focus_minutes}m focus · ${preset.short_minutes}m break`,
          icon: 'focus',
          run: () => applyPreset(preset),
        });
      }

      const currentAmbient: AmbientSound = data.settings.ambient_sound ?? 'off';
      for (const sound of AMBIENT_PRESETS) {
        if (sound.id === currentAmbient) continue;
        list.push({
          id: `ambient-${sound.id}`,
          group: 'Soundscapes',
          label: `Soundscape: ${sound.name}`,
          hint: sound.hint,
          icon: 'waves',
          run: () => {
            void state
              .run({
                type: 'save_settings',
                settings: normalizeSettings({
                  ...data.settings,
                  ambient_sound: sound.id,
                }),
              })
              .then(() =>
                state.setNotice(
                  sound.id === 'off'
                    ? 'Ambient soundscape turned off.'
                    : `Ambient soundscape set to ${sound.name}.`,
                ),
              )
              .catch(() => {});
          },
        });
      }
    }

    data.widgets.forEach((w, index) => {
      list.push({
        id: `widget-${index}`,
        group: 'Widgets',
        label: `${w.enabled ? 'Hide' : 'Show'} ${widgetNames[index]} widget`,
        icon: 'maximize',
        run: () =>
          state.act({
            type: 'widget',
            index,
            preferences: { ...w, enabled: !w.enabled },
          }),
      });
    });

    const activeTasks = data.tasks.filter(t => !t.deleted && !t.done);
    for (const task of activeTasks.slice(0, 80)) {
      const tagPrefix = task.tag ? `#${task.tag} · ` : '';
      list.push({
        id: `task-${task.id}`,
        group: 'Tasks',
        label: task.title,
        hint: `${tagPrefix}${task.completed}/${task.estimate} sessions · ${!timer.started && timer.mode === 'Focus' ? 'Start focus' : 'Queue next'}`,
        icon: 'tasks',
        run: () => {
          state.act({ type: 'start_task', id: task.id });
          setPage('focus');
        },
      });
    }

    return list;
  }, [data, timer, page, zen, maximized, state, setPage, toggleZen, toggleMaximize]);

  const filtered = useMemo(() => {
    const q = query.trim().toLocaleLowerCase();
    const matched = q
      ? items.filter(
          item =>
            item.label.toLocaleLowerCase().includes(q) ||
            item.group.toLocaleLowerCase().includes(q) ||
            (item.hint && item.hint.toLocaleLowerCase().includes(q)),
        )
      : items.slice(0, 14);

    if (q && data && !data.storage_blocked && [...query.trim()].length <= 240) {
      const exactTask = matched.some(
        m => m.group === 'Tasks' && m.label.toLocaleLowerCase() === q,
      );
      if (!exactTask) {
        matched.push({
          id: 'quick-create-focus',
          group: 'Tasks',
          label: `Create & focus “${query.trim()}”`,
          hint: 'Add to Today and start focusing',
          icon: 'plus',
          run: () => {
            state.act({
              type: 'create_task',
              title: query.trim(),
              estimate: 1,
              today: true,
              start: true,
            });
            setPage('focus');
          },
        });
      }
    }
    return matched.slice(0, 16);
  }, [items, query, data, state, setPage]);

  useEffect(() => {
    setActiveIndex(0);
  }, [query]);

  if (!open || !data || !timer) return null;

  const execute = (item: PaletteItem | undefined) => {
    if (!item) return;
    close();
    void item.run();
  };

  const renderIcon = (kind: PaletteItem['icon']) => {
    switch (kind) {
      case 'play':
        return <IconPlay size={14} />;
      case 'pause':
        return <IconPause size={14} />;
      case 'plus':
        return <IconPlus size={15} />;
      case 'skip':
        return <IconSkip size={15} />;
      case 'reset':
        return <IconReset size={15} />;
      case 'tasks':
        return <IconTasks size={15} />;
      case 'insights':
        return <IconInsights size={15} />;
      case 'settings':
        return <IconSettings size={15} />;
      case 'zen':
        return <IconExpand size={15} />;
      case 'maximize':
        return <IconMaximize size={14} />;
      case 'waves':
        return <IconWaves size={15} />;
      default:
        return <IconFocus size={15} />;
    }
  };

  return (
    <div
      className="dialog-scrim palette-scrim"
      onMouseDown={e => {
        if (e.target === e.currentTarget) close();
      }}
    >
      <section
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
        className="command-palette"
        onKeyDown={e => {
          if (e.key === 'Escape') {
            e.preventDefault();
            close();
          } else if (e.key === 'ArrowDown') {
            e.preventDefault();
            setActiveIndex(i => (filtered.length ? (i + 1) % filtered.length : 0));
          } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            setActiveIndex(i =>
              filtered.length ? (i - 1 + filtered.length) % filtered.length : 0,
            );
          } else if (e.key === 'Enter') {
            e.preventDefault();
            execute(filtered[activeIndex]);
          }
        }}
      >
        <div className="palette-input-row">
          <IconSearch size={16} />
          <input
            ref={inputRef}
            aria-label="Type a command or task"
            placeholder="Jump to a task, preset, or command…"
            value={query}
            onChange={e => setQuery(e.target.value)}
          />
          <kbd>Esc</kbd>
        </div>

        <div className="palette-list" role="listbox" aria-label="Commands">
          {filtered.length === 0 ? (
            <p className="palette-empty">No matching commands or tasks.</p>
          ) : (
            filtered.map((item, idx) => (
              <button
                key={item.id}
                type="button"
                role="option"
                aria-selected={idx === activeIndex}
                className={`palette-item ${idx === activeIndex ? 'is-active' : ''}`}
                onMouseEnter={() => setActiveIndex(idx)}
                onClick={() => execute(item)}
              >
                <span className="palette-item-icon">{renderIcon(item.icon)}</span>
                <span className="palette-item-body">
                  <strong>{item.label}</strong>
                  {item.hint && <small>{item.hint}</small>}
                </span>
                <span className="palette-item-meta">
                  <span className="palette-group">{item.group}</span>
                  {item.shortcut && <kbd>{item.shortcut}</kbd>}
                </span>
              </button>
            ))
          )}
        </div>
      </section>
    </div>
  );
}
