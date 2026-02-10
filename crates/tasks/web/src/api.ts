import type { CocoonClient, Task, TaskWithDependencies, TasksStats } from "./types";

const SVC = "tasks";

export const listTasks = (c: CocoonClient, params?: { status?: string }) =>
  c.request<Task[]>(SVC, "list", params ?? {});

export const createTask = (c: CocoonClient, params: { title: string; description?: string; depends_on?: number[] }) =>
  c.request<{ task_id: number }>(SVC, "create", params);

export const getTask = (c: CocoonClient, taskId: number) =>
  c.request<TaskWithDependencies>(SVC, "get", { task_id: taskId });

export const updateTask = (c: CocoonClient, params: { task_id: number; title?: string; description?: string; status?: string }) =>
  c.request<{ task_id: number }>(SVC, "update", params);

export const deleteTask = (c: CocoonClient, taskId: number) =>
  c.request<{ deleted: boolean }>(SVC, "delete", { task_id: taskId });

export const searchTasks = (c: CocoonClient, query: string, limit?: number) =>
  c.request<Task[]>(SVC, "search", { query, limit });

export const getReady = (c: CocoonClient) =>
  c.request<Task[]>(SVC, "ready", {});

export const getBlocked = (c: CocoonClient) =>
  c.request<Task[]>(SVC, "blocked", {});

export const getStats = (c: CocoonClient) =>
  c.request<TasksStats>(SVC, "stats", {});

export const addDependency = (c: CocoonClient, fromTaskId: number, toTaskId: number) =>
  c.request<{ from_task_id: number; to_task_id: number }>(SVC, "add_dependency", { from_task_id: fromTaskId, to_task_id: toTaskId });

export const removeDependency = (c: CocoonClient, fromTaskId: number, toTaskId: number) =>
  c.request<{ removed: boolean }>(SVC, "remove_dependency", { from_task_id: fromTaskId, to_task_id: toTaskId });

export const detectCycles = (c: CocoonClient) =>
  c.request<{ cycles: number[][] }>(SVC, "detect_cycles", {});
