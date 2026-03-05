import { AdiPlugin } from '@adi-family/sdk-plugin';
import { AdiRouterBusKey } from '@adi/router-web-plugin/bus';
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

    this.bus.emit(AdiRouterBusKey.RegisterRoute, { pluginId: this.id, path: '', element: 'adi-tasks', label: 'Tasks' }, this.id);
    this.bus.emit('nav:add', { id: this.id, label: 'Tasks', path: `/${this.id}` }, this.id);

    this.bus.on('tasks:list', async ({ status }) => {
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
        const stats = statsResults.reduce<TasksStats>(
          (acc, r) => r.status === 'fulfilled' ? mergeStats(acc, r.value) : acc,
          emptyStats(),
        );
        this.bus.emit('tasks:list-changed', { tasks, stats }, 'tasks');
      } catch (err) {
        console.error('[TasksPlugin] tasks:list error:', err);
        this.bus.emit('tasks:list-changed', { tasks: [], stats: emptyStats() }, 'tasks');
      }
    }, 'tasks');

    this.bus.on('tasks:search', async ({ query, limit }) => {
      try {
        const conns = connectionsWithTasks();
        const results = await Promise.allSettled(conns.map(c => api.searchTasks(c, query, limit)));
        const tasks: Task[] = results.flatMap((r, i) =>
          r.status === 'fulfilled'
            ? r.value.map(t => ({ ...t, cocoonId: conns[i].id }))
            : []
        );
        this.bus.emit('tasks:search-changed', { tasks }, 'tasks');
      } catch (err) {
        console.error('[TasksPlugin] tasks:search error:', err);
        this.bus.emit('tasks:search-changed', { tasks: [] }, 'tasks');
      }
    }, 'tasks');

    this.bus.on('tasks:stats', async () => {
      try {
        const conns = connectionsWithTasks();
        const results = await Promise.allSettled(conns.map(c => api.getStats(c)));
        const stats = results.reduce<TasksStats>(
          (acc, r) => r.status === 'fulfilled' ? mergeStats(acc, r.value) : acc,
          emptyStats(),
        );
        this.bus.emit('tasks:stats-changed', { stats }, 'tasks');
      } catch (err) {
        console.error('[TasksPlugin] tasks:stats error:', err);
        this.bus.emit('tasks:stats-changed', { stats: emptyStats() }, 'tasks');
      }
    }, 'tasks');

    this.bus.on('tasks:get', async ({ task_id, cocoonId }) => {
      try {
        const raw = await api.getTask(getConnection(cocoonId), task_id);
        this.bus.emit('tasks:detail-changed', {
          task: {
            task: { ...raw.task, cocoonId },
            depends_on: raw.depends_on.map(t => ({ ...t, cocoonId })),
            dependents: raw.dependents.map(t => ({ ...t, cocoonId })),
          },
        }, 'tasks');
      } catch (err) {
        console.error('[TasksPlugin] tasks:get error:', err);
      }
    }, 'tasks');

    this.bus.on('tasks:create', async ({ cocoonId, title, description, depends_on }) => {
      try {
        const raw = await api.createTask(getConnection(cocoonId), { title, description, depends_on });
        this.bus.emit('tasks:task-mutated', { task: { ...raw, cocoonId } }, 'tasks');
      } catch (err) {
        console.error('[TasksPlugin] tasks:create error:', err);
      }
    }, 'tasks');

    this.bus.on('tasks:update', async ({ cocoonId, task_id, title, description, status }) => {
      try {
        const raw = await api.updateTask(getConnection(cocoonId), { task_id, title, description, status });
        this.bus.emit('tasks:task-mutated', { task: { ...raw, cocoonId } }, 'tasks');
      } catch (err) {
        console.error('[TasksPlugin] tasks:update error:', err);
      }
    }, 'tasks');

    this.bus.on('tasks:delete', async ({ cocoonId, task_id }) => {
      try {
        await api.deleteTask(getConnection(cocoonId), task_id);
        this.bus.emit('tasks:task-deleted', { task_id, cocoonId }, 'tasks');
      } catch (err) {
        console.error('[TasksPlugin] tasks:delete error:', err);
      }
    }, 'tasks');
  }
}
