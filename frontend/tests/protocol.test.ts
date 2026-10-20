import { describe, expect, it } from 'vitest';
import { filterTasks, duration, timeLabel, type AppCommand, type Task } from '../src/lib/protocol';
const task: Task = {id: '9223372036854775807', title: 'Plan café research', estimate: 3, completed: 1, done: false, deleted: false, planned_for: '2026-10-20', today_order: 1};
describe('presentation contracts', () => {
  it('keeps IDs as strings in a typed wire command', () => {const c: AppCommand = {type: 'start_task', id: task.id}; expect(JSON.parse(JSON.stringify(c)).id).toBe('9223372036854775807');});
  it('filters Today, carry forward, completed, and Trash independently', () => {
    const tasks = [task, {...task, id: '2', planned_for: '2026-10-19'}, {...task, id: '3', done: true}, {...task, id: '4', deleted: true}];
    for (const [view, id] of [['today', task.id], ['carry', '2'], ['completed', '3'], ['trash', '4']]) expect(filterTasks(tasks, view, '2026-10-20').map(t => t.id)).toEqual([id]);
    expect(filterTasks(tasks, 'all', '2026-10-20', 'CAFÉ')).toHaveLength(2);
  });
  it('rounds the countdown upward and does not discard partial focus', () => {expect(timeLabel(1)).toBe('00:01'); expect(timeLabel(60001)).toBe('01:01'); expect(duration(1234)).toBe('<1m'); expect(duration(5_040_000)).toBe('1h 24m');});
  it('handles 5,000 tasks without changing IDs or planning membership', () => {const tasks = Array.from({length: 5000}, (_, i) => ({...task, id: String(i), title: `Task ${i}`})); expect(filterTasks(tasks, 'today', '2026-10-20', 'Task 4999').map(t => t.id)).toEqual(['4999']);});
});
