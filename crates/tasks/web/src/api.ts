import type { Connection, Task, TaskWithDependencies, TasksStats } from './types.js';

const SVC = 'tasks';

export const listTasks = (c: Connection, params?: { status?: string }) =>
  c.request<Task[]>(SVC, 'list', params ?? {});

export const createTask = (c: Connection, params: { title: string; description?: string; depends_on?: number[] }) =>
  c.request<Task>(SVC, 'create', params);

export const getTask = (c: Connection, taskId: number) =>
  c.request<TaskWithDependencies>(SVC, 'get', { task_id: taskId });

export const updateTask = (c: Connection, params: { task_id: number; title?: string; description?: string; status?: string }) =>
  c.request<Task>(SVC, 'update', params);

export const deleteTask = (c: Connection, taskId: number) =>
  c.request<{ deleted: boolean }>(SVC, 'delete', { task_id: taskId });

export const searchTasks = (c: Connection, query: string, limit?: number) =>
  c.request<Task[]>(SVC, 'search', { query, limit });

export const getReady = (c: Connection) =>
  c.request<Task[]>(SVC, 'ready', {});

export const getBlocked = (c: Connection) =>
  c.request<Task[]>(SVC, 'blocked', {});

export const getStats = (c: Connection) =>
  c.request<TasksStats>(SVC, 'stats', {});

export const addDependency = (c: Connection, fromTaskId: number, toTaskId: number) =>
  c.request<{ from_task_id: number; to_task_id: number }>(SVC, 'add_dependency', { from_task_id: fromTaskId, to_task_id: toTaskId });

export const removeDependency = (c: Connection, fromTaskId: number, toTaskId: number) =>
  c.request<{ removed: boolean }>(SVC, 'remove_dependency', { from_task_id: fromTaskId, to_task_id: toTaskId });

export const detectCycles = (c: Connection) =>
  c.request<{ cycles: number[][] }>(SVC, 'detect_cycles', {});
