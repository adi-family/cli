import { LitElement, html, nothing } from 'lit';
import { customElement, state } from 'lit/decorators.js';
import { unsafeHTML } from 'lit/directives/unsafe-html.js';
import type { EventBus } from '@adi-family/sdk-plugin';

interface NavItem { id: string; label: string; path: string; icon?: string }
interface RouteEntry { path: string; element: string }

@customElement('app-root')
export class AppRoot extends LitElement {
  @state() private navItems: NavItem[] = [];
  @state() private routes: RouteEntry[] = [];
  @state() private currentPath = window.location.pathname;

  override createRenderRoot() { return this; }

  override connectedCallback(): void {
    super.connectedCallback();
    window.addEventListener('popstate', this.#onPopState);
    // window.sdk is set asynchronously in main.ts — wait for sdk-ready signal
    if ((window as { sdk?: unknown }).sdk) {
      this.#subscribe();
    } else {
      window.addEventListener('sdk-ready', () => this.#subscribe(), { once: true });
    }
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    window.removeEventListener('popstate', this.#onPopState);
  }

  readonly #onPopState = () => { this.currentPath = window.location.pathname; };

  #subscribe(): void {
    const bus = window.sdk.bus as EventBus;

    bus.on('nav:add', ({ id, label, path, icon }) => {
      if (!this.navItems.find(n => n.id === id)) {
        this.navItems = [...this.navItems, { id, label, path, icon }];
      }
    });

    bus.on('route:register', ({ path, element }) => {
      if (!this.routes.find(r => r.path === path)) {
        this.routes = [...this.routes, { path, element }];
        this.requestUpdate();
      }
    });

    bus.on('router:navigate', ({ path, replace }) => {
      if (replace) history.replaceState(null, '', path);
      else history.pushState(null, '', path);
      this.currentPath = path;
      bus.emit('router:changed', { path, params: {} });
    });
  }

  #navigate(path: string): void {
    history.pushState(null, '', path);
    this.currentPath = path;
    if ((window as { sdk?: unknown }).sdk) {
      window.sdk.bus.emit('router:changed', { path, params: {} });
    }
  }

  override render() {
    const activeRoute = this.routes.find(r => this.currentPath.startsWith(r.path));

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
                ${item.icon ? html`<span class="text-base leading-none">${item.icon}</span>` : nothing}
                <span>${item.label}</span>
              </a>
            `)}
          </nav>
        ` : nothing}
        <main class="flex-1 min-w-0">
          ${activeRoute
            ? unsafeHTML(`<${activeRoute.element}></${activeRoute.element}>`)
            : html`
                <div class="flex items-center justify-center min-h-screen">
                  <h1 class="text-4xl font-bold text-text">App</h1>
                </div>
              `
          }
        </main>
      </div>
    `;
  }
}
