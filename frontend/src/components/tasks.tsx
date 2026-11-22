'use client';
import { useContext, useEffect, useMemo, useRef, useState } from 'react';
import type { OrbitState } from '@/lib/use-orbit';
import {
  duration,
  filterTasks,
  SUGGESTED_TAGS,
  tagTone,
  type Task,
  type TaskAction,
} from '@/lib/protocol';
import { useWorkspaceState } from '@/lib/use-workspace-state';
import { useExperience, WorkspaceContext } from '@/lib/experience';
import {
  IconArrowDown,
  IconArrowRight,
  IconArrowUp,
  IconArrowUpRight,
  IconCalendar,
  IconCheck,
  IconClose,
  IconDots,
  IconEdit,
  IconPlus,
  IconReset,
  IconSearch,
  IconTag,
  IconTrash,
} from './icons';
import { Button, Empty, IconButton, PanelHeading, Sheet } from './ui';
import { TaskFocusAction } from './timer';

export function TasksView({
  state,
  compact = false,
  onViewAll,
}: {
  state: OrbitState;
  compact?: boolean;
  onViewAll?: () => void;
}) {
  const data = state.snapshot!;
  const [view, setView] = useWorkspaceState('tasks.view', 'today');
  const [search, setSearch] = useWorkspaceState('tasks.search', '');
  const [tagFilter, setTagFilter] = useWorkspaceState('tasks.tag', '');
  const [title, setTitle] = useState('');
  const [estimate, setEstimate] = useState(1);
  const [startDate, setStartDate] = useState('');
  const [endDate, setEndDate] = useState('');
  const [tag, setTag] = useState('');
  const [adding, setAdding] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [undo, setUndo] = useState<string | null>(null);
  const [acceptedTitle, setAcceptedTitle] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);

  const availableTags = useMemo(() => {
    const set = new Set<string>();
    for (const t of data.tasks) {
      if (!t.deleted && t.tag) set.add(t.tag);
    }
    return [...set].sort((a, b) => a.localeCompare(b));
  }, [data.tasks]);

  const activeTagFilter = !compact && availableTags.includes(tagFilter) ? tagFilter : '';

  const rows = useMemo(
    () =>
      filterTasks(
        data.tasks,
        compact ? 'today' : view,
        data.today,
        compact ? '' : search,
        activeTagFilter,
      ),
    [data.tasks, compact, view, data.today, search, activeTagFilter],
  );
  const titleError = [...title.trim()].length > 240 ? 'Use 240 characters or fewer.' : '';
  const estimateError =
    !Number.isInteger(estimate) || estimate < 1 || estimate > 99 ? 'Enter 1–99 sessions.' : '';
  const tagError = [...tag.trim()].length > 32 ? 'Use 32 characters or fewer for a tag.' : '';
  const dateError =
    startDate && endDate && endDate < startDate
      ? 'End date cannot be earlier than start date.'
      : '';
  const todayCount = useMemo(
    () => filterTasks(data.tasks, 'today', data.today).length,
    [data.tasks, data.today],
  );
  const completed = useMemo(
    () => data.tasks.filter(t => !t.deleted && t.done && t.planned_for === data.today).length,
    [data.tasks, data.today],
  );
  const carried = useMemo(
    () => filterTasks(data.tasks, 'carry', data.today).length,
    [data.tasks, data.today],
  );
  const inspected = data.tasks.find(t => t.id === selected);

  useEffect(() => {
    if (!undo) return;
    const id = setTimeout(() => setUndo(null), 10000);
    return () => clearTimeout(id);
  }, [undo]);

  useEffect(() => {
    if (acceptedTitle !== null && !data.storage_blocked) {
      setTitle(prev => (prev === acceptedTitle ? '' : prev));
      setAcceptedTitle(null);
    }
  }, [acceptedTitle, data.storage_blocked]);

  async function add(start: boolean) {
    if (
      !title.trim() ||
      [...title.trim()].length > 240 ||
      adding ||
      estimateError ||
      dateError ||
      tagError
    ) {
      return;
    }
    setAdding(true);
    const submitted = title;
    try {
      await state.run({
        type: 'create_task',
        title,
        estimate,
        today: compact || view === 'today',
        start,
        start_date: startDate || null,
        end_date: endDate || null,
        tag: (tag.trim() || activeTagFilter).slice(0, 32),
      });
      setTitle(prev => (prev === submitted ? '' : prev));
      setEstimate(1);
      setStartDate('');
      setEndDate('');
      setTag('');
      input.current?.focus();
    } catch (e) {
      if (e && typeof e === 'object' && 'accepted' in e && e.accepted) {
        setAcceptedTitle(submitted);
      }
    } finally {
      setAdding(false);
    }
  }

  async function action(task: Task, taskAction: TaskAction) {
    try {
      await state.run({ type: 'task', id: task.id, action: taskAction });
      if (taskAction === 'delete') {
        setUndo(task.id);
        setSelected(null);
      }
    } catch {
      /* Shared error toast owns recovery. */
    }
  }

  const canSubmit =
    Boolean(title.trim()) &&
    !titleError &&
    !estimateError &&
    !dateError &&
    !tagError &&
    !adding &&
    !data.storage_blocked;

  return (
    <section
      className={`tasks-surface ${compact ? 'today-panel' : 'tasks-page'}`}
      aria-label={compact ? 'Today’s plan' : 'Task planning'}
    >
      {compact ? (
        <PanelHeading
          eyebrow="Daily queue"
          title="Today’s plan"
          action={
            <IconButton label="Add a task" onClick={() => input.current?.focus()}>
              <IconPlus size={17} />
            </IconButton>
          }
        />
      ) : (
        <div className="task-toolbar">
          <div className="filter-tabs" aria-label="Task views">
            {(
              [
                ['today', 'Today'],
                ['all', 'All active'],
                ['completed', 'Completed'],
                ['trash', 'Trash'],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                className={view === id ? 'active' : ''}
                aria-pressed={view === id}
                onClick={() => setView(id)}
              >
                {label}
                {id === 'today' && <span className="tab-count">{todayCount}</span>}
              </button>
            ))}
          </div>
          <label className="search-field">
            <IconSearch size={15} />
            <input
              aria-label="Search tasks"
              placeholder="Search tasks…"
              value={search}
              onChange={e => setSearch(e.target.value)}
            />
            {search && (
              <IconButton label="Clear search" onClick={() => setSearch('')}>
                <IconClose size={13} />
              </IconButton>
            )}
          </label>
        </div>
      )}

      {compact && (
        <div className="today-summary">
          <span>
            <strong className="mono-meta">{rows.length}</strong> planned <i>·</i>{' '}
            <strong className="mono-meta">{completed}</strong> completed
          </span>
          <span className="day-marker">
            {new Date(`${data.today}T12:00:00`).toLocaleDateString('en', {
              weekday: 'short',
              day: 'numeric',
            })}
          </span>
        </div>
      )}

      {!compact && availableTags.length > 0 && (
        <div className="tag-filter-bar" role="group" aria-label="Filter by tag">
          <IconTag size={13} className="tag-filter-icon" />
          <button
            type="button"
            className={`tag-filter-chip ${!activeTagFilter ? 'active' : ''}`}
            aria-pressed={!activeTagFilter}
            onClick={() => setTagFilter('')}
          >
            All tags
          </button>
          {availableTags.map(t => (
            <button
              key={t}
              type="button"
              className={`tag-filter-chip ${activeTagFilter === t ? 'active' : ''}`}
              data-tone={tagTone(t)}
              aria-pressed={activeTagFilter === t}
              onClick={() => setTagFilter(activeTagFilter === t ? '' : t)}
            >
              {t}
            </button>
          ))}
        </div>
      )}

      {!compact && carried > 0 && (
        <button
          className={`carry-forward ${view === 'carry' ? 'active' : ''}`}
          onClick={() => setView(view === 'carry' ? 'today' : 'carry')}
        >
          <IconCalendar size={16} />
          <span>
            {carried} unfinished {carried === 1 ? 'task' : 'tasks'} from earlier days
          </span>
          <span className="carry-cta">
            Review <IconArrowRight size={14} />
          </span>
        </button>
      )}

      {(!compact && view === 'trash') || view === 'completed' ? null : (
        <form
          className={`quick-add ${compact ? 'compact-add' : ''}`}
          onSubmit={e => {
            e.preventDefault();
            void add(false);
          }}
        >
          <IconPlus size={16} className="quick-add-icon" />
          <input
            ref={input}
            aria-label="Task title"
            placeholder={compact ? 'Add a task for today…' : 'Add a task…'}
            value={title}
            onChange={e => setTitle(e.target.value)}
            maxLength={480}
            disabled={adding || data.storage_blocked}
          />
          {!compact && (
            <>
              <label className="tag-input-field">
                <IconTag size={13} />
                <input
                  aria-label="Task tag"
                  placeholder="Tag…"
                  list="orbit-tag-suggestions"
                  maxLength={32}
                  value={tag}
                  onChange={e => setTag(e.target.value)}
                  disabled={adding || data.storage_blocked}
                />
                <datalist id="orbit-tag-suggestions">
                  {Array.from(new Set([...availableTags, ...SUGGESTED_TAGS])).map(t => (
                    <option key={t} value={t} />
                  ))}
                </datalist>
              </label>
              <label className="estimate-input">
                <input
                  aria-label="Estimated sessions"
                  type="number"
                  min={1}
                  max={99}
                  value={estimate}
                  onChange={e => setEstimate(Number(e.target.value))}
                />
                <span>sessions</span>
              </label>
              <div className="quick-add-dates">
                <label className="date-input">
                  <span>Start</span>
                  <input
                    aria-label="New task start date"
                    type="date"
                    value={startDate}
                    onChange={e => setStartDate(e.target.value)}
                    disabled={adding || data.storage_blocked}
                  />
                </label>
                <label className="date-input">
                  <span>End</span>
                  <input
                    aria-label="New task end date"
                    type="date"
                    min={startDate || undefined}
                    value={endDate}
                    onChange={e => setEndDate(e.target.value)}
                    disabled={adding || data.storage_blocked}
                  />
                </label>
              </div>
            </>
          )}
          <button
            type="submit"
            className="add-submit"
            aria-label="Add task"
            disabled={!canSubmit}
          >
            {adding ? 'Saving…' : compact ? <IconArrowRight size={15} /> : 'Add task'}
          </button>
          {!compact && (
            <button
              type="button"
              className="add-and-focus"
              disabled={!canSubmit}
              onClick={() => void add(true)}
            >
              Add &amp; focus
            </button>
          )}
        </form>
      )}

      {(titleError || estimateError || dateError || tagError) && (
        <p className="field-error" role="alert">
          {titleError || estimateError || dateError || tagError}
        </p>
      )}

      {rows.length > 0 ? (
        <VirtualTasks
          memoryKey={compact ? 'today.scroll' : 'tasks.scroll'}
          rows={rows}
          render={(task, index) => (
            <div
              className={`task-row ${task.id === state.timer?.selected_task ? 'is-selected' : ''} ${task.done ? 'is-done' : ''}`}
            >
              <button
                className={`task-check ${task.done ? 'checked' : ''}`}
                aria-label={`${task.done ? 'Reopen' : 'Complete'} ${task.title}`}
                disabled={data.storage_blocked || !!state.pending || task.deleted}
                onClick={() => void action(task, 'toggle_done')}
              >
                {task.done ? (
                  <IconCheck size={12} />
                ) : (
                  <span className="task-number">{String(index + 1).padStart(2, '0')}</span>
                )}
              </button>
              <button
                className="task-description"
                onClick={() => setSelected(task.id)}
                title={task.title}
              >
                <span className="task-title-line">
                  <span>{task.title}</span>
                  {task.tag && (
                    <span className="tag-pill" data-tone={tagTone(task.tag)} aria-hidden="true">
                      {task.tag}
                    </span>
                  )}
                </span>
                <small>
                  <span className="mono-meta">
                    {task.completed} / {task.estimate} sessions
                  </span>
                  {task.planned_for && task.planned_for !== data.today
                    ? ` · ${task.planned_for}`
                    : ''}
                  <span className="task-dates-meta" aria-hidden="true">
                    {task.start_date || task.end_date
                      ? ` · ${task.start_date || '—'} → ${task.end_date || '—'}`
                      : !compact
                        ? ' · Dates not captured'
                        : ''}
                  </span>
                  <span className="task-progress-dots" aria-hidden="true">
                    {Array.from({ length: Math.min(task.estimate, 5) }, (_, i) => (
                      <i key={i} className={i < task.completed ? 'filled' : ''} />
                    ))}
                  </span>
                </small>
              </button>
              {!compact && !task.deleted && (!task.start_date || !task.end_date) && (
                <button
                  type="button"
                  className="add-dates-action"
                  aria-label={`Add dates for ${task.title}`}
                  onClick={() => setSelected(task.id)}
                >
                  <IconCalendar size={13} />
                  <span>Add dates</span>
                </button>
              )}
              {!task.done && !task.deleted && <TaskFocusAction state={state} task={task} />}
              {task.deleted ? (
                <IconButton
                  label={`Restore ${task.title}`}
                  disabled={data.storage_blocked || !!state.pending}
                  onClick={() => void action(task, 'restore')}
                >
                  <IconReset size={16} />
                </IconButton>
              ) : (
                <IconButton label={`Edit ${task.title}`} onClick={() => setSelected(task.id)}>
                  <IconDots size={18} />
                </IconButton>
              )}
            </div>
          )}
        />
      ) : (
        <Empty
          title={
            search
              ? 'No matching tasks'
              : view === 'trash'
                ? 'Trash is empty'
                : view === 'completed'
                  ? 'No completed tasks yet'
                  : view === 'carry'
                    ? 'Nothing carried over'
                    : compact
                      ? 'A fresh page for today.'
                      : 'No tasks scheduled'
          }
        >
          {search
            ? 'Try another search term or clear the filter.'
            : view === 'trash'
              ? 'Deleted tasks stay here until you restore them.'
              : view === 'completed'
                ? 'Finished tasks appear here with their session history.'
                : 'Add a task above to plan your next focus session.'}
        </Empty>
      )}

      {undo && (
        <div className="undo-notice" role="status">
          <span>Task moved to Trash</span>
          <button
            onClick={() => {
              const task = data.tasks.find(t => t.id === undo);
              if (task) void action(task, 'restore');
              setUndo(null);
            }}
          >
            Undo
          </button>
        </div>
      )}

      {compact && (
        <>
          <button className="view-all" onClick={onViewAll}>
            <span>See all your tasks</span>
            <IconArrowUpRight size={14} />
          </button>
          <div className="today-totals">
            <div>
              <span className="eyebrow">Focused today</span>
              <strong>{duration(data.today_ms)}</strong>
            </div>
            <div>
              <span className="eyebrow">Sessions finished</span>
              <strong>
                {String(data.today_completed).padStart(2, '0')}
                <span className="total-suffix"> / today</span>
              </strong>
            </div>
          </div>
        </>
      )}

      {inspected && (
        <TaskInspector
          key={inspected.id}
          task={inspected}
          state={state}
          action={action}
          close={() => setSelected(null)}
        />
      )}
    </section>
  );
}

