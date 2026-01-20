type RouteChangeCallback = (route: string) => void;

class Router {
  private basePath = '/app';
  private validRoutes = ['board', 'credentials', 'components'];
  private defaultRoute = 'board';
  private listeners: RouteChangeCallback[] = [];
  private currentRoute: string;

  constructor() {
    this.currentRoute = this.getRouteFromUrl();
    window.addEventListener('popstate', () => {
      this.currentRoute = this.getRouteFromUrl();
      this.notify();
    });
  }

  private getRouteFromUrl(): string {
    let path = window.location.pathname;
    if (path.startsWith(this.basePath)) {
      path = path.slice(this.basePath.length);
    }
    path = path.replace(/^\//, '').replace(/\/$/, '');
    return this.validRoutes.includes(path) ? path : this.defaultRoute;
  }

  private buildPath(route: string): string {
    return `${this.basePath}/${route}`;
  }

  private notify() {
    this.listeners.forEach(cb => cb(this.currentRoute));
  }

  get route(): string {
    return this.currentRoute;
  }

  navigate(route: string) {
    if (!this.validRoutes.includes(route)) {
      console.warn(`Invalid route: ${route}`);
      route = this.defaultRoute;
    }

    if (route === this.currentRoute) return;

    this.currentRoute = route;
    const newPath = this.buildPath(route);
    history.pushState({ route }, '', newPath);
    this.notify();
  }

  subscribe(callback: RouteChangeCallback): () => void {
    this.listeners.push(callback);
    return () => {
      this.listeners = this.listeners.filter(cb => cb !== callback);
    };
  }

  init() {
    // Redirect base path to default route
    const currentPath = window.location.pathname.replace(/\/$/, '');
    if (currentPath === this.basePath || currentPath === '') {
      history.replaceState({ route: this.defaultRoute }, '', this.buildPath(this.defaultRoute));
    }
    this.notify();
  }
}

export const router = new Router();
