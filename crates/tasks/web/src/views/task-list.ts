import { html, nothing, type TemplateResult } from "lit";
import type { Task, TaskStatus, TasksStats } from "../types";

const STATUS_LABELS: Record<TaskStatus, string> = {
  todo: "Todo",
  in_progress: "In Progress",
  done: "Done",
  blocked: "Blocked",
  cancelled: "Cancelled",
};

const STATUS_COLORS: Record<TaskStatus, string> = {
  todo: "bg-gray-500/20 text-gray-300",
  in_progress: "bg-blue-500/20 text-blue-300",
  done: "bg-green-500/20 text-green-300",
  blocked: "bg-red-500/20 text-red-300",
  cancelled: "bg-gray-600/20 text-gray-400",
};

const timeAgo = (ts: number): string => {
  const seconds = Math.floor(Date.now() / 1000 - ts);
  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
};

interface TaskListCallbacks {
  onSelectTask: (id: number) => void;
  onCreateTask: () => void;
  onFilterChange: (status: TaskStatus | null) => void;
  onSearch: (query: string) => void;
}

const statBadge = (label: string, count: number, color: string) => html`
  <div class="flex flex-col items-center px-3 py-1.5 rounded-lg ${color}">
    <span class="text-lg font-semibold">${count}</span>
    <span class="text-xs opacity-70">${label}</span>
  </div>
`;

const statsBar = (stats: TasksStats | null) => {
  if (!stats) return nothing;
  return html`
    <div class="flex gap-2 flex-wrap mb-4">
      ${statBadge("Total", stats.total_tasks, "bg-white/5 text-gray-300")}
      ${statBadge("Todo", stats.todo_count, "bg-gray-500/10 text-gray-300")}
      ${statBadge("In Progress", stats.in_progress_count, "bg-blue-500/10 text-blue-300")}
      ${statBadge("Done", stats.done_count, "bg-green-500/10 text-green-300")}
      ${statBadge("Blocked", stats.blocked_count, "bg-red-500/10 text-red-300")}
      ${statBadge("Cancelled", stats.cancelled_count, "bg-gray-600/10 text-gray-400")}
    </div>
  `;
};

const filterTabs = (current: TaskStatus | null, onChange: (s: TaskStatus | null) => void) => {
  const tabs: Array<{ label: string; value: TaskStatus | null }> = [
    { label: "All", value: null },
    { label: "Todo", value: "todo" },
    { label: "In Progress", value: "in_progress" },
    { label: "Done", value: "done" },
    { label: "Blocked", value: "blocked" },
    { label: "Cancelled", value: "cancelled" },
  ];

  return html`
    <div class="flex gap-1 mb-4 flex-wrap">
      ${tabs.map(
        (t) => html`
          <button
            class="px-3 py-1 rounded-full text-sm transition-colors ${
              current === t.value
                ? "bg-purple-500/30 text-purple-200 font-medium"
                : "bg-white/5 text-gray-400 hover:bg-white/10 hover:text-gray-200"
            }"
            @click=${() => onChange(t.value)}
          >
            ${t.label}
          </button>
        `
      )}
    </div>
  `;
};

const taskRow = (task: Task, onSelect: (id: number) => void) => html`
  <button
    class="w-full text-left p-3 rounded-lg bg-white/5 hover:bg-white/10 transition-colors flex items-start gap-3 group"
    @click=${() => onSelect(task.id)}
  >
    <span class="inline-flex px-2 py-0.5 rounded text-xs font-medium shrink-0 mt-0.5 ${STATUS_COLORS[task.status]}">
      ${STATUS_LABELS[task.status]}
    </span>
    <div class="flex-1 min-w-0">
      <div class="text-sm text-gray-200 group-hover:text-white truncate">${task.title}</div>
      ${task.description
        ? html`<div class="text-xs text-gray-500 mt-0.5 truncate">${task.description}</div>`
        : nothing}
    </div>
    <span class="text-xs text-gray-600 shrink-0 mt-0.5">${timeAgo(task.updated_at)}</span>
  </button>
`;

export const renderTaskList = (
  tasks: Task[],
  stats: TasksStats | null,
  filter: TaskStatus | null,
  searchQuery: string,
  loading: boolean,
  cb: TaskListCallbacks
): TemplateResult => html`
  <div class="space-y-3">
    <div class="flex items-center justify-between mb-2">
      <h2 class="text-lg font-semibold text-gray-200">Tasks</h2>
      <button
        class="px-3 py-1.5 rounded-lg bg-purple-500/20 text-purple-200 hover:bg-purple-500/30 transition-colors text-sm font-medium"
        @click=${cb.onCreateTask}
      >
        + New Task
      </button>
    </div>

    ${statsBar(stats)}
    ${filterTabs(filter, cb.onFilterChange)}

    <div class="relative mb-3">
      <input
        type="text"
        placeholder="Search tasks..."
        .value=${searchQuery}
        @input=${(e: InputEvent) => cb.onSearch((e.target as HTMLInputElement).value)}
        class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50"
      />
    </div>

    ${loading
      ? html`<div class="text-center py-8 text-gray-500 text-sm">Loading...</div>`
      : tasks.length === 0
        ? html`<div class="text-center py-8 text-gray-500 text-sm">No tasks found</div>`
        : html`
            <div class="space-y-1.5">
              ${tasks.map((t) => taskRow(t, cb.onSelectTask))}
            </div>
          `}
  </div>
`;
