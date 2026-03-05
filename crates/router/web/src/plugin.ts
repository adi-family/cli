import { AdiPlugin } from '@adi-family/sdk-plugin';
import { CommandBusKey } from '@adi/command-palette-web-plugin/bus';
import { AdiRouterBusKey, AdiRouterRegisterRouteEvent } from './bus';
import { PLUGIN_ID, PLUGIN_VERSION } from './config';

const GOTO_PREFIX = 'router:goto:';

export class RouterPlugin extends AdiPlugin {
  readonly id = PLUGIN_ID;
  readonly version = PLUGIN_VERSION;

  routes: AdiRouterRegisterRouteEvent[] = [];
  currentPath = window.location.pathname;

  get api() {
    return this;
  }

  private readonly onPopState = () => {
    this.currentPath = window.location.pathname;
    this.bus.emit(
      AdiRouterBusKey.Changed,
      { path: this.currentPath, params: {} },
      PLUGIN_ID,
    );
  };

  private registerGoToCommand(route: AdiRouterRegisterRouteEvent): void {
    const label = route.label ?? route.path;
    this.bus.emit(
      CommandBusKey.Register,
      { id: `${GOTO_PREFIX}${route.path}`, label: `Go To ${label}` },
      PLUGIN_ID,
    );
  }

  override onRegister(): void {
    this.bus.on(
      AdiRouterBusKey.RegisterRoute,
      ({ path, element, label }) => {
        if (this.routes.some((r) => r.path === path)) return;

        const route = { path, element, label };
        this.routes = [...this.routes, route];
        this.registerGoToCommand(route);
      },
      PLUGIN_ID,
    );

    this.bus.on(
      AdiRouterBusKey.Navigate,
      ({ path, replace }) => {
        if (replace) history.replaceState(null, '', path);
        else history.pushState(null, '', path);
        this.currentPath = path;
        this.bus.emit(AdiRouterBusKey.Changed, { path, params: {} }, PLUGIN_ID);
      },
      PLUGIN_ID,
    );

    this.bus.on(
      CommandBusKey.Execute,
      ({ id }) => {
        if (!id.startsWith(GOTO_PREFIX)) return;
        const path = id.slice(GOTO_PREFIX.length);
        this.navigate(path);
      },
      PLUGIN_ID,
    );

    window.addEventListener('popstate', this.onPopState);
  }

  navigate(path: string): void {
    history.pushState(null, '', path);
    this.currentPath = path;
    this.bus.emit(AdiRouterBusKey.Changed, { path, params: {} }, PLUGIN_ID);
  }

  override onUnregister(): void {
    window.removeEventListener('popstate', this.onPopState);
  }
}
