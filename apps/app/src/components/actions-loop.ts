import { LitElement, html } from "lit";
import { customElement, state } from "lit/decorators.js";
import { unsafeHTML } from "lit/directives/unsafe-html.js";
import { AdiPlugin } from "@adi-family/sdk-plugin";

interface ActionCard {
  id: string;
  plugin: string;
  kind: string;
  data: Record<string, unknown>;
  priority: "low" | "normal" | "urgent";
}

type RenderFn = (data: Record<string, unknown>, actionId: string) => string;

const rendererKey = (plugin: string, kind: string) => `${plugin}::${kind}`;

const store = {
  actions: [] as ActionCard[],
  renderers: new Map<string, RenderFn>(),
  listeners: new Set<() => void>(),
  notify() {
    for (const fn of this.listeners) fn();
  },
};

@customElement("app-actions-loop")
export class AppActionsLoop extends LitElement {
  @state() private actions: ActionCard[] = [];

  private unsub: (() => void) | null = null;

  override createRenderRoot() {
    return this;
  }

  override connectedCallback(): void {
    super.connectedCallback();
    this.actions = store.actions;
    const listener = () => {
      this.actions = store.actions;
    };
    store.listeners.add(listener);
    this.unsub = () => store.listeners.delete(listener);
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.unsub?.();
    this.unsub = null;
  }

  #dismiss(id: string): void {
    if ((window as { sdk?: unknown }).sdk) {
      window.sdk.bus.emit("actions:dismiss", { id }, "actions-loop");
    }
  }

  #renderCard(card: ActionCard) {
    const renderer = store.renderers.get(rendererKey(card.plugin, card.kind));
    const borderColor =
      card.priority === "urgent" ? "border-red-500/60" : "border-border";

    const body = renderer
      ? unsafeHTML(renderer(card.data, card.id))
      : html`
          <div class="text-xs text-text-muted">
            <span class="font-medium text-text">${card.kind}</span>
            <span class="ml-1 opacity-60">(${card.plugin})</span>
          </div>
        `;

    return html`
      <div class="relative bg-surface border ${borderColor} rounded-lg p-3">
        <button
          type="button"
          class="absolute top-1.5 right-1.5 w-5 h-5 flex items-center justify-center rounded text-text-muted hover:text-text hover:bg-surface-alt transition-colors text-xs"
          @click=${() => this.#dismiss(card.id)}
          aria-label="Dismiss"
        >
          &times;
        </button>
        ${body}
      </div>
    `;
  }

  override render() {
    const sorted = [...this.actions].sort((a, b) => {
      const order = { urgent: 0, normal: 1, low: 2 };
      return order[a.priority] - order[b.priority];
    });

    return html`
      <div class="min-h-screen bg-bg p-6 space-y-1">
        <div class="mb-2">
          <h1 class="text-xl font-semibold text-text">Actions</h1>
          <p class="text-sm text-text-muted">
            ${this.actions.length} pending
            action${this.actions.length !== 1 ? "s" : ""}
          </p>
        </div>

        ${sorted.length > 0
          ? html`<div class="flex flex-col gap-2">
              ${sorted.map((card) => this.#renderCard(card))}
            </div>`
          : html`
              <div
                class="flex items-center justify-center py-24 text-text-muted text-sm"
              >
                No pending actions.
              </div>
            `}
      </div>
    `;
  }
}

export class ActionsPlugin extends AdiPlugin {
  readonly id = "app.actions";
  readonly version = "1.0.0";

  override onRegister(): void {
    this.bus.emit(
      "route:register",
      { path: "/actions", element: "app-actions-loop", label: "Actions" },
      "actions-loop",
    );
    this.bus
      .send(
        "nav:add",
        { id: "app.actions", label: "Actions", path: "/actions" },
        "actions-loop",
      )
      .handle(() => {});

    this.bus.emit(
      "command:register",
      { id: "app:actions", label: "Open Actions", shortcut: "⌘⇧A" },
      "actions-loop",
    );
    this.bus.on(
      "command:execute",
      ({ id }) => {
        if (id === "app:actions")
          this.bus.emit(
            "router:navigate",
            { path: "/actions" },
            "actions-loop",
          );
      },
      "actions-loop",
    );

    this.bus.on(
      "actions:register-renderer",
      ({ plugin, kind, render }) => {
        store.renderers.set(rendererKey(plugin, kind), render);
        store.notify();
      },
      "actions-loop",
    );

    this.bus.on(
      "actions:push",
      ({ id, plugin, kind, data, priority }) => {
        const card: ActionCard = {
          id,
          plugin,
          kind,
          data,
          priority: priority ?? "normal",
        };
        const idx = store.actions.findIndex((a) => a.id === id);
        store.actions =
          idx >= 0
            ? store.actions.map((a, i) => (i === idx ? card : a))
            : [...store.actions, card];
        store.notify();
      },
      "actions-loop",
    );

    this.bus.on(
      "actions:dismiss",
      ({ id }) => {
        const card = store.actions.find((a) => a.id === id);
        if (!card) return;
        store.actions = store.actions.filter((a) => a.id !== id);
        this.bus.emit(
          "actions:dismissed",
          { id: card.id, plugin: card.plugin, kind: card.kind },
          "actions-loop",
        );
        store.notify();
      },
      "actions-loop",
    );
  }
}
