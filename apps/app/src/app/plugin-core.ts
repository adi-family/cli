import {
  type AdiPlugin,
  type EventBus,
  initInternalPlugin,
} from '@adi-family/sdk-plugin';

export class PluginCore {
  private readonly plugins = new Map<string, AdiPlugin>();

  constructor(private readonly bus: EventBus) {}

  async install(plugin: AdiPlugin): Promise<void> {
    await initInternalPlugin(this.bus, plugin);
    this.plugins.set(plugin.id, plugin);
  }

  get<T extends AdiPlugin>(id: string): T | undefined {
    return this.plugins.get(id) as T | undefined;
  }

  has(id: string): boolean {
    return this.plugins.has(id);
  }

  ids(): string[] {
    return [...this.plugins.keys()];
  }
}
