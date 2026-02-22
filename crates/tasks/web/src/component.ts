import { LitElement, html } from 'lit';
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

  private loadData(): void {
    this.loading = true;
    this.error = null;
    if (this.searchQuery.trim()) {
      this.bus.send('tasks:search', { query: this.searchQuery }).handle((result) => {
        this.tasks = result.tasks;
        this.loading = false;
      });
    } else {
      this.bus.send('tasks:list', { status: this.filter }).handle((result) => {
        this.tasks = result.tasks;
        this.stats = result.stats;
        this.loading = false;
      });
    }
  }

  private loadDetail(task: Task): void {
    this.loading = true;
    this.bus.send('tasks:get', { task_id: task.id, cocoonId: task.cocoonId }).handle((result) => {
      this.selectedTask = result.task;
      this.view = 'detail';
      this.loading = false;
    });
  }

  private handleStatusChange(task: Task, status: TaskStatus): void {
    this.bus.send('tasks:update', { task_id: task.id, cocoonId: task.cocoonId, status }).handle((result) => {
      this.tasks = this.tasks.map(t =>
        t.id === task.id && t.cocoonId === task.cocoonId ? result.task : t
      );
      if (this.selectedTask?.task.id === task.id) {
        this.selectedTask = { ...this.selectedTask, task: result.task };
      }
    });
  }

  private handleDelete(task: Task): void {
    if (!this.confirmingDelete) { this.confirmingDelete = true; return; }
    this.submitting = true;
    this.bus.send('tasks:delete', { task_id: task.id, cocoonId: task.cocoonId }).handle(() => {
      this.tasks = this.tasks.filter(t => !(t.id === task.id && t.cocoonId === task.cocoonId));
      this.view = 'list';
      this.submitting = false;
      this.confirmingDelete = false;
    });
  }

  private handleCreate(data: { title: string; description?: string; cocoonId: string }): void {
    this.submitting = true;
    this.bus.send('tasks:create', data).handle((result) => {
      this.tasks = [...this.tasks, result.task];
      this.view = 'list';
      this.submitting = false;
    });
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
        onBack: () => { this.view = 'list'; this.selectedTask = null; },
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
