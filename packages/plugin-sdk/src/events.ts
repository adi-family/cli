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
      reasons: Record<string, string>;
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

    // --- App lifecycle ---

    /** Host emits after mounting, ready to receive plugin events. */
    'app:ready': void;

    'app:theme-changed': { theme: string; mode: 'dark' | 'light' };

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
