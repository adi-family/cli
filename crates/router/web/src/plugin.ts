import { LitElement, html, nothing } from 'lit';
import { customElement, state } from 'lit/decorators.js';
import { AdiPlugin } from '@adi-family/sdk-plugin';
import { CommandBusKey } from '@adi/command-palette-web-plugin/bus';
import { SlotsBusKey } from '@adi/slots-web-plugin/bus';
import { AdiRouterBusKey } from './bus';
import { PLUGIN_ID, PLUGIN_VERSION } from './config';

// ── Route matching ──────────────────────────────────────

interface Route {
  path: string;
  element: string;
}

function buildFullPath(pluginId: string, path: string): string {
  return path ? `/${pluginId}/${path}` : `/${pluginId}`;
}

function matchRoute(routes: Route[], path: string): Route | undefined {
  let best: Route | undefined;
  for (const r of routes) {
    if (path !== r.path && !path.startsWith(r.path + '/')) continue;
    if (!best || r.path.length > best.path.length) best = r;
  }
  return best;
}

// ── Shared state ────────────────────────────────────────

let sharedRoutes: Route[] = [];
let sharedPath = window.location.pathname;
const outlets = new Set<RouterOutlet>();

function notifyOutlets(): void {
  for (const o of outlets) o.sync();
}

// ── <router-outlet> ─────────────────────────────────────

@customElement('router-outlet')
export class RouterOutlet extends LitElement {
  @state() private routes: Route[] = sharedRoutes;
  @state() private currentPath = sharedPath;

  private readonly elementCache = new Map<string, HTMLElement>();

  override createRenderRoot() {
    return this;
  }

  override connectedCallback(): void {
    super.connectedCallback();
    outlets.add(this);
    this.sync();
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    outlets.delete(this);
  }

  sync(): void {
    this.routes = sharedRoutes;
    this.currentPath = sharedPath;
  }

  override render() {
    const active = matchRoute(this.routes, this.currentPath);
    if (!active) return nothing;

    let el = this.elementCache.get(active.path);
    if (!el) {
      el = document.createElement(active.element);
      this.elementCache.set(active.path, el);
    }

    return html`${el}`;
  }
}

// ── RouterPlugin ────────────────────────────────────────

const GOTO_PREFIX = 'goto:';

export class RouterPlugin extends AdiPlugin {
  readonly id = PLUGIN_ID;
  readonly version = PLUGIN_VERSION;

  private routes: Route[] = [];
  private routePaths = new Map<string, string>();
  private routerOutlet: RouterOutlet | null = null;

  get api() {
    return this;
  }

  private readonly onPopState = () => {
    sharedPath = window.location.pathname;
    notifyOutlets();
    this.bus.emit(
      AdiRouterBusKey.Changed,
      { path: sharedPath, params: {} },
      PLUGIN_ID,
    );
  };

  override onRegister(): void {
    this.routerOutlet = document.createElement('router-outlet') as RouterOutlet;
    this.bus.emit(
      SlotsBusKey.Place,
      { slot: 'maincontent', elementRef: this.routerOutlet, priority: 0, pluginId: PLUGIN_ID },
      PLUGIN_ID,
    );

    this.bus.on(
      AdiRouterBusKey.RegisterRoute,
      ({ pluginId, path, element, label }) => {
        const fullPath = buildFullPath(pluginId, path);
        if (this.routes.some((r) => r.path === fullPath)) return;

        this.routes = [...this.routes, { path: fullPath, element }];
        sharedRoutes = [...this.routes];
        notifyOutlets();

        const commandId = `${GOTO_PREFIX}${pluginId}`;
        this.routePaths.set(commandId, fullPath);
        this.bus.emit(
          CommandBusKey.Register,
          { id: commandId, label: `Go To ${label ?? pluginId}` },
          PLUGIN_ID,
        );
      },
      PLUGIN_ID,
    );

    this.bus.on(
      AdiRouterBusKey.Navigate,
      ({ path, replace }) => {
        if (replace) history.replaceState(null, '', path);
        else history.pushState(null, '', path);
        sharedPath = path;
        notifyOutlets();
        this.bus.emit(AdiRouterBusKey.Changed, { path, params: {} }, PLUGIN_ID);
      },
      PLUGIN_ID,
    );

    this.bus.on(
      CommandBusKey.Execute,
      ({ id }) => {
        const path = this.routePaths.get(id);
        if (path) this.navigate(path);
      },
      PLUGIN_ID,
    );

    window.addEventListener('popstate', this.onPopState);
  }

  navigate(path: string): void {
    history.pushState(null, '', path);
    sharedPath = path;
    notifyOutlets();
    this.bus.emit(AdiRouterBusKey.Changed, { path, params: {} }, PLUGIN_ID);
  }

  override onUnregister(): void {
    window.removeEventListener('popstate', this.onPopState);
    sharedRoutes = [];
    sharedPath = '/';
    this.routePaths.clear();
  }
}
