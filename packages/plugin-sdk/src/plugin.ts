import type { EventBus } from './types.js';

export abstract class AdiPlugin {
  abstract readonly id: string;
  abstract readonly version: string;
  readonly dependencies: string[] = [];

  #bus: EventBus | undefined;

  protected get bus(): EventBus {
    if (!this.#bus) {
      throw new Error(`Plugin '${this.id}' accessed bus before _init() was called`);
    }
    return this.#bus;
  }

  onRegister?(): Promise<void> | void;
  onUnregister?(): Promise<void> | void;

  /** @internal SDK use only. */
  async _init(bus: EventBus): Promise<void> {
    this.#bus = bus;
    await this.onRegister?.();
    bus.emit('register-finished', { pluginId: this.id }, `plugin:${this.id}`);
  }

  /** @internal SDK use only. */
  async _destroy(): Promise<void> {
    await this.onUnregister?.();
  }
}