function VirtualTasks({
  rows,
  render,
  memoryKey,
}: {
  rows: Task[];
  render: (task: Task, index: number) => React.ReactNode;
  memoryKey: string;
}) {
  const memory = useContext(WorkspaceContext);
  const { density } = useExperience();
  const rowHeight = density === 'compact' ? 68 : 84;
  const [scroll, setScroll] = useState(() => Number(memory.values.get(memoryKey) ?? 0));
  const root = useRef<HTMLDivElement>(null);
  const [focusTarget, setFocusTarget] = useState<number | null>(null);

  useEffect(() => {
    if (!root.current) return;
    const maximum = Math.max(0, rows.length * rowHeight - root.current.clientHeight);
    const restored = Math.min(Number(memory.values.get(memoryKey) ?? 0), maximum);
    root.current.scrollTop = restored;
    setScroll(restored);
  }, [rows.length, rowHeight, memory, memoryKey]);

  useEffect(() => {
    if (focusTarget !== null) {
      root.current
        ?.querySelector<HTMLElement>(`[data-row="${focusTarget}"] .task-description`)
        ?.focus();
    }
  }, [focusTarget, scroll]);

  const start = Math.max(0, Math.floor(scroll / rowHeight) - 3);
  const end = Math.min(rows.length, start + 22);

  return (
    <div
      className="task-list"
      ref={root}
      tabIndex={0}
      onKeyDown={e => {
        if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(e.key)) return;
        const current = Number(
          (e.target as HTMLElement).closest('[data-row]')?.getAttribute('data-row') ?? -1,
        );
        const next =
          e.key === 'Home'
            ? 0
            : e.key === 'End'
              ? rows.length - 1
              : Math.max(0, Math.min(rows.length - 1, current + (e.key === 'ArrowDown' ? 1 : -1)));
        e.preventDefault();
        if (root.current) {
          const top = next * rowHeight;
          if (
            top < root.current.scrollTop ||
            top + rowHeight > root.current.scrollTop + root.current.clientHeight
          ) {
            root.current.scrollTop = top;
          }
          setScroll(root.current.scrollTop);
        }
        setFocusTarget(next);
      }}
      onScroll={e => {
        const value = e.currentTarget.scrollTop;
        memory.values.set(memoryKey, value);
        setScroll(value);
      }}
      role="list"
      aria-label="Tasks"
    >
      <div style={{ height: rows.length * rowHeight, position: 'relative' }}>
        {rows.slice(start, end).map((task, i) => (
          <div
            role="listitem"
            data-row={start + i}
            aria-posinset={start + i + 1}
            aria-setsize={rows.length}
            key={task.id}
            style={{
              position: 'absolute',
              top: (start + i) * rowHeight,
              width: '100%',
              height: rowHeight,
            }}
          >
            {render(task, start + i)}
          </div>
        ))}
      </div>
    </div>
  );
}

