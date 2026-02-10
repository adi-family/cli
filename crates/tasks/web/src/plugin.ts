import { LitElement, html, type TemplateResult } from "lit";
import { state } from "lit/decorators.js";
import type { CocoonClient, Task, TaskStatus, TaskWithDependencies, TasksStats } from "./types";
import { listTasks, getStats, getTask, createTask, updateTask, deleteTask, searchTasks } from "./api";
import { renderTaskList } from "./views/task-list";
import { renderTaskDetail } from "./views/task-detail";
import { renderTaskForm } from "./views/task-form";

type View = "list" | "detail" | "create";

export class TasksPlugin extends LitElement {
  static id = "adi.tasks";
  static services = ["tasks"];

  cocoons: ReadonlyMap<string, CocoonClient> = new Map();

  @state() private tasks: Task[] = [];
  @state() private stats: TasksStats | null = null;
  @state() private selectedTask: TaskWithDependencies | null = null;
  @state() private filter: TaskStatus | null = null;
  @state() private searchQuery = "";
  @state() private view: View = "list";
  @state() private loading = false;
  @state() private submitting = false;
  @state() private confirmingDelete = false;
  @state() private error: string | null = null;

  private get client(): CocoonClient | null {
    const first = this.cocoons.values().next();
    return first.done ? null : first.value;
  }

  createRenderRoot() {
    return this;
  }

  onCocoonConnected(_cocoonId: string, _services: string[]) {
    this.loadData();
  }

  onCocoonDisconnected(_cocoonId: string) {
    if (this.cocoons.size === 0) {
      this.tasks = [];
      this.stats = null;
      this.selectedTask = null;
      this.view = "list";
    }
  }

  private async loadData() {
    const c = this.client;
    if (!c) return;
    this.loading = true;
    this.error = null;
    try {
      const [tasks, stats] = await Promise.all([
        this.searchQuery
          ? searchTasks(c, this.searchQuery)
          : listTasks(c, this.filter ? { status: this.filter } : undefined),
        getStats(c),
      ]);
      this.tasks = tasks;
      this.stats = stats;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }

  private async loadTaskDetail(taskId: number) {
    const c = this.client;
    if (!c) return;
    this.loading = true;
    this.confirmingDelete = false;
    try {
      this.selectedTask = await getTask(c, taskId);
      this.view = "detail";
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }

  private async handleStatusChange(taskId: number, status: TaskStatus) {
    const c = this.client;
    if (!c) return;
    try {
      await updateTask(c, { task_id: taskId, status });
      await this.loadTaskDetail(taskId);
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    }
  }

  private async handleDelete(taskId: number) {
    if (taskId === -1) {
      this.confirmingDelete = true;
      return;
    }
    const c = this.client;
    if (!c) return;
    try {
      await deleteTask(c, taskId);
      this.view = "list";
      this.selectedTask = null;
      this.confirmingDelete = false;
      await this.loadData();
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    }
  }

  private async handleCreate(title: string, description: string) {
    const c = this.client;
    if (!c) return;
    this.submitting = true;
    try {
      await createTask(c, { title, description: description || undefined });
      this.view = "list";
      await this.loadData();
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.submitting = false;
    }
  }

  private handleFilterChange(status: TaskStatus | null) {
    this.filter = status;
    this.searchQuery = "";
    this.loadData();
  }

  private searchTimeout: ReturnType<typeof setTimeout> | null = null;

  private handleSearch(query: string) {
    this.searchQuery = query;
    if (this.searchTimeout) clearTimeout(this.searchTimeout);
    this.searchTimeout = setTimeout(() => this.loadData(), 300);
  }

  render(): TemplateResult {
    if (this.cocoons.size === 0) {
      return html`
        <div class="text-center py-12 text-gray-500 text-sm">
          <p class="mb-1">No cocoon connected</p>
          <p class="text-xs text-gray-600">Connect a cocoon with the tasks service to manage your tasks</p>
        </div>
      `;
    }

    if (this.error) {
      return html`
        <div class="space-y-3">
          <div class="p-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-300">
            ${this.error}
          </div>
          <button
            class="text-sm text-gray-400 hover:text-gray-200 transition-colors"
            @click=${() => { this.error = null; this.loadData(); }}
          >
            Dismiss &amp; retry
          </button>
        </div>
      `;
    }

    switch (this.view) {
      case "detail":
        return renderTaskDetail(this.selectedTask, this.loading, this.confirmingDelete, {
          onBack: () => { this.view = "list"; this.confirmingDelete = false; this.loadData(); },
          onStatusChange: (id, s) => this.handleStatusChange(id, s),
          onDelete: (id) => this.handleDelete(id),
          onSelectTask: (id) => this.loadTaskDetail(id),
        });

      case "create":
        return renderTaskForm(this.submitting, {
          onSubmit: (title, desc) => this.handleCreate(title, desc),
          onCancel: () => { this.view = "list"; },
        });

      default:
        return renderTaskList(this.tasks, this.stats, this.filter, this.searchQuery, this.loading, {
          onSelectTask: (id) => this.loadTaskDetail(id),
          onCreateTask: () => { this.view = "create"; },
          onFilterChange: (s) => this.handleFilterChange(s),
          onSearch: (q) => this.handleSearch(q),
        });
    }
  }
}
