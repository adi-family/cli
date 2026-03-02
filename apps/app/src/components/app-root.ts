import { LitElement, html, nothing } from 'lit';
import { customElement } from 'lit/decorators.js';
import { App } from '../app/app.ts';

const DEBUG_ROUTE = '/debug';

/** Longest-prefix route match; exact hit preferred, then path + '/'. */
function matchRoute<T extends { path: string }>(routes: T[], path: string): T | undefined {
  let best: T | undefined;
  for (const r of routes) {
    if (path !== r.path && !path.startsWith(r.path + '/')) continue;
    if (!best || r.path.length > best.path.length) best = r;
  }
  return best;
}

@customElement('app-root')
export class AppRoot extends LitElement {
  override createRenderRoot() { return this; }

  readonly #routeCache = new Map<string, HTMLElement>();

  override connectedCallback(): void {
    super.connectedCallback();
    if (App.instance) {
      this.#subscribe();
    } else {
      window.addEventListener('app-ready', () => this.#subscribe(), { once: true });
    }
  }

  #subscribe(): void {
    const bus = App.reqInstance.bus;

    bus.use({
      before: (event, payload, meta) =>
        console.debug(`%c[event:before] ${event}`, 'color: #7c9ef8; font-weight: bold', payload, meta),
      after: (event, payload, meta) =>
        console.debug(`%c[event:after]  ${event}`, 'color: #a78bfa; font-weight: bold', payload, meta),
      ignored: (event, payload, meta) =>
        console.debug(`%c[event:ignored] ${event}`, 'color: #f87171; font-weight: bold', payload, meta),
    });

    bus.on('route:register', () => this.requestUpdate(), 'app-root');
    bus.on('nav:add', () => this.requestUpdate(), 'app-root');
    bus.on('router:changed', () => this.requestUpdate(), 'app-root');

    this.requestUpdate();
  }

  #navigate(path: string): void {
    App.reqInstance.router.navigate(path);
  }

  #routeElement(route: { path: string; element: string }): HTMLElement {
    let el = this.#routeCache.get(route.path);
    if (!el) {
      el = document.createElement(route.element);
      this.#routeCache.set(route.path, el);
    }
    return el;
  }

  override render() {
    if (!App.instance?.router) return nothing;
    const { routes, navItems, currentPath } = App.instance.router;
    const isDebug = currentPath.startsWith(DEBUG_ROUTE);
    const activeRoute = isDebug ? null : matchRoute(routes, currentPath);

    return html`
      <div class="flex min-h-screen">
        ${navItems.length > 0 ? html`
          <nav class="w-48 shrink-0 border-r border-border bg-surface flex flex-col gap-1 p-3">
            ${navItems.map(item => html`
              <a
                href=${item.path}
                class=${[
                  'flex items-center gap-2 px-3 py-2 rounded-lg text-sm transition-colors no-underline',
                  currentPath.startsWith(item.path)
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
                .routes=${routes}
                .navItems=${navItems}
              ></app-debug-screen>`
            : activeRoute
                ? this.#routeElement(activeRoute)
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
