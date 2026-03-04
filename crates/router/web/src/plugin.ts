import { AdiPlugin } from '@adi-family/sdk-plugin';
import type { RouteRegisterEvent, NavAddEvent } from './generated/bus';
import './generated/bus';

const PLUGIN_ID = 'app.router';

export class RouterPlugin extends AdiPlugin {
  readonly id = PLUGIN_ID;
  readonly version = '0.1.0';

  routes: RouteRegisterEvent[] = [];
  navItems: NavAddEvent[] = [];
  currentPath = window.location.pathname;

  private readonly onPopState = () => {
    this.currentPath = window.location.pathname;
    this.bus.emit('router:changed', { path: this.currentPath, params: {} }, PLUGIN_ID);
  };

  override onRegister(): void {
    this.bus.on('route:register', ({ path, element, label }) => {
      if (!this.routes.some(r => r.path === path)) {
        this.routes = [...this.routes, { path, element, label }];
      }
    }, PLUGIN_ID);

    this.bus.on('nav:add', ({ id, label, path, icon }) => {
      if (!this.navItems.some(n => n.id === id)) {
        this.navItems = [...this.navItems, { id, label, path, icon }];
      }
    }, PLUGIN_ID);

    this.bus.on('router:navigate', ({ path, replace }) => {
      if (replace) history.replaceState(null, '', path);
      else history.pushState(null, '', path);
      this.currentPath = path;
      this.bus.emit('router:changed', { path, params: {} }, PLUGIN_ID);
    }, PLUGIN_ID);

    window.addEventListener('popstate', this.onPopState);
  }

  navigate(path: string): void {
    history.pushState(null, '', path);
    this.currentPath = path;
    this.bus.emit('router:changed', { path, params: {} }, PLUGIN_ID);
  }

  override onUnregister(): void {
    window.removeEventListener('popstate', this.onPopState);
  }
}
