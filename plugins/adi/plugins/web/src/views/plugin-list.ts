import { html, nothing, type TemplateResult } from 'lit';
import type { PluginFilter, PluginItem } from '../types.js';
import { renderPluginCard } from './plugin-card.js';

export interface PluginListProps {
  plugins: PluginItem[];
  searchQuery: string;
  filter: PluginFilter;
  loading: boolean;
  error: string | null;
  onSearch: (query: string) => void;
  onFilterChange: (filter: PluginFilter) => void;
  onInstallWeb: (pluginId: string) => void;
  onInstallCocoon: (pluginId: string, cocoonId: string) => void;
  onSelectPlugin: (pluginId: string) => void;
}

const FILTERS: { key: PluginFilter; label: string }[] = [
  { key: 'all', label: 'All' },
  { key: 'web', label: 'Web' },
  { key: 'cocoon', label: 'Cocoon' },
  { key: 'installed', label: 'Installed' },
];

const filterPlugins = (plugins: PluginItem[], filter: PluginFilter): PluginItem[] => {
  switch (filter) {
    case 'web':
      return plugins.filter(p => p.plugin.pluginTypes.some(t => t === 'web' || t === 'extension'));
    case 'cocoon':
      return plugins.filter(p => p.plugin.pluginTypes.some(t => t !== 'web' && t !== 'extension'));
    case 'installed':
      return plugins.filter(p => p.webInstalled || p.cocoonStatuses.some(s => s.installed));
    default:
      return plugins;
  }
};

let searchTimeout: ReturnType<typeof setTimeout> | undefined;

const debouncedSearch = (fn: (q: string) => void, query: string): void => {
  clearTimeout(searchTimeout);
  searchTimeout = setTimeout(() => fn(query), 300);
};

export const renderPluginList = (props: PluginListProps): TemplateResult => {
  const filtered = filterPlugins(props.plugins, props.filter);

  return html`
    <div class="p-4 max-w-5xl mx-auto">
      <div class="flex items-center justify-between mb-4">
        <h1 class="text-lg font-semibold text-gray-200">Plugins</h1>
        <span class="text-xs text-gray-500">${filtered.length} plugin${filtered.length !== 1 ? 's' : ''}</span>
      </div>

      <div class="mb-4">
        <input
          type="text"
          placeholder="Search plugins..."
          .value=${props.searchQuery}
          @input=${(e: InputEvent) => {
            const q = (e.target as HTMLInputElement).value;
            debouncedSearch(props.onSearch, q);
          }}
          class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50"
        />
      </div>

      <div class="flex gap-1 mb-4">
        ${FILTERS.map(f => html`
          <button
            class="px-3 py-1 rounded-full text-xs font-medium transition-colors ${
              props.filter === f.key
                ? 'bg-purple-500/20 text-purple-200'
                : 'text-gray-400 hover:text-gray-200 hover:bg-white/5'
            }"
            @click=${() => props.onFilterChange(f.key)}
          >${f.label}</button>
        `)}
      </div>

      ${props.error
        ? html`<div class="p-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-300 mb-4">${props.error}</div>`
        : nothing}

      ${props.loading
        ? html`<div class="text-center py-12 text-gray-500 text-sm">Loading plugins...</div>`
        : filtered.length === 0
          ? html`<div class="text-center py-12 text-gray-500 text-sm">No plugins found</div>`
          : html`
            <div class="grid gap-3 grid-cols-1 md:grid-cols-2 lg:grid-cols-3">
              ${filtered.map(item => renderPluginCard({
                item,
                onInstallWeb: () => props.onInstallWeb(item.plugin.id),
                onInstallCocoon: (cocoonId) => props.onInstallCocoon(item.plugin.id, cocoonId),
                onSelect: () => props.onSelectPlugin(item.plugin.id),
              }))}
            </div>
          `}
    </div>
  `;
};
