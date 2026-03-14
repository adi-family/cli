import { AdiPlugin, HttpPluginRegistry, loadPlugins } from '@adi-family/sdk-plugin';
import { AdiSignalingBusKey, type DeviceInfo, type IceServer } from '@adi-family/plugin-signaling';
import { AdiRouterBusKey } from '@adi-family/plugin-router';
import { PLUGIN_ID, PLUGIN_VERSION } from './config.js';
import { setBus } from './context.js';
import * as api from './api.js';
import type { CocoonDevice, CocoonInstallStatus, PluginItem, RegistryPlugin } from './types.js';
import './events.js';

interface TrackedCocoon {
  info: DeviceInfo;
  signalingUrl: string;
  installedPlugins: Map<string, string>;
  fetchedInstalled: boolean;
}

export class PluginsPlugin extends AdiPlugin {
  readonly id = PLUGIN_ID;
  readonly version = PLUGIN_VERSION;

  get api() { return this; }

  private readonly unsubs: (() => void)[] = [];
  private readonly cocoons = new Map<string, TrackedCocoon>();
  private iceServers: IceServer[] | undefined;
  private allPlugins: RegistryPlugin[] = [];
  private loadedWebPlugins = new Set<string>();

  override async onRegister(): Promise<void> {
    setBus(this.bus);

    const { AdiPluginsElement } = await import('./component.js');
    if (!customElements.get('adi-plugins')) {
      customElements.define('adi-plugins', AdiPluginsElement);
    }

    this.bus.emit(AdiRouterBusKey.RegisterRoute, {
      pluginId: this.id,
      path: '',
      init: () => document.createElement('adi-plugins'),
      label: 'Plugins',
    }, this.id);

    this.bus.emit('adi.actions-feed:nav-add', {
      id: this.id,
      label: 'Plugins',
      path: `/${this.id}`,
    }, this.id);

    this.unsubs.push(
      this.bus.on(AdiSignalingBusKey.ConnectionInfo, ({ connectionInfo }) => {
        this.iceServers = connectionInfo.ice_servers;
      }, PLUGIN_ID),
      this.bus.on(AdiSignalingBusKey.Devices, ({ url, devices }) => {
        for (const d of devices) {
          if (d.device_type === 'cocoon') {
            const existing = this.cocoons.get(d.device_id);
            this.cocoons.set(d.device_id, {
              info: d,
              signalingUrl: url,
              installedPlugins: existing?.installedPlugins ?? new Map(),
              fetchedInstalled: existing?.fetchedInstalled ?? false,
            });
          }
        }
      }, PLUGIN_ID),
      this.bus.on(AdiSignalingBusKey.DeviceDeregistered, ({ deviceId }) => {
        const tracked = this.cocoons.get(deviceId);
        if (tracked) {
          this.cocoons.set(deviceId, { ...tracked, info: { ...tracked.info, online: false } });
        }
      }, PLUGIN_ID),
    );

    this.bus.on('plugins:search', async ({ query }) => {
      try {
        const registryUrls = this.getRegistryUrls();
        const plugins = query.trim()
          ? (await Promise.allSettled(registryUrls.map(u => api.searchPlugins(u, query))))
              .flatMap(r => r.status === 'fulfilled' ? r.value : [])
          : await api.fetchAllPlugins(registryUrls);

        this.allPlugins = plugins;
        this.collectLoadedWebPlugins();
        const items = this.buildPluginItems(plugins);
        this.bus.emit('plugins:search-changed', {
          plugins: items,
          total: items.length,
          hasMore: false,
        }, PLUGIN_ID);
      } catch (err) {
        console.error('[PluginsPlugin] search error:', err);
        this.bus.emit('plugins:search-changed', { plugins: [], total: 0, hasMore: false }, PLUGIN_ID);
      }
    }, PLUGIN_ID);

    this.bus.on('plugins:install-web', async ({ pluginId }) => {
      try {
        const registryUrls = this.getRegistryUrls();
        const allDescs = await api.fetchAllPlugins(registryUrls);
        const target = allDescs.find(p => p.id === pluginId);
        if (!target) {
          this.bus.emit('plugins:install-result', { pluginId, success: false, error: 'Plugin not found in registry' }, PLUGIN_ID);
          return;
        }

        // Build a PluginDescriptor for the SDK's loadPlugins
        const registryUrl = registryUrls[0];
        const registry = new HttpPluginRegistry(registryUrl);
        await loadPlugins(this.bus, [{
          id: pluginId,
          installedVersion: target.latestVersion,
          registry,
        }]);

        this.loadedWebPlugins.add(pluginId);
        this.bus.emit('plugins:install-result', { pluginId, success: true }, PLUGIN_ID);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        this.bus.emit('plugins:install-result', { pluginId, success: false, error: msg }, PLUGIN_ID);
      }
    }, PLUGIN_ID);

    this.bus.on('plugins:install-cocoon', async ({ pluginId, cocoonId }) => {
      try {
        const session = await this.createCocoonSession(cocoonId);
        if (!session) {
          this.bus.emit('plugins:install-result', {
            pluginId, cocoonId, success: false, error: 'Cannot connect to cocoon',
          }, PLUGIN_ID);
          return;
        }

        const { exitCode, output } = await api.executeOnCocoon(session, `adi plugin install ${pluginId}`);
        const success = exitCode === 0;
        this.bus.emit('plugins:install-result', {
          pluginId, cocoonId, success,
          error: success ? undefined : output.slice(0, 500),
        }, PLUGIN_ID);

        if (success) {
          const tracked = this.cocoons.get(cocoonId);
          if (tracked) {
            tracked.installedPlugins.set(pluginId, 'latest');
          }
        }
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        this.bus.emit('plugins:install-result', { pluginId, cocoonId, success: false, error: msg }, PLUGIN_ID);
      }
    }, PLUGIN_ID);
  }

