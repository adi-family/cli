import { AdiPlugin } from '@adi-family/sdk-plugin';
import type { WithCid } from '@adi-family/sdk-plugin';
import * as api from './api.js';
import type { Connection, Task, TasksStats } from './types.js';
import './events.js';

function connectionsWithTasks(): Connection[] {
  return [...window.sdk.getConnections().values()]
    .filter(c => c.services.includes('tasks'));
}

function getConnection(cocoonId: string): Connection {
  const c = window.sdk.getConnections().get(cocoonId);
  if (!c) throw new Error(`Connection '${cocoonId}' not found`);
  return c;
}

function emptyStats(): TasksStats {
  return {
    total_tasks: 0, todo_count: 0, in_progress_count: 0,
    done_count: 0, blocked_count: 0, cancelled_count: 0,
    total_dependencies: 0, has_cycles: false,
  };
}

function mergeStats(a: TasksStats, b: TasksStats): TasksStats {
  return {
    total_tasks:        a.total_tasks        + b.total_tasks,
    todo_count:         a.todo_count         + b.todo_count,
    in_progress_count:  a.in_progress_count  + b.in_progress_count,
    done_count:         a.done_count         + b.done_count,
    blocked_count:      a.blocked_count      + b.blocked_count,
    cancelled_count:    a.cancelled_count    + b.cancelled_count,
    total_dependencies: a.total_dependencies + b.total_dependencies,
    has_cycles:         a.has_cycles         || b.has_cycles,
  };
}

export class TasksPlugin extends AdiPlugin {
  readonly id = 'adi.tasks';
  readonly version = '0.1.0';

  async onRegister(): Promise<void> {
    const { AdiTasksElement } = await import('./component.js');
    if (!customElements.get('adi-tasks')) {
      customElements.define('adi-tasks', AdiTasksElement);
    }

    this.bus.emit('route:register', { path: '/tasks', element: 'adi-tasks' });
    this.bus.send('nav:add', { id: 'tasks', label: 'Tasks', path: '/tasks', icon: '✓' }).handle(() => {});

    this.bus.on('tasks:list', async (p) => {
      const { _cid, status } = p as WithCid<typeof p>;
      try {
        const conns = connectionsWithTasks();
        const [taskResults, statsResults] = await Promise.all([
          Promise.allSettled(conns.map(c => api.listTasks(c, { status }))),
          Promise.allSettled(conns.map(c => api.getStats(c))),
        ]);
        const tasks: Task[] = taskResults.flatMap((r, i) =>
          r.status === 'fulfilled'
            ? r.value.map(t => ({ ...t, cocoonId: conns[i].id }))
            : []
        );
        const stats = statsResults.reduce(
          (acc, r) => r.status === 'fulfilled' ? mergeStats(acc, r.value) : acc,
          emptyStats()
        );
        this.bus.emit('tasks:list:ok', { tasks, stats, _cid });
      } catch (err) {
        console.error('[TasksPlugin] tasks:list error:', err);
        this.bus.emit('tasks:list:ok', { tasks: [], stats: emptyStats(), _cid });
      }
    });

    this.bus.on('tasks:search', async (p) => {
      const { _cid, query, limit } = p as WithCid<typeof p>;
      try {
        const conns = connectionsWithTasks();
        const results = await Promise.allSettled(conns.map(c => api.searchTasks(c, query, limit)));
        const tasks: Task[] = results.flatMap((r, i) =>
          r.status === 'fulfilled'
            ? r.value.map(t => ({ ...t, cocoonId: conns[i].id }))
            : []
        );
        this.bus.emit('tasks:search:ok', { tasks, _cid });
      } catch (err) {
        console.error('[TasksPlugin] tasks:search error:', err);
        this.bus.emit('tasks:search:ok', { tasks: [], _cid });
      }
    });

    this.bus.on('tasks:stats', async (p) => {
      const { _cid } = p as WithCid<typeof p>;
      try {
        const conns = connectionsWithTasks();
        const results = await Promise.allSettled(conns.map(c => api.getStats(c)));
        const stats = results.reduce(
          (acc, r) => r.status === 'fulfilled' ? mergeStats(acc, r.value) : acc,
          emptyStats()
        );
        this.bus.emit('tasks:stats:ok', { stats, _cid });
      } catch (err) {
        console.error('[TasksPlugin] tasks:stats error:', err);
        this.bus.emit('tasks:stats:ok', { stats: emptyStats(), _cid });
      }
    });

    this.bus.on('tasks:get', async (p) => {
      const { _cid, task_id, cocoonId } = p as WithCid<typeof p>;
      try {
        const raw = await api.getTask(getConnection(cocoonId), task_id);
        const task = {
          ...raw,
          task: { ...raw.task, cocoonId },
          depends_on: raw.depends_on.map(t => ({ ...t, cocoonId })),
          dependents:  raw.dependents.map(t => ({ ...t, cocoonId })),
        };
        this.bus.emit('tasks:get:ok', { task, _cid });
      } catch (err) {
        console.error('[TasksPlugin] tasks:get error:', err);
      }
    });

    this.bus.on('tasks:create', async (p) => {
      const { _cid, cocoonId, title, description, depends_on } = p as WithCid<typeof p>;
      try {
        const raw = await api.createTask(getConnection(cocoonId), { title, description, depends_on });
        this.bus.emit('tasks:create:ok', { task: { ...raw, cocoonId }, _cid });
      } catch (err) {
        console.error('[TasksPlugin] tasks:create error:', err);
      }
    });

    this.bus.on('tasks:update', async (p) => {
      const { _cid, cocoonId, task_id, title, description, status } = p as WithCid<typeof p>;
      try {
        const raw = await api.updateTask(getConnection(cocoonId), { task_id, title, description, status });
        this.bus.emit('tasks:update:ok', { task: { ...raw, cocoonId }, _cid });
      } catch (err) {
        console.error('[TasksPlugin] tasks:update error:', err);
      }
    });

    this.bus.on('tasks:delete', async (p) => {
      const { _cid, cocoonId, task_id } = p as WithCid<typeof p>;
      try {
        await api.deleteTask(getConnection(cocoonId), task_id);
        this.bus.emit('tasks:delete:ok', { _cid });
      } catch (err) {
        console.error('[TasksPlugin] tasks:delete error:', err);
      }
    });
  }
}
