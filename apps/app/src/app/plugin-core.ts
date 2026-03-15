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
    reasons: Record<string, string>;
  }> {
    const all = dedupeById(await this.registryHub.fetchAllDescriptors());
    const foundIds = new Set(all.map((d) => d.id));
    const toLoad = all.filter((d) => this.pendingIds.has(d.id));
    const notFound = [...this.pendingIds].filter((id) => !foundIds.has(id));

    let loaded: string[] = [];
    let failed: string[] = [...notFound];
    let timedOut: string[] = [];
    let reasons: Record<string, string> = Object.fromEntries(
      notFound.map((id) => [id, 'plugin not found in registry']),
    );

    if (toLoad.length > 0) {
      const resultPromise = new Promise<{ loaded: string[]; failed: string[]; timedOut: string[]; reasons: Record<string, string> }>((resolve) => {
        const unsub = this.bus.on(
          'loading-finished',
          (result) => {
            unsub();
            resolve(result);
          },
          'app',
        );
      });

      await loadPlugins(this.bus, toLoad, { availablePlugins: all });
      const result = await resultPromise;
      loaded = result.loaded;
      failed = [...failed, ...result.failed];
      timedOut = result.timedOut;
      reasons = { ...reasons, ...result.reasons };
    }

    this.pendingIds.clear();
    return { allPlugins: all, loaded, failed, timedOut, reasons };
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
