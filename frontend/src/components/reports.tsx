'use client';
import { useEffect, useRef, useState } from 'react';
import { command, chooseFile, normalizeError } from '@/lib/bridge';
import {
  defaultQuery,
  duration,
  tagTone,
  type ActivityHistory,
  type Report,
  type ReportQuery,
} from '@/lib/protocol';
import type { OrbitState } from '@/lib/use-orbit';
import { useWorkspaceState } from '@/lib/use-workspace-state';
import {
  IconArrowDown,
  IconArrowLeft,
  IconArrowRight,
  IconArrowUpRight,
  IconCalendar,
  IconClock,
  IconFlame,
  IconTag,
} from './icons';
import { Button, Empty, IconButton, Sheet } from './ui';

export function ReportsView({ state }: { state: OrbitState }) {
  const [query, setQuery] = useWorkspaceState<ReportQuery>('reports.query', defaultQuery);
  const [draft, setDraft] = useWorkspaceState('reports.dates', { start: '', end: '' });
  const [report, setReport] = useState<Report | null>(null);
  const [activity, setActivity] = useState<ActivityHistory | null>(null);
  const [loading, setLoading] = useState(true);
  const [problem, setProblem] = useState('');
  const [details, setDetails] = useState<{ title: string; rows: string[] } | null>(null);
  const generation = useRef(0);
  const detailGeneration = useRef(0);
  const [reload, setReload] = useState(0);

  useEffect(() => () => { detailGeneration.current++; }, []);

  useEffect(() => {
    const id = ++generation.current;
    setLoading(true);
    setProblem('');
    command({ type: 'report', query })
      .then(reply => {
        if (id === generation.current && reply.type === 'report') setReport(reply.value);
      })
      .catch(e => {
        if (id === generation.current) setProblem(normalizeError(e).message);
      })
      .finally(() => {
        if (id === generation.current) setLoading(false);
      });
    return () => { generation.current++; };
  }, [query, state.reportRevision, reload]);

  const dailyGoal = state.snapshot?.settings.daily_goal_sessions ?? 4;

  useEffect(() => {
    let cancelled = false;
    command({ type: 'activity_history', days: 84 })
      .then(reply => {
        if (!cancelled && reply.type === 'activity_history') setActivity(reply.value);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [state.reportRevision, reload, dailyGoal]);

  async function detail(id: string | null, title: string) {
    const gen = ++detailGeneration.current;
    setDetails({ title, rows: ['Loading session history…'] });
    try {
      const reply = await command({ type: 'session_details', id, query });
      if (gen === detailGeneration.current && reply.type === 'details') {
        setDetails({ title, rows: reply.value });
      }
    } catch (e) {
      if (gen === detailGeneration.current) {
        setDetails({ title, rows: [normalizeError(e).message] });
      }
    }
  }

  async function exportCsv() {
    try {
      const file = await chooseFile(2, query);
      if (file) state.setNotice('Report exported. Your timer keeps its current state.');
    } catch (e) {
      state.setError(normalizeError(e));
    }
  }

  const max = Math.max(1, ...(report?.days.map(d => d.active_ms) || []));
  const hourBuckets = report?.hours ?? [];
  const maxHour = Math.max(1, ...hourBuckets.map(h => h.active_ms));
  const peakHour = hourBuckets.reduce<{ label: string; active_ms: number } | null>(
    (best, cur) => (cur.active_ms > (best?.active_ms ?? 0) ? cur : best),
    null,
  );
  const tagReports = report?.tags ?? [];

  return (
    <section className="reports-page">
      <div className="report-toolbar">
        <div className="filter-tabs" role="group" aria-label="Report period">
          {(['day', 'week', 'month'] as const).map(period => (
            <button
              key={period}
              aria-pressed={query.period === period && !query.start}
              className={query.period === period && !query.start ? 'active' : ''}
              onClick={() => setQuery({ ...defaultQuery, period })}
            >
              {period === 'day' ? 'Daily' : period === 'week' ? 'Weekly' : 'Monthly'}
            </button>
          ))}
        </div>
        <Button onClick={() => void exportCsv()}>
          <IconArrowDown size={15} />
          Export CSV
        </Button>
      </div>

      <div className="report-range">
        <div className="range-title">
          <IconButton
            label="Previous period"
            onClick={() => setQuery(q => ({ ...q, start: '', end: '', offset: q.offset - 1 }))}
          >
            <IconArrowLeft size={15} />
          </IconButton>
          <span>
            <IconCalendar size={15} />
            {report?.title || 'Focus history'}
          </span>
          <IconButton
            label="Next period"
            onClick={() => setQuery(q => ({ ...q, start: '', end: '', offset: q.offset + 1 }))}
          >
            <IconArrowRight size={15} />
          </IconButton>
          <button
            className="text-button"
            onClick={() => setQuery({ ...defaultQuery, period: query.period })}
          >
            Current period
          </button>
        </div>
        <details className="custom-range">
          <summary>Custom dates</summary>
          <form
            onSubmit={e => {
              e.preventDefault();
              setQuery(q => ({ ...q, ...draft }));
            }}
          >
            <label>
              From
              <input
                aria-label="Report start date"
                type="date"
                required
                value={draft.start}
                onChange={e => setDraft(d => ({ ...d, start: e.target.value }))}
              />
            </label>
            <label>
              Through
              <input
                aria-label="Report end date"
                type="date"
                required
                value={draft.end}
                min={draft.start}
                onChange={e => setDraft(d => ({ ...d, end: e.target.value }))}
              />
            </label>
            <Button primary>Apply range</Button>
          </form>
        </details>
      </div>

      {problem ? (
        <div className="inline-error" role="alert">
          <span>{problem}</span>
          <Button onClick={() => setReload(v => v + 1)}>Retry report</Button>
        </div>
      ) : loading && !report ? (
        <div className="skeleton report-skeleton" aria-label="Loading report" />
      ) : (
        report && (
          <div aria-busy={loading} className={loading ? 'is-refreshing' : ''}>
            <div className="report-metrics">
              <div className="metric-card">
                <span className="eyebrow">Focused time</span>
                <strong>{duration(report.active_ms)}</strong>
                <span>Total active focus recorded.</span>
              </div>
              <div className="metric-card">
                <span className="eyebrow">Completed sessions</span>
                <strong>{String(report.completed).padStart(2, '0')}</strong>
                <span>Full focus blocks finished.</span>
              </div>
              <div className="metric-card">
                <span className="eyebrow">Partial focus</span>
                <strong>{duration(report.partial_ms)}</strong>
                <span>Time kept from paused or skipped runs.</span>
              </div>
            </div>

            <div className="report-chart-card">
              <div className="chart-heading">
                <h2>Daily rhythm</h2>
                <span className="chart-legend">
                  <i />
                  Active focus
                </span>
              </div>
              <div
                className="focus-chart"
                role="img"
                aria-label={report.days
                  .map(d => `${d.label}: ${duration(d.active_ms)}`)
                  .join(', ')}
              >
                {report.days.map((day, i) => {
                  const ratio = day.active_ms / max;
                  return (
                    <div
                      className={`chart-column ${day.active_ms > 0 ? 'has-value' : ''}`}
                      key={`${day.label}-${i}`}
                    >
                      <span className="chart-value">
                        {day.active_ms ? duration(day.active_ms) : '—'}
                      </span>
                      <div className="chart-bar-space">
                        <div
                          className="chart-bar"
                          style={{
                            transform: `scaleY(${ratio})`,
                            transitionDelay: `${Math.min(i * 22, 180)}ms`,
                          }}
                        />
                      </div>
                      <span className="chart-label">{day.label}</span>
                    </div>
                  );
                })}
              </div>
            </div>

            {activity && (
              <div className="consistency-card">
                <div className="consistency-header">
                  <div>
                    <h2>Consistency</h2>
                    <span className="consistency-sub">Last 12 weeks · click any day to inspect</span>
                  </div>
                  <div className="streak-stats">
                    <div className="streak-stat">
                      <span className="eyebrow">
                        <IconFlame size={12} /> Current streak
                      </span>
                      <strong className="mono-meta">
                        {activity.current_streak}{' '}
                        <small>{activity.current_streak === 1 ? 'day' : 'days'}</small>
                      </strong>
                    </div>
                    <div className="streak-stat">
                      <span className="eyebrow">Best streak</span>
                      <strong className="mono-meta">
                        {activity.best_streak}{' '}
                        <small>{activity.best_streak === 1 ? 'day' : 'days'}</small>
                      </strong>
                    </div>
                    <div className="streak-stat">
                      <span className="eyebrow">Goal met</span>
                      <strong className="mono-meta">
                        {activity.goal_days} <small>/ {activity.days.length}d</small>
                      </strong>
                    </div>
                  </div>
                </div>
                <div
                  className="activity-heatmap"
                  role="group"
                  aria-label="12-week focus activity heatmap"
                >
                  {activity.days.map(day => {
                    const level =
                      day.completed >= dailyGoal
                        ? 4
                        : day.completed >= Math.max(1, Math.ceil(dailyGoal * 0.6))
                          ? 3
                          : day.completed >= 1
                            ? 2
                            : day.active_ms > 0
                              ? 1
                              : 0;
                    const label = `${day.date}: ${duration(day.active_ms)}, ${day.completed} ${
                      day.completed === 1 ? 'session' : 'sessions'
                    }`;
                    const selected = query.start === day.date && query.end === day.date;
                    return (
                      <button
                        key={day.date}
                        type="button"
                        className={`heatmap-cell level-${level} ${selected ? 'selected' : ''}`}
                        title={label}
                        aria-label={label}
                        onClick={() => {
                          setDraft({ start: day.date, end: day.date });
                          setQuery({ period: 'day', offset: 0, start: day.date, end: day.date });
                        }}
                      />
                    );
                  })}
                </div>
                <div className="heatmap-legend" aria-hidden="true">
                  <span>Less</span>
                  <i className="level-0" />
                  <i className="level-1" />
                  <i className="level-2" />
                  <i className="level-3" />
                  <i className="level-4" />
                  <span>Goal ({dailyGoal}+)</span>
                </div>
              </div>
            )}

            <div className="insights-secondary-grid">
              <div className="report-chart-card peak-hours-card">
                <div className="chart-heading">
                  <h2>
                    <IconClock size={15} /> Time of day
                  </h2>
                  <span className="mono-meta peak-badge">
                    {peakHour
                      ? `Peak: ${peakHour.label} · ${duration(peakHour.active_ms)}`
                      : 'No focus yet'}
                  </span>
                </div>
                <div
                  className="hour-chart"
                  role="img"
                  aria-label={
                    hourBuckets.length
                      ? hourBuckets.map(h => `${h.label}: ${duration(h.active_ms)}`).join(', ')
                      : 'No focus recorded by time of day'
                  }
                >
                  {hourBuckets.map(bucket => {
                    const ratio = bucket.active_ms / maxHour;
                    const isPeak = peakHour?.label === bucket.label && bucket.active_ms > 0;
                    return (
                      <div
                        key={bucket.label}
                        className={`hour-column ${bucket.active_ms > 0 ? 'has-value' : ''} ${isPeak ? 'is-peak' : ''}`}
                      >
                        <span className="chart-value">
                          {bucket.active_ms > 0 ? duration(bucket.active_ms) : '—'}
                        </span>
                        <div className="hour-bar-space">
                          <div
                            className="hour-bar"
                            style={{ transform: `scaleY(${ratio})` }}
                          />
                        </div>
                        <span className="chart-label">{bucket.label}</span>
                      </div>
                    );
                  })}
                </div>
              </div>

              <div className="report-chart-card tag-breakdown-card">
                <div className="chart-heading">
                  <h2>
                    <IconTag size={14} /> Focus by tag
                  </h2>
                  <span className="mono-meta">
                    {tagReports.length} {tagReports.length === 1 ? 'group' : 'groups'}
                  </span>
                </div>
                {tagReports.length > 0 ? (
                  <div className="tag-breakdown-list">
                    {tagReports.map(entry => {
                      const pct = Math.round(
                        (entry.active_ms / Math.max(report.active_ms, 1)) * 100,
                      );
                      return (
                        <div key={entry.tag} className="tag-breakdown-row">
                          <div className="tag-breakdown-head">
                            <span
                              className={`tag-pill ${entry.tag === 'Untagged' ? 'is-muted' : ''}`}
                              data-tone={
                                entry.tag === 'Untagged' ? undefined : tagTone(entry.tag)
                              }
                            >
                              {entry.tag}
                            </span>
                            <span className="mono-meta tag-breakdown-meta">
                              {duration(entry.active_ms)} · {pct}%
                            </span>
                          </div>
                          <span className="report-share">
                            <i
                              style={{
                                transform: `scaleX(${entry.active_ms / Math.max(report.active_ms, 1)})`,
                              }}
                            />
                          </span>
                        </div>
                      );
                    })}
                  </div>
                ) : (
                  <p className="muted tag-breakdown-empty">
                    Tag tasks to see how your focus distributes across projects.
                  </p>
                )}
              </div>
            </div>

            <div className="chart-heading">
              <h2>Focus by task</h2>
              <span className="mono-meta">
                {report.tasks.length} {report.tasks.length === 1 ? 'task' : 'tasks'}
              </span>
            </div>

            {report.tasks.length ? (
              <div className="report-task-list">
                {report.tasks.map((task, i) => (
                  <button
                    key={task.id ?? 'unassigned'}
                    className="report-task"
                    onClick={() => void detail(task.id, task.title)}
                  >
                    <span className="report-index">{String(i + 1).padStart(2, '0')}</span>
                    <span className="report-task-info">
                      <strong>
                        {task.title}
                        {task.tag && (
                          <span className="tag-pill" data-tone={tagTone(task.tag)}>
                            {task.tag}
                          </span>
                        )}
                        {task.deleted && <small> · Deleted</small>}
                        {task.done && !task.deleted && <small> · Completed</small>}
                        {(task.start_date || task.end_date) && (
                          <small>
                            {' '}
                            · {task.start_date || '—'} → {task.end_date || '—'}
                          </small>
                        )}
                      </strong>
                      <span className="report-share">
                        <i
                          style={{
                            transform: `scaleX(${task.active_ms / Math.max(report.active_ms, 1)})`,
                          }}
                        />
                      </span>
                    </span>
                    <span className="report-task-time">
                      {duration(task.active_ms)}
                      <small>{task.completed} sessions</small>
                    </span>
                    <IconArrowUpRight size={15} />
                  </button>
                ))}
              </div>
            ) : (
              <Empty title="No focus sessions in this range">
                Completed and partial focus sessions appear here once recorded.
              </Empty>
            )}
          </div>
        )
      )}

      {details && (
        <Sheet
          title={details.title}
          close={() => {
            detailGeneration.current++;
            setDetails(null);
          }}
        >
          <span className="eyebrow">Session history</span>
          <ul className="session-details">
            {details.rows.length ? (
              details.rows.map((row, i) => <li key={i}>{row}</li>)
            ) : (
              <li>No sessions in this range.</li>
            )}
          </ul>
        </Sheet>
      )}
    </section>
  );
}
