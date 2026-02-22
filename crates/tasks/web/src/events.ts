import type { Task, TaskWithDependencies, TasksStats, TaskStatus } from './types.js';

declare module '@adi-family/sdk-plugin' {
  interface EventRegistry {
    'tasks:list':       { status?: TaskStatus };
    'tasks:list:ok':    { tasks: Task[]; stats: TasksStats; _cid: string };

    'tasks:search':     { query: string; limit?: number };
    'tasks:search:ok':  { tasks: Task[]; _cid: string };

    'tasks:stats':      Record<string, never>;
    'tasks:stats:ok':   { stats: TasksStats; _cid: string };

    'tasks:get':        { task_id: number; cocoonId: string };
    'tasks:get:ok':     { task: TaskWithDependencies; _cid: string };

    'tasks:create':     { title: string; description?: string; cocoonId: string; depends_on?: number[] };
    'tasks:create:ok':  { task: Task; _cid: string };

    'tasks:update':     { task_id: number; cocoonId: string; title?: string; description?: string; status?: TaskStatus };
    'tasks:update:ok':  { task: Task; _cid: string };

    'tasks:delete':     { task_id: number; cocoonId: string };
    'tasks:delete:ok':  { _cid: string };
  }
}

export {};