function TaskInspector({
  task,
  state,
  action,
  close,
}: {
  task: Task;
  state: OrbitState;
  action: (task: Task, action: TaskAction) => Promise<void>;
  close: () => void;
}) {
  const [title, setTitle] = useState(task.title);
  const [estimate, setEstimate] = useState(task.estimate);
  const [startDate, setStartDate] = useState(task.start_date ?? '');
  const [endDate, setEndDate] = useState(task.end_date ?? '');
  const [tag, setTag] = useState(task.tag ?? '');

  useEffect(() => {
    setStartDate(task.start_date ?? '');
    setEndDate(task.end_date ?? '');
    setTag(task.tag ?? '');
  }, [task.start_date, task.end_date, task.tag]);

  const existingTags = useMemo(() => {
    const set = new Set<string>(SUGGESTED_TAGS);
    for (const t of state.snapshot?.tasks ?? []) {
      if (!t.deleted && t.tag) set.add(t.tag);
    }
    return [...set];
  }, [state.snapshot?.tasks]);

  const dateError =
    startDate && endDate && endDate < startDate
      ? 'End date cannot be earlier than start date.'
      : '';
  const invalidTitle = [...title.trim()].length > 240 || !title.trim();
  const invalidEstimate = !Number.isInteger(estimate) || estimate < 1 || estimate > 99;
  const invalidTag = [...tag.trim()].length > 32;

  return (
    <Sheet title="Task details" close={close}>
      <span className="eyebrow">Properties</span>
      <form
        className="detail-form"
        onSubmit={e => {
          e.preventDefault();
          if (dateError || invalidTitle || invalidEstimate || invalidTag) return;
          void state
            .run({
              type: 'edit_task',
              id: task.id,
              title,
              estimate,
              start_date: startDate || null,
              end_date: endDate || null,
              tag: tag.trim(),
            })
            .then(() => state.setNotice('Task updated.'))
            .catch(() => {});
        }}
      >
        <label>
          Title
          <textarea
            value={title}
            maxLength={480}
            onChange={e => setTitle(e.target.value)}
            required
          />
        </label>
        <label>
          Estimated focus sessions
          <input
            type="number"
            min={1}
            max={99}
            value={estimate}
            onChange={e => setEstimate(Number(e.target.value))}
          />
        </label>
        <label>
          Tag / category
          <input
            aria-label="Edit task tag"
            placeholder="e.g. Deep Work, Client, Study"
            maxLength={32}
            value={tag}
            onChange={e => setTag(e.target.value)}
          />
        </label>
        <div className="detail-tag-chips" role="group" aria-label="Quick tags">
          {existingTags.slice(0, 8).map(t => (
            <button
              key={t}
              type="button"
              className={`tag-filter-chip ${tag.trim() === t ? 'active' : ''}`}
              data-tone={tagTone(t)}
              onClick={() => setTag(tag.trim() === t ? '' : t)}
            >
              {t}
            </button>
          ))}
        </div>
        <div className="detail-dates">
          <label>
            <span>
              Start date {!startDate && <small className="date-untracked">(not captured)</small>}
            </span>
            <input
              aria-label="Start date"
              type="date"
              value={startDate}
              onChange={e => setStartDate(e.target.value)}
            />
          </label>
          <label>
            <span>
              End date {!endDate && <small className="date-untracked">(not captured)</small>}
            </span>
            <input
              aria-label="End date"
              type="date"
              min={startDate || undefined}
              value={endDate}
              onChange={e => setEndDate(e.target.value)}
            />
          </label>
        </div>
        {(!startDate || !endDate) && (
          <button
            type="button"
            className="text-button fill-today-dates"
            onClick={() => {
              const today = state.snapshot?.today ?? '';
              if (!startDate) setStartDate(today);
              if (!endDate) setEndDate(today);
            }}
          >
            Use today ({state.snapshot?.today}) for missing dates
          </button>
        )}
        {invalidTitle && (
          <p className="field-error" role="alert">
            Enter a title of 1–240 characters.
          </p>
        )}
        {invalidEstimate && (
          <p className="field-error" role="alert">
            Enter 1–99 sessions.
          </p>
        )}
        {dateError && (
          <p className="field-error" role="alert">
            {dateError}
          </p>
        )}
        <p className="muted">
          {task.completed} completed sessions recorded on this task.
        </p>
        <Button
          primary
          disabled={
            state.snapshot?.storage_blocked ||
            !!state.pending ||
            invalidTitle ||
            invalidEstimate ||
            !!dateError
          }
        >
          <IconEdit size={15} />
          Save changes
        </Button>
      </form>
      <div className="detail-actions">
        {!task.deleted && (
          <>
            <Button
              onClick={() =>
                void action(
                  task,
                  task.planned_for === state.snapshot?.today ? 'unplan' : 'plan',
                )
              }
            >
              <IconCalendar size={15} />
              {task.planned_for === state.snapshot?.today ? 'Remove from today' : 'Plan for today'}
            </Button>
            <div className="button-pair">
              <Button onClick={() => void action(task, 'up')}>
                <IconArrowUp size={15} />
                Move up
              </Button>
              <Button onClick={() => void action(task, 'down')}>
                <IconArrowDown size={15} />
                Move down
              </Button>
            </div>
            <Button onClick={() => void action(task, 'toggle_done')}>
              <IconCheck size={15} />
              {task.done ? 'Reopen task' : 'Mark complete'}
            </Button>
          </>
        )}
        <Button
          className={task.deleted ? '' : 'danger'}
          onClick={() => void action(task, task.deleted ? 'restore' : 'delete')}
        >
          {task.deleted ? <IconReset size={15} /> : <IconTrash size={15} />}
          {task.deleted ? 'Restore task' : 'Move to Trash'}
        </Button>
      </div>
    </Sheet>
  );
}
