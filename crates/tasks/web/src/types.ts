export type TaskStatus = "todo" | "in_progress" | "done" | "blocked" | "cancelled";

export interface Task {
  id: number;
  title: string;
  description: string | null;
  status: TaskStatus;
  symbol_id: number | null;
  project_path: string | null;
  created_at: number;
  updated_at: number;
}

export interface TaskWithDependencies {
  task: Task;
  depends_on: Task[];
  dependents: Task[];
}

export interface TasksStats {
  total_tasks: number;
  todo_count: number;
  in_progress_count: number;
  done_count: number;
  blocked_count: number;
  cancelled_count: number;
  total_dependencies: number;
  has_cycles: boolean;
}

export interface CocoonClient {
  id: string;
  services: string[];
  request<T = unknown>(service: string, method: string, params?: unknown): Promise<T>;
  stream<T = unknown>(service: string, method: string, params?: unknown): AsyncGenerator<T>;
  httpProxy(service: string, path: string, init?: RequestInit): Promise<Response>;
  httpDirect(url: string, init?: RequestInit): Promise<Response>;
}
