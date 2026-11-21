'use client';
import { useEffect, useRef, useState } from 'react';
import { command, chooseFile, normalizeError } from '@/lib/bridge';
import { defaultQuery, duration, type Report, type ReportQuery } from '@/lib/protocol';
import type { OrbitState } from '@/lib/use-orbit';
import { useWorkspaceState } from '@/lib/use-workspace-state';
import {
  IconArrowDown,
  IconArrowLeft,
  IconArrowRight,
  IconArrowUpRight,
  IconCalendar,
} from './icons';
import { Button, Empty, IconButton, Sheet } from './ui';

export function ReportsView({ state }: { state: OrbitState }) {
  const [query, setQuery] = useWorkspaceState<ReportQuery>('reports.query', defaultQuery);
  const [draft, setDraft] = useWorkspaceState('reports.dates', { start: '', end: '' });
  const [report, setReport] = useState<Report | null>(null);
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
