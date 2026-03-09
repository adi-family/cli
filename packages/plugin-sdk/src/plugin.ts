import type { EventBus } from './bus.js';
import type { AppContext } from './app-context.js';

export abstract class AdiPlugin {
  abstract readonly id: string;
  abstract readonly version: string;
  readonly dependencies: string[] = [];
  readonly requires: string[] = [];

  #app: AppContext | undefined;

  protected get app(): AppContext {
    if (!this.#app) {
      throw new Error(`Plugin '${this.id}' accessed app before _init() was called`);
    }
    return this.#app;
  }

  /** Shorthand for this.app.bus. */
  protected get bus(): EventBus {
    return this.app.bus;
  }

  onRegister?(): Promise<void> | void;
  onUnregister?(): Promise<void> | void;

  /** @internal SDK use only. */
  async _init(app: AppContext): Promise<void> {
    this.#app = app;
    const api = (this as Record<string, unknown>)['api'];
    if (api !== undefined) app._provide(this.id, api);
    await this.onRegister?.();
    app._registerPlugin(this.id);
    app.bus.emit('register-finished', { pluginId: this.id }, `plugin:${this.id}`);
  }

  /** @internal SDK use only. */
  async _destroy(): Promise<void> {
    await this.onUnregister?.();
    this.#app?._unregisterPlugin(this.id);
  }
}
