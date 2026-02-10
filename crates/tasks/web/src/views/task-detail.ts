import { html, nothing, type TemplateResult } from "lit";
import type { Task, TaskStatus, TaskWithDependencies } from "../types";

const STATUS_OPTIONS: TaskStatus[] = ["todo", "in_progress", "done", "blocked", "cancelled"];

const STATUS_COLORS: Record<TaskStatus, string> = {
  todo: "bg-gray-500/20 text-gray-300",
  in_progress: "bg-blue-500/20 text-blue-300",
  done: "bg-green-500/20 text-green-300",
  blocked: "bg-red-500/20 text-red-300",
  cancelled: "bg-gray-600/20 text-gray-400",
};

const STATUS_LABELS: Record<TaskStatus, string> = {
  todo: "Todo",
  in_progress: "In Progress",
  done: "Done",
  blocked: "Blocked",
  cancelled: "Cancelled",
};

const formatTime = (ts: number): string =>
  new Date(ts * 1000).toLocaleString();

interface TaskDetailCallbacks {
  onBack: () => void;
  onStatusChange: (taskId: number, status: TaskStatus) => void;
  onDelete: (taskId: number) => void;
  onSelectTask: (taskId: number) => void;
}

const depList = (label: string, tasks: Task[], onSelect: (id: number) => void) => {
  if (tasks.length === 0) return nothing;
  return html`
    <div class="mt-4">
      <h4 class="text-xs font-medium text-gray-400 uppercase tracking-wider mb-2">${label}</h4>
      <div class="space-y-1">
        ${tasks.map(
          (t) => html`
            <button
              class="w-full text-left px-3 py-2 rounded-lg bg-white/5 hover:bg-white/10 transition-colors flex items-center gap-2 text-sm"
              @click=${() => onSelect(t.id)}
            >
              <span class="inline-flex px-1.5 py-0.5 rounded text-xs ${STATUS_COLORS[t.status]}">
                ${STATUS_LABELS[t.status]}
              </span>
              <span class="text-gray-300 truncate">${t.title}</span>
            </button>
          `
        )}
      </div>
    </div>
  `;
};

export const renderTaskDetail = (
  data: TaskWithDependencies | null,
  loading: boolean,
  confirmingDelete: boolean,
  cb: TaskDetailCallbacks
): TemplateResult => {
  if (loading || !data) {
    return html`
      <div class="space-y-3">
        <button class="text-sm text-gray-400 hover:text-gray-200 transition-colors" @click=${cb.onBack}>
          &larr; Back to list
        </button>
        <div class="text-center py-8 text-gray-500 text-sm">${loading ? "Loading..." : "Task not found"}</div>
      </div>
    `;
  }

  const { task, depends_on, dependents } = data;

  return html`
    <div class="space-y-4">
      <button class="text-sm text-gray-400 hover:text-gray-200 transition-colors" @click=${cb.onBack}>
        &larr; Back to list
      </button>

      <div class="bg-white/5 rounded-xl p-4 space-y-4">
        <h2 class="text-lg font-semibold text-gray-100">${task.title}</h2>

        ${task.description
          ? html`<p class="text-sm text-gray-400 whitespace-pre-wrap">${task.description}</p>`
          : nothing}

        <div class="flex items-center gap-3 flex-wrap">
          <label class="text-xs text-gray-500 uppercase tracking-wider">Status</label>
          <div class="flex gap-1 flex-wrap">
            ${STATUS_OPTIONS.map(
              (s) => html`
                <button
                  class="px-2.5 py-1 rounded text-xs transition-colors ${
                    task.status === s
                      ? STATUS_COLORS[s] + " font-medium ring-1 ring-white/20"
                      : "bg-white/5 text-gray-500 hover:bg-white/10 hover:text-gray-300"
                  }"
                  @click=${() => { if (task.status !== s) cb.onStatusChange(task.id, s); }}
                >
                  ${STATUS_LABELS[s]}
                </button>
              `
            )}
          </div>
        </div>

        <div class="flex gap-4 text-xs text-gray-500">
          <span>Created: ${formatTime(task.created_at)}</span>
          <span>Updated: ${formatTime(task.updated_at)}</span>
        </div>

        ${depList("Depends on", depends_on, cb.onSelectTask)}
        ${depList("Blocked by this", dependents, cb.onSelectTask)}

        <div class="pt-3 border-t border-white/10">
          ${confirmingDelete
            ? html`
                <div class="flex items-center gap-2">
                  <span class="text-sm text-red-400">Delete this task?</span>
                  <button
                    class="px-3 py-1 rounded text-sm bg-red-500/20 text-red-300 hover:bg-red-500/30 transition-colors"
                    @click=${() => cb.onDelete(task.id)}
                  >
                    Confirm
                  </button>
                  <button
                    class="px-3 py-1 rounded text-sm bg-white/5 text-gray-400 hover:bg-white/10 transition-colors"
                    @click=${cb.onBack}
                  >
                    Cancel
                  </button>
                </div>
              `
            : html`
                <button
                  class="px-3 py-1 rounded text-sm bg-red-500/10 text-red-400 hover:bg-red-500/20 transition-colors"
                  @click=${() => cb.onDelete(-1)}
                >
                  Delete Task
                </button>
              `}
        </div>
      </div>
    </div>
  `;
};
