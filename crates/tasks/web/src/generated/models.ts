/**
 * Auto-generated models from TypeSpec.
 * DO NOT EDIT.
 */

import { TaskStatus } from './enums';

export interface Task {
  id: number;
  title: string;
  description?: string;
  status: TaskStatus;
  symbolId?: number;
  projectPath?: string;
  createdAt: number;
  updatedAt: number;
}

export interface TaskWithDependencies {
  task: Task;
  dependsOn: Task[];
  dependents: Task[];
}

export interface TasksStatus {
  totalTasks: number;
  todoCount: number;
  inProgressCount: number;
  doneCount: number;
  blockedCount: number;
  cancelledCount: number;
  totalDependencies: number;
  hasCycles: boolean;
}

export interface CreateTaskInput {
  title: string;
  description?: string;
  dependsOn?: number[];
  symbolId?: number;
}

export interface UpdateTaskInput {
  title?: string;
  description?: string;
  status?: string;
  symbolId?: number;
}

export interface UpdateStatusInput {
  status: string;
}

export interface AddDependencyInput {
  dependsOn: number;
}

export interface ListQuery {
  status?: string;
}

export interface SearchQuery {
  q: string;
  limit?: number;
}

export interface GraphNode {
  task: Task;
  dependencies: number[];
}

export interface IdResponse {
  id: number;
}

export interface DeletedResponse {
  deleted: number;
}

export interface DependencyResponse {
  from: number;
  to: number;
}

export interface RemovedResponse {
  removed: boolean;
}

export interface CyclesResponse {
  cycles: number[][];
}

export interface LinkResponse {
  taskId: number;
  symbolId: number;
}

export interface UnlinkResponse {
  taskId: number;
  unlinked: boolean;
}
