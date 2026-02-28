import { LitElement, html, nothing } from "lit";
import { customElement, state } from "lit/decorators.js";
import { getGlobal } from "../global.ts";

type Status =
  | "idle"
  | "waiting"
  | "detected"
  | "connecting"
  | "connected"
  | "error";
type Mode = "pick" | "local" | "remote";

const LOCAL_PORT = 14730;
const POLL_MS = 2000;

const LOCAL_PROXY_PORT = import.meta.env.VITE_LOCAL_PROXY_PORT as
  | string
  | undefined;
const resolveLocalUrl = (url: string): string =>
  LOCAL_PROXY_PORT
    ? url.replace("://adi.test/", `://127.0.0.1:${LOCAL_PROXY_PORT}/`)
    : url;

const getSignalingHub = () => getGlobal('signalingHub') ?? null;

const getSignalingUrl = (): string | null => {
  const hub = getSignalingHub();
  if (!hub) return null;
  const first = hub.managers.values().next();
  if (first.done) return null;
  return first.value.url;
};

const getFirstManager = () => {
  const hub = getSignalingHub();
  if (!hub) return null;
  const first = hub.managers.values().next();
  return first.done ? null : first.value;
};

// Inline SVG helpers
const svgCopy = html`<svg
  class="w-3.5 h-3.5"
  fill="none"
  stroke="currentColor"
  viewBox="0 0 24 24"
>
  <path
    stroke-linecap="round"
    stroke-linejoin="round"
    stroke-width="2"
    d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
  />
</svg>`;
const svgCheck = html`<svg
  class="w-3.5 h-3.5"
  fill="none"
  stroke="currentColor"
  viewBox="0 0 24 24"
>
  <path
    stroke-linecap="round"
    stroke-linejoin="round"
    stroke-width="2"
    d="M5 13l4 4L19 7"
  />
</svg>`;

@customElement("cocoon-manual-setup")
export class CocoonManualSetup extends LitElement {
  @state() private expanded = false;
  @state() private mode: Mode = "pick";
  @state() private status: Status = "idle";
  @state() private copied = false;
  @state() private errorMsg = "";
  @state() private machineName = "";
  @state() private remoteToken = "";

  private pollTimer: number | null = null;

  createRenderRoot() {
    return this;
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    this.stopPolling();
  }

  private toggle() {
    this.expanded = !this.expanded;
    if (!this.expanded) {
      this.stopPolling();
      if (this.status !== "connected") {
        this.status = "idle";
        this.mode = "pick";
      }
    }
  }

  private pickMode(mode: "local" | "remote") {
    this.mode = mode;
    if (mode === "local") {
      this.status = "waiting";
      this.startPolling();
    } else if (mode === "remote") {
      this.fetchRemoteToken();
    }
  }

  private async fetchRemoteToken() {
    const manager = getFirstManager();
    if (!manager) return;
    try {
      this.remoteToken = await manager.requestSetupToken();
    } catch (err) {
      console.debug("[cocoon-setup] failed to get remote setup token:", err);
    }
  }

  private backToPick() {
    this.stopPolling();
    this.status = "idle";
    this.mode = "pick";
  }

  // --- Polling ---

  private startPolling() {
    this.stopPolling();
    this.poll();
    this.pollTimer = window.setInterval(() => this.poll(), POLL_MS);
  }

  private stopPolling() {
    if (this.pollTimer !== null) {
      clearInterval(this.pollTimer);
      this.pollTimer = null;
    }
  }

  private async poll() {
    try {
      const res = await fetch(`http://localhost:${LOCAL_PORT}/health`, {
        method: "GET",
        mode: "cors",
      });
      if (!res.ok) return;

      const data = await res.json().catch(() => ({}));
      const name = data.name || "Local Machine";

      if (data.connected) {
        this.handleConnected(name);
        return;
      }

      this.status = "detected";
      this.machineName = name;
      await this.sendConnect(name);
    } catch {
      // Server not up yet
    }
  }

