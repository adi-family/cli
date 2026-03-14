import {
  type AdiPlugin,
  type EventBus,
  type PluginDescriptor,
  initInternalPlugin,
  loadPlugins,
} from '@adi-family/sdk-plugin';
import type { RegistryHub } from './registry-hub';

const dedupeById = (descriptors: PluginDescriptor[]): PluginDescriptor[] => {
  const seen = new Set<string>();
  return descriptors.filter((d) => {
    if (seen.has(d.id)) return false;
    seen.add(d.id);
    return true;
  });
};

export class PluginCore {
  private readonly plugins = new Map<string, AdiPlugin>();
  private readonly pendingIds = new Set<string>();

  constructor(
    private readonly bus: EventBus,
    private readonly registryHub: RegistryHub,
  ) {}

  async registerPlugin(plugin: AdiPlugin): Promise<void> {
    await initInternalPlugin(this.bus, plugin);
    this.plugins.set(plugin.id, plugin);
  }

  registerPluginById(id: string): void {
    this.pendingIds.add(id);
  }

  async fetchPlugins(): Promise<{
    allPlugins: PluginDescriptor[];
    loaded: string[];
    failed: string[];
    timedOut: string[];
  }> {
    const all = dedupeById(await this.registryHub.fetchAllDescriptors());
    const toLoad = all.filter((d) => this.pendingIds.has(d.id));

    let loaded: string[] = [];
    let failed: string[] = [];
    let timedOut: string[] = [];

    if (toLoad.length > 0) {
      const resultPromise = new Promise<{ loaded: string[]; failed: string[]; timedOut: string[] }>((resolve) => {
        const unsub = this.bus.on(
          'loading-finished',
          (result: { loaded: string[]; failed: string[]; timedOut: string[] }) => {
            unsub();
            resolve(result);
          },
          'app',
        );
      });

      await loadPlugins(this.bus, toLoad, { availablePlugins: all });
      ({ loaded, failed, timedOut } = await resultPromise);
    }

    this.pendingIds.clear();
    return { allPlugins: all, loaded, failed, timedOut };
  }

  dispose(): void {
    this.registryHub.dispose();
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
