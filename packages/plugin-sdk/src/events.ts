import type {} from './types.js';

declare module './types.js' {
  interface EventRegistry {
    // --- Plugin lifecycle ---

    /** Auto-emitted by AdiPlugin after onRegister() resolves. */
    'register-finished': { pluginId: string };

    /** Emitted by loadPlugins() when all plugins are done (success/fail/timeout). */
    'loading-finished': {
      loaded: string[];
      failed: string[];
      timedOut: string[];
    };

    // --- Plugin versioning ---

    /** SW detected a newer version. App decides when to call upgradePlugin(). */
    'plugin:update-available': {
      pluginId: string;
      currentVersion: string;
      newVersion: string;
      newUrl: string;
    };

    /** Hot-swap started — old plugin torn down, new version loading. */
    'plugin:upgrading': {
      pluginId: string;
      fromVersion: string;
      toVersion: string;
    };

    /** Hot-swap completed successfully. */
    'plugin:upgraded': {
      pluginId: string;
      fromVersion: string;
      toVersion: string;
    };

    /** Hot-swap failed. */
    'plugin:upgrade-failed': { pluginId: string; reason: string };

    // --- UI registration ---

    /** Register a client-side route. element = custom element tag to render. */
    'route:register': { path: string; element: string; label?: string };

    /** Add a nav item. */
    'nav:add': {
      id: string;
      label: string;
      path: string;
      icon?: string;
    };

    /** Reply to nav:add. Host emits after adding the nav item. */
    'nav:add:ok': { id: string; _cid: string };

    /** Register a command palette entry. */
    'command:register': { id: string; label: string; shortcut?: string };

    /** Execute a registered command by id. */
    'command:execute': { id: string };

    /** Programmatically open the command palette, optionally pre-filling a query. */
    'command-palette:open': { query?: string };

    // --- App lifecycle ---

    /** Host emits after mounting, ready to receive plugin events. */
    'app:ready': void;

    /** Emitted when active theme or color mode changes. */
    'app:theme-changed': { theme: string; mode: 'dark' | 'light' };

    // --- Router ---

    /** Plugin emits to trigger navigation. */
    'router:navigate': { path: string; replace?: boolean };

    /** Host emits after every route change. */
    'router:changed': { path: string; params: Record<string, string> };
  }
}

// No runtime exports — this file exists purely for its declaration merging side-effect.
export {};
