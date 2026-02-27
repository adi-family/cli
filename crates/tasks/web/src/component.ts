import { LitElement } from 'lit';
import { state } from 'lit/decorators.js';
import type { Task, TasksStats, TaskWithDependencies, TaskStatus, Connection } from './types.js';
import { renderTaskList } from './views/task-list.js';
import { renderTaskDetail } from './views/task-detail.js';
import { renderTaskForm } from './views/task-form.js';

declare global {
  interface Window {
    sdk: {
      bus: import('@adi-family/sdk-plugin').EventBus;
      getConnections(): Map<string, Connection>;
    };
  }
}

type View = 'list' | 'detail' | 'create';

export class AdiTasksElement extends LitElement {
  @state() private tasks: Task[] = [];
  @state() private stats: TasksStats | null = null;
  @state() private selectedTask: TaskWithDependencies | null = null;
  @state() private filter: TaskStatus | undefined = undefined;
  @state() private searchQuery = '';
  @state() private view: View = 'list';
  @state() private loading = false;
  @state() private submitting = false;
  @state() private confirmingDelete = false;
  @state() private error: string | null = null;

  override createRenderRoot() { return this; }

  override connectedCallback(): void {
    super.connectedCallback();
    this.loadData();
  }

  private get bus() { return window.sdk.bus; }

  private async loadData(): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      if (this.searchQuery.trim()) {
        this.stats = null;
        const result = await this.bus.send('tasks:search', { query: this.searchQuery }, 'tasks-ui').wait();
        this.tasks = result.tasks;
      } else {
        const result = await this.bus.send('tasks:list', { status: this.filter }, 'tasks-ui').wait();
        this.tasks = result.tasks;
        this.stats = result.stats;
      }
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Failed to load tasks';
    } finally {
      this.loading = false;
    }
  }

  private async loadDetail(task: Task): Promise<void> {
    this.loading = true;
    try {
      const result = await this.bus.send('tasks:get', { task_id: task.id, cocoonId: task.cocoonId }, 'tasks-ui').wait();
      this.selectedTask = result.task;
      this.view = 'detail';
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Failed to load task';
    } finally {
      this.loading = false;
    }
  }

  private async handleStatusChange(task: Task, status: TaskStatus): Promise<void> {
    try {
      const result = await this.bus.send('tasks:update', { task_id: task.id, cocoonId: task.cocoonId, status }, 'tasks-ui').wait();
      this.tasks = this.tasks.map(t =>
        t.id === task.id && t.cocoonId === task.cocoonId ? result.task : t
      );
      if (this.selectedTask?.task.id === task.id) {
        this.selectedTask = { ...this.selectedTask, task: result.task };
      }
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Failed to update task';
    }
  }

  private async handleDelete(task: Task): Promise<void> {
    if (!this.confirmingDelete) { this.confirmingDelete = true; return; }
    this.submitting = true;
    try {
      await this.bus.send('tasks:delete', { task_id: task.id, cocoonId: task.cocoonId }, 'tasks-ui').wait();
      this.tasks = this.tasks.filter(t => !(t.id === task.id && t.cocoonId === task.cocoonId));
      this.view = 'list';
      this.confirmingDelete = false;
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Failed to delete task';
      this.confirmingDelete = false;
    } finally {
      this.submitting = false;
    }
  }

  private async handleCreate(data: { title: string; description?: string; cocoonId: string }): Promise<void> {
    this.submitting = true;
    try {
      const result = await this.bus.send('tasks:create', data, 'tasks-ui').wait();
      this.tasks = [...this.tasks, result.task];
      this.view = 'list';
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Failed to create task';
    } finally {
      this.submitting = false;
    }
  }

  private handleFilterChange(status: TaskStatus | undefined): void {
    this.filter = status;
    this.loadData();
  }

  private handleSearch(query: string): void {
    this.searchQuery = query;
    this.loadData();
  }

  override render() {
    const connections: Connection[] = [...window.sdk.getConnections().values()];

    if (this.view === 'detail' && this.selectedTask) {
      return renderTaskDetail({
        task: this.selectedTask,
        submitting: this.submitting,
        confirmingDelete: this.confirmingDelete,
        onBack: () => { this.view = 'list'; this.selectedTask = null; this.confirmingDelete = false; },
        onCancelDelete: () => { this.confirmingDelete = false; },
        onStatusChange: (status) => this.handleStatusChange(this.selectedTask!.task, status),
        onDelete: () => this.handleDelete(this.selectedTask!.task),
        onNavigate: (task) => this.loadDetail(task),
      });
    }

    if (this.view === 'create') {
      return renderTaskForm({
        connections,
        submitting: this.submitting,
        onBack: () => { this.view = 'list'; },
        onCreate: (data) => this.handleCreate(data),
      });
    }

    return renderTaskList({
      tasks: this.tasks,
      stats: this.stats,
      filter: this.filter,
      searchQuery: this.searchQuery,
      loading: this.loading,
      error: this.error,
      onSelectTask: (task) => this.loadDetail(task),
      onFilterChange: (status) => this.handleFilterChange(status),
      onSearch: (query) => this.handleSearch(query),
      onNewTask: () => { this.view = 'create'; },
    });
  }
}
