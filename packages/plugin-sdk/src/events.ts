import type { RegistryHealth } from './registry-http.js';
import type {} from './types.js';

declare module './types.js' {
  interface EventRegistry {
    // --- Plugin lifecycle ---

    /** Auto-emitted by AdiPlugin after onRegister() resolves. */
    'register-finished': { pluginId: string };

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

    'plugin:upgrading': {
      pluginId: string;
      fromVersion: string;
      toVersion: string;
    };

    'plugin:upgraded': {
      pluginId: string;
      fromVersion: string;
      toVersion: string;
    };

    'plugin:upgrade-failed': { pluginId: string; reason: string };

    /** Emitted when a plugin is installed (auto-required or restored from prefs). */
    'plugin:installed': { pluginId: string; reason: 'auto' | 'restored' };

    // --- UI registration ---

    /** Register a client-side route. element = custom element tag to render. */
    'route:register': { path: string; element: string; label?: string };

    'nav:add': {
      id: string;
      label: string;
      path: string;
      icon?: string;
    };

    'command:register': { id: string; label: string; shortcut?: string };
    'command:execute': { id: string };
    'command-palette:open': { query?: string };

    // --- App lifecycle ---

    /** Host emits after mounting, ready to receive plugin events. */
    'app:ready': void;

    'app:theme-changed': { theme: string; mode: 'dark' | 'light' };

    // --- Router ---

    /** Plugin emits to trigger navigation. */
    'router:navigate': { path: string; replace?: boolean };

    /** Host emits after every route change. */
    'router:changed': { path: string; params: Record<string, string> };

    // --- Actions loop ---

    /** Register a kind with a display mode. 'exclusive' = only one action of this plugin+kind visible at a time. */
    'actions:register-kind': {
      plugin: string;
      kind: string;
      mode: 'exclusive';
    };

    /** Push an action card into the floating overlay. Same id replaces existing. */
    'actions:push': {
      id: string;
      plugin: string;
      kind: string;
      data: Record<string, unknown>;
      priority?: 'low' | 'normal' | 'urgent';
    };

    'actions:dismiss': { id: string };

    'actions:register-renderer': {
      plugin: string;
      kind: string;
      render: (data: Record<string, unknown>, actionId: string) => string;
    };

    'actions:dismissed': { id: string; plugin: string; kind: string };

    // --- Registry ---

    'registry:health': { url: string; health: RegistryHealth };
    'registry:added': { url: string };
    'registry:removed': { url: string };

    // --- Database ---

    'db:connected': Record<string, never>;
    'db:disconnected': { reason: 'closed' | 'version-change' };
    'db:reconnecting': { store: string; mode: string };
    'db:error': { error: string; store?: string; mode?: string };
  }
}

// No runtime exports — this file exists purely for its declaration merging side-effect.
export {};