  private async sendConnect(name: string) {
    this.status = "connecting";
    const signalingUrl = getSignalingUrl();

    if (!signalingUrl) {
      this.status = "error";
      this.errorMsg = "No signaling server configured";
      return;
    }

    // Request a setup token from signaling server so the cocoon auto-claims ownership
    let setupToken = "";
    const manager = getFirstManager();
    if (manager) {
      try {
        setupToken = await manager.requestSetupToken();
      } catch (err) {
        console.debug("[cocoon-setup] failed to get setup token:", err);
      }
    }

    try {
      const res = await fetch(`http://localhost:${LOCAL_PORT}/connect`, {
        method: "POST",
        mode: "cors",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          token: setupToken,
          signaling_url: resolveLocalUrl(signalingUrl),
        }),
      });

      if (!res.ok) {
        this.status = "error";
        this.errorMsg = `Server responded ${res.status}`;
        return;
      }

      const result = await res.json();
      if (
        result.status === "connecting" ||
        result.status === "connected" ||
        result.status === "already_connected"
      ) {
        this.handleConnected(result.name || name);
      } else {
        this.status = "error";
        this.errorMsg = result.error || "Unexpected response";
      }
    } catch (err) {
      this.status = "error";
      this.errorMsg = err instanceof Error ? err.message : "Connection failed";
    }
  }

  private handleConnected(name: string) {
    this.stopPolling();
    this.machineName = name;
    this.status = "connected";
    this.dispatchEvent(
      new CustomEvent("cocoon-connected", {
        bubbles: true,
        composed: true,
        detail: { name },
      }),
    );
  }

  // --- Copy ---

  private async copy(text: string) {
    await navigator.clipboard.writeText(text);
    this.copied = true;
    setTimeout(() => {
      this.copied = false;
    }, 2000);
  }

  // --- Render ---

  render() {
    return html`
      <div class="border border-border rounded overflow-hidden">
        <button
          type="button"
          class="w-full flex items-center justify-between px-4 py-2.5 text-xs font-medium text-text hover:bg-surface/50 transition-colors"
          @click=${() => this.toggle()}
        >
          <span class="flex items-center gap-2">
            <svg
              class="w-4 h-4 text-accent"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
              />
            </svg>
            Connect Machine
          </span>
          <span class="flex items-center gap-2">
            ${this.status === "connected"
              ? html`
                  <span class="text-green-400 text-[10px] font-medium"
                    >Connected</span
                  >
                `
              : nothing}
            <svg
              class="w-3.5 h-3.5 text-text-muted transition-transform ${this
                .expanded
                ? "rotate-180"
                : ""}"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M19 9l-7 7-7-7"
              />
            </svg>
          </span>
        </button>

        ${this.expanded
          ? html`
              <div class="px-4 pb-4 pt-1 border-t border-border space-y-3">
                ${this.status === "connected"
                  ? this.renderConnected()
                  : this.mode === "pick"
                    ? this.renderPick()
                    : this.mode === "local"
                      ? this.renderLocal()
                      : this.renderRemote()}
              </div>
            `
          : nothing}
      </div>
    `;
  }

  private renderPick() {
    return html`
      <div class="grid grid-cols-2 gap-2">
        <button
          type="button"
          class="flex flex-col items-center gap-1.5 px-3 py-3 rounded border border-border hover:border-accent/50 hover:bg-accent/5 transition-colors"
          @click=${() => this.pickMode("local")}
        >
          <svg
            class="w-5 h-5 text-text-muted"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
            />
          </svg>
          <span class="text-xs font-medium text-text">This Machine</span>
          <span class="text-[10px] text-text-muted"
            >Auto-detected via localhost</span
          >
        </button>
        <button
          type="button"
          class="flex flex-col items-center gap-1.5 px-3 py-3 rounded border border-border hover:border-accent/50 hover:bg-accent/5 transition-colors"
          @click=${() => this.pickMode("remote")}
        >
          <svg
            class="w-5 h-5 text-text-muted"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M5 12h14M12 5l7 7-7 7"
            />
          </svg>
          <span class="text-xs font-medium text-text">Remote Machine</span>
          <span class="text-[10px] text-text-muted"
            >SSH into server and run command</span
          >
        </button>
      </div>
    `;
  }

  private renderLocal() {
    return html`
      ${this.renderBackButton()}
      <p class="text-xs text-text-muted">Run this command in your terminal:</p>
      ${this.renderCommandBox("adi cocoon setup")} ${this.renderStatus()}
    `;
  }

  private renderRemote() {
    const signalingUrl = getSignalingUrl() || "<signaling-url>";
    const tokenPart = this.remoteToken ? ` --token ${this.remoteToken}` : "";
    const cmd = `adi cocoon setup --url ${signalingUrl}${tokenPart}`;
    return html`
      ${this.renderBackButton()}
      <p class="text-xs text-text-muted">SSH into your server and run:</p>
      ${this.renderCommandBox(cmd)}
      <p class="text-[10px] text-text-muted leading-relaxed">
        The cocoon will connect to the signaling server and appear in the list
        below.
      </p>
    `;
  }

  private renderBackButton() {
    return html`
      <button
        type="button"
        class="flex items-center gap-1 text-[10px] text-text-muted hover:text-text transition-colors -mt-0.5"
        @click=${() => this.backToPick()}
      >
        <svg
          class="w-3 h-3"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M15 19l-7-7 7-7"
          />
        </svg>
        Back
      </button>
    `;
  }

  private renderCommandBox(cmd: string) {
    return html`
      <div
        class="flex items-center gap-2 px-3 py-2 bg-surface border border-border rounded font-mono text-xs"
      >
        <span class="text-text-muted select-none">$</span>
        <code class="flex-1 text-text overflow-x-auto whitespace-nowrap"
          >${cmd}</code
        >
        <button
          type="button"
          class="flex items-center justify-center w-6 h-6 rounded transition-colors shrink-0 ${this
            .copied
            ? "text-green-400"
            : "text-text-muted hover:text-text"}"
          @click=${() => this.copy(cmd)}
          title="Copy command"
        >
          ${this.copied ? svgCheck : svgCopy}
        </button>
      </div>
    `;
  }

  private renderStatus() {
    switch (this.status) {
      case "waiting":
        return html`
          <div class="flex items-center gap-2 text-xs text-text-muted">
            <span
              class="w-3 h-3 border-2 border-current/30 border-t-current rounded-full animate-spin"
            ></span>
            Waiting for local server on port ${LOCAL_PORT}...
          </div>
        `;
      case "detected":
        return html`
          <div class="flex items-center gap-2 text-xs text-accent">
            ${svgCheck} Server
            detected${this.machineName
              ? html` — <span class="font-mono">${this.machineName}</span>`
              : nothing}
          </div>
        `;
      case "connecting":
        return html`
          <div class="flex items-center gap-2 text-xs text-accent">
            <span
              class="w-3 h-3 border-2 border-current/30 border-t-current rounded-full animate-spin"
            ></span>
            Connecting...
          </div>
        `;
      case "error":
        return html`
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2 text-xs text-red-400">
              <svg
                class="w-3.5 h-3.5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              ${this.errorMsg}
            </div>
            <button
              type="button"
              class="text-[10px] px-2 py-1 rounded border border-border text-text-muted hover:text-text transition-colors"
              @click=${() => {
                this.status = "waiting";
                this.startPolling();
              }}
            >
              Retry
            </button>
          </div>
        `;
      default:
        return nothing;
    }
  }

  private renderConnected() {
    return html`
      <div class="flex items-center gap-2 text-xs text-green-400">
        <svg
          class="w-4 h-4"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
        <span class="font-medium">${this.machineName || "Local Machine"}</span>
        connected
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "cocoon-manual-setup": CocoonManualSetup;
  }
}
