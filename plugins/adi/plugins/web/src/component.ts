import { LitElement } from 'lit';
import { state } from 'lit/decorators.js';
import type { PluginFilter, PluginItem, View } from './models.js';
import { renderPluginList } from './views/plugin-list.js';
import { renderPluginDetail } from './views/plugin-detail.js';
import { getBus } from './context.js';

export class AdiPluginsElement extends LitElement {
  @state() private plugins: PluginItem[] = [];
  @state() private searchQuery = '';
  @state() private filter: PluginFilter = 'web';
  @state() private view: View = 'list';
  @state() private selectedPluginId: string | null = null;
  @state() private loading = false;
  @state() private error: string | null = null;

  private unsubs: Array<() => void> = [];

  override createRenderRoot() { return this; }

  private get bus() { return getBus(); }

  override connectedCallback(): void {
    super.connectedCallback();
    this.unsubs.push(
      this.bus.on('plugins:search-changed', ({ plugins }) => {
        this.plugins = plugins;
        this.loading = false;
        this.error = null;
      }, 'plugins-ui'),
      this.bus.on('plugins:install-result', ({ pluginId, cocoonId, success, error }) => {
        this.plugins = this.plugins.map(item => {
          if (item.plugin.id !== pluginId) return item;

          if (cocoonId) {
            return {
              ...item,
              cocoonStatuses: item.cocoonStatuses.map(s =>
                s.cocoonId === cocoonId
                  ? { ...s, installing: false, installed: success || s.installed, error }
                  : s,
              ),
            };
          }

          return {
            ...item,
            webStatus: success
              ? { kind: 'installed' as const }
              : { kind: 'error' as const, message: error ?? 'Install failed' },
          };
        });
      }, 'plugins-ui'),
    );
    this.loadPlugins();
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.unsubs.forEach(fn => fn());
    this.unsubs = [];
  }

  private loadPlugins(): void {
    this.loading = true;
    this.error = null;
    this.bus.emit('plugins:search', {
      query: this.searchQuery,
      offset: 0,
      limit: 100,
    }, 'plugins-ui');
  }

  private handleSearch(query: string): void {
    this.searchQuery = query;
    this.loading = true;
    this.bus.emit('plugins:search', {
      query,
      offset: 0,
      limit: 100,
    }, 'plugins-ui');
  }

  private handleFilterChange(filter: PluginFilter): void {
    this.filter = filter;
  }

  private handleInstallWeb(pluginId: string): void {
    this.plugins = this.plugins.map(item =>
      item.plugin.id === pluginId ? { ...item, webStatus: { kind: 'installing' as const } } : item,
    );
    this.bus.emit('plugins:install-web', { pluginId }, 'plugins-ui');
  }

  private handleInstallCocoon(pluginId: string, cocoonId: string): void {
    this.plugins = this.plugins.map(item =>
      item.plugin.id === pluginId
        ? {
            ...item,
            cocoonStatuses: item.cocoonStatuses.map(s =>
              s.cocoonId === cocoonId ? { ...s, installing: true } : s,
            ),
          }
        : item,
    );
    this.bus.emit('plugins:install-cocoon', { pluginId, cocoonId }, 'plugins-ui');
  }

  private handleSelectPlugin(pluginId: string): void {
    this.selectedPluginId = pluginId;
    this.view = 'detail';
  }

  override render() {
    if (this.view === 'detail' && this.selectedPluginId) {
      const item = this.plugins.find(p => p.plugin.id === this.selectedPluginId);
      if (item) {
        return renderPluginDetail({
          item,
          onBack: () => { this.view = 'list'; this.selectedPluginId = null; },
          onInstallWeb: () => this.handleInstallWeb(item.plugin.id),
          onInstallCocoon: (cocoonId) => this.handleInstallCocoon(item.plugin.id, cocoonId),
        });
      }
    }

    return renderPluginList({
      plugins: this.plugins,
      searchQuery: this.searchQuery,
      filter: this.filter,
      loading: this.loading,
      error: this.error,
      onSearch: (q) => this.handleSearch(q),
      onFilterChange: (f) => this.handleFilterChange(f),
      onInstallWeb: (id) => this.handleInstallWeb(id),
      onInstallCocoon: (id, cocoonId) => this.handleInstallCocoon(id, cocoonId),
      onSelectPlugin: (id) => this.handleSelectPlugin(id),
    });
  }
}
