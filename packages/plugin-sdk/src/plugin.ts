// src/plugin.ts
import type { EventBus } from './types.js';

export abstract class AdiPlugin {
  /** Unique identifier — must match PluginDescriptor.id. */
  abstract readonly id: string;

  /** Semver version — must match PluginDescriptor.installedVersion. */
  abstract readonly version: string;

  /** Plugin IDs that must complete onRegister() before this plugin starts. */
  readonly dependencies: string[] = [];

  /** Event bus — injected by SDK via _init(). Available inside onRegister(). */
  protected bus!: EventBus;

  /**
   * Emit routes, nav items, commands, etc. here.
   * Called after all declared dependencies have finished registering.
   * SDK emits 'register-finished' automatically after this resolves.
   */
  onRegister?(): Promise<void> | void;

  /** Clean up subscriptions, timers, etc. Called on teardown or upgrade. */
  onUnregister?(): Promise<void> | void;

  /** @internal SDK use only. */
  async _init(bus: EventBus): Promise<void> {
    this.bus = bus;
    await this.onRegister?.();
    bus.emit('register-finished', { pluginId: this.id });
  }

  /** @internal SDK use only. */
  async _destroy(): Promise<void> {
    await this.onUnregister?.();
  }
}
