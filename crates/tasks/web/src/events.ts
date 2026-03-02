import type { Task, TaskStatus, TasksStats, TaskWithDependencies } from './types.js';

declare module '@adi-family/sdk-plugin' {
  interface EventRegistry {
    'tasks:list':    { status?: TaskStatus };
    'tasks:search':  { query: string; limit?: number };
    'tasks:stats':   Record<string, never>;
    'tasks:get':     { task_id: number; cocoonId: string };
    'tasks:create':  { title: string; description?: string; cocoonId: string; depends_on?: number[] };
    'tasks:update':  { task_id: number; cocoonId: string; title?: string; description?: string; status?: TaskStatus };
    'tasks:delete':  { task_id: number; cocoonId: string };

    'tasks:list-changed':   { tasks: Task[]; stats: TasksStats };
    'tasks:search-changed': { tasks: Task[] };
    'tasks:detail-changed': { task: TaskWithDependencies };
    'tasks:task-mutated':   { task: Task };
    'tasks:task-deleted':   { task_id: number; cocoonId: string };
    'tasks:stats-changed':  { stats: TasksStats };
  }
}

export {};