  override onUnregister(): void {
    this.unsubs.forEach(fn => fn());
    this.unsubs.length = 0;
    this.cocoons.clear();
  }

  private getRegistryUrls(): string[] {
    return this.app.env('DEFAULT_REGISTRY_URLS');
  }

  private collectLoadedWebPlugins(): void {
    this.loadedWebPlugins.clear();
    for (const id of this.app.registeredPlugins) {
      this.loadedWebPlugins.add(id);
    }
  }

  private buildPluginItems(plugins: RegistryPlugin[]): PluginItem[] {
    return plugins.map(plugin => {
      const webInstalled = this.loadedWebPlugins.has(plugin.id);
      const cocoonStatuses: CocoonInstallStatus[] = [];

      for (const [deviceId, tracked] of this.cocoons) {
        if (!tracked.info.online) continue;
        cocoonStatuses.push({
          cocoonId: deviceId,
          cocoonName: tracked.info.tags?.name ?? deviceId.slice(0, 8),
          installed: tracked.installedPlugins.has(plugin.id),
          installedVersion: tracked.installedPlugins.get(plugin.id),
          installing: false,
        });
      }

      return { plugin, webInstalled, webInstalling: false, cocoonStatuses };
    });
  }

  getCocoonDevices(): CocoonDevice[] {
    const devices: CocoonDevice[] = [];
    for (const [deviceId, tracked] of this.cocoons) {
      devices.push({
        deviceId,
        signalingUrl: tracked.signalingUrl,
        name: tracked.info.tags?.name,
        online: tracked.info.online,
      });
    }
    return devices;
  }

  private async createCocoonSession(cocoonId: string) {
    const cocoonApi = await this.app.api('adi.cocoon');
    let client = cocoonApi.getClient(cocoonId);

    if (!client) {
      const tracked = this.cocoons.get(cocoonId);
      if (!tracked) return undefined;
      const rtcConfig = this.iceServers ? { iceServers: this.iceServers } : undefined;
      client = await cocoonApi.createClient(cocoonId, tracked.signalingUrl, rtcConfig);
    }

    if (!client) return undefined;

    try {
      return await client.createSession();
    } catch {
      return undefined;
    }
  }
}
