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

  private unsubs: Array<() => void> = [];

  override createRenderRoot() { return this; }

  override connectedCallback(): void {
    super.connectedCallback();
    this.unsubs.push(
      this.bus.on('tasks:list-changed', ({ tasks, stats }) => {
        this.tasks = tasks;
        this.stats = stats;
        this.loading = false;
      }, 'tasks-ui'),
      this.bus.on('tasks:search-changed', ({ tasks }) => {
        this.tasks = tasks;
        this.loading = false;
      }, 'tasks-ui'),
      this.bus.on('tasks:detail-changed', ({ task }) => {
        this.selectedTask = task;
        this.loading = false;
      }, 'tasks-ui'),
      this.bus.on('tasks:task-mutated', () => {
        this.submitting = false;
        this.loadData();
      }, 'tasks-ui'),
      this.bus.on('tasks:task-deleted', ({ task_id, cocoonId }) => {
        this.tasks = this.tasks.filter(t => !(t.id === task_id && t.cocoonId === cocoonId));
        this.view = 'list';
        this.selectedTask = null;
        this.confirmingDelete = false;
        this.submitting = false;
      }, 'tasks-ui'),
      this.bus.on('tasks:stats-changed', ({ stats }) => {
        this.stats = stats;
      }, 'tasks-ui'),
    );
    this.loadData();
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.unsubs.forEach(fn => fn());
    this.unsubs = [];
  }

  private get bus() { return window.sdk.bus; }

  private loadData(): void {
    this.loading = true;
    this.error = null;
    if (this.searchQuery.trim()) {
      this.stats = null;
      this.bus.emit('tasks:search', { query: this.searchQuery }, 'tasks-ui');
    } else {
      this.bus.emit('tasks:list', { status: this.filter }, 'tasks-ui');
    }
  }

  private loadDetail(task: Task): void {
    this.loading = true;
    this.view = 'detail';
    this.bus.emit('tasks:get', { task_id: task.id, cocoonId: task.cocoonId }, 'tasks-ui');
  }

  private handleStatusChange(task: Task, status: TaskStatus): void {
    this.bus.emit('tasks:update', { task_id: task.id, cocoonId: task.cocoonId, status }, 'tasks-ui');
  }

  private handleDelete(task: Task): void {
    if (!this.confirmingDelete) { this.confirmingDelete = true; return; }
    this.submitting = true;
    this.bus.emit('tasks:delete', { task_id: task.id, cocoonId: task.cocoonId }, 'tasks-ui');
  }

  private handleCreate(data: { title: string; description?: string; cocoonId: string }): void {
    this.submitting = true;
    this.bus.emit('tasks:create', data, 'tasks-ui');
    this.view = 'list';
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
