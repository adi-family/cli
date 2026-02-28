import { LitElement, html, nothing } from 'lit';
import { customElement, state } from 'lit/decorators.js';
import { unsafeHTML } from 'lit/directives/unsafe-html.js';
import type { EventBus } from '@adi-family/sdk-plugin';

interface NavItem { id: string; label: string; path: string; icon?: string }
interface RouteEntry { path: string; element: string; label?: string }
interface Command { id: string; label: string; shortcut?: string }

const DEBUG_ROUTE = '/debug';

@customElement('app-root')
export class AppRoot extends LitElement {
  @state() private navItems: NavItem[] = [];
  @state() private routes: RouteEntry[] = [];
  @state() private commands: Command[] = [];
  @state() private currentPath = window.location.pathname;

  override createRenderRoot() { return this; }

  override connectedCallback(): void {
    super.connectedCallback();
    window.addEventListener('popstate', this.#onPopState);
    if ((window as { sdk?: unknown }).sdk) {
      this.#subscribe();
    } else {
      window.addEventListener('app-ready', () => this.#subscribe(), { once: true });
    }
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    window.removeEventListener('popstate', this.#onPopState);
  }

  readonly #onPopState = () => { this.currentPath = window.location.pathname; };

  #subscribe(): void {
    const bus = window.sdk.bus as EventBus;

    bus.use({
      before: (event, payload, meta) =>
        console.debug(`%c[event:before] ${event}`, 'color: #7c9ef8; font-weight: bold', payload, meta),
      after: (event, payload, meta) =>
        console.debug(`%c[event:after]  ${event}`, 'color: #a78bfa; font-weight: bold', payload, meta),
      ignored: (event, payload, meta) =>
        console.debug(`%c[event:ignored] ${event}`, 'color: #f87171; font-weight: bold', payload, meta),
    });

    bus.on('nav:add', ({ id, label, path, icon }) => {
      if (!this.navItems.find(n => n.id === id)) {
        this.navItems = [...this.navItems, { id, label, path, icon }];
      }
    }, 'app-root');

    bus.on('route:register', ({ path, element, label }) => {
      if (!this.routes.find(r => r.path === path)) {
        this.routes = [...this.routes, { path, element, label }];
        this.requestUpdate();
      }
    }, 'app-root');

    bus.on('router:navigate', ({ path, replace }) => {
      if (replace) history.replaceState(null, '', path);
      else history.pushState(null, '', path);
      this.currentPath = path;
      bus.emit('router:changed', { path, params: {} }, 'app-root');
    }, 'app-root');

    bus.on('command:register', ({ id, label, shortcut }) => {
      if (!this.commands.find(c => c.id === id)) {
        this.commands = [...this.commands, { id, label, shortcut }];
      }
    }, 'app-root');

    // Defer built-in command registration by one microtask so the command-palette
    // (rendered in app-root's first render, which is also a microtask) has time
    // to connect and subscribe to command:register before we emit.
    queueMicrotask(() => {
      bus.emit('command:register', { id: 'app:debug', label: 'Open Debug Screen', shortcut: '⌘⇧D' }, 'app-root');
      bus.emit('command:register', { id: 'app:ops-log', label: 'Toggle Operations Log', shortcut: '⌘⇧O' }, 'app-root');
    });

    bus.on('command:execute', ({ id }) => {
      if (id === 'app:debug') this.#navigate(DEBUG_ROUTE);
    }, 'app-root');
  }

  #navigate(path: string): void {
    history.pushState(null, '', path);
    this.currentPath = path;
    if ((window as { sdk?: unknown }).sdk) {
      window.sdk.bus.emit('router:changed', { path, params: {} }, 'app-root');
    }
  }

  override render() {
    const isDebug = this.currentPath.startsWith(DEBUG_ROUTE);
    const activeRoute = isDebug ? null : this.routes.find(r => this.currentPath.startsWith(r.path));

    return html`
      <div class="flex min-h-screen">
        ${this.navItems.length > 0 ? html`
          <nav class="w-48 shrink-0 border-r border-border bg-surface flex flex-col gap-1 p-3">
            ${this.navItems.map(item => html`
              <a
                href=${item.path}
                class=${[
                  'flex items-center gap-2 px-3 py-2 rounded-lg text-sm transition-colors no-underline',
                  this.currentPath.startsWith(item.path)
                    ? 'bg-accent/20 text-accent font-medium'
                    : 'text-text-muted hover:text-text hover:bg-surface-alt',
                ].join(' ')}
                @click=${(e: Event) => { e.preventDefault(); this.#navigate(item.path); }}
              >
                <span>${item.label}</span>
              </a>
            `)}
          </nav>
        ` : nothing}

        <main class="flex-1 min-w-0">
          ${isDebug
            ? html`<app-debug-screen
                .routes=${this.routes}
                .navItems=${this.navItems}
                .commands=${this.commands}
              ></app-debug-screen>`
            : activeRoute
                ? unsafeHTML(`<${activeRoute.element}></${activeRoute.element}>`)
                : html`
                    <div class="flex items-center justify-center min-h-screen">
                      <h1 class="text-4xl font-bold text-text">App</h1>
                    </div>
                  `
          }
        </main>
      </div>

      <!-- Command palette always mounted, hidden until triggered -->
      <app-command-palette></app-command-palette>
      <app-ops-log></app-ops-log>
    `;
  }
}
