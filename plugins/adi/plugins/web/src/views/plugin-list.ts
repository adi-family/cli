import { html, nothing, type TemplateResult } from 'lit';
import type { PluginFilter, PluginItem } from '../models.js';
import { renderPluginRow } from './plugin-card.js';

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

const TABS: { key: PluginFilter; label: string }[] = [
  { key: 'web', label: 'Web' },
  { key: 'installed', label: 'Installed' },
];

const filterPlugins = (plugins: PluginItem[], filter: PluginFilter): PluginItem[] => {
  switch (filter) {
    case 'web':
      return plugins.filter(p => p.plugin.pluginTypes.some(t => t === 'web' || t === 'extension'));
    case 'installed':
      return plugins.filter(p => p.webStatus.kind === 'installed' || p.cocoonStatuses.some(s => s.installed));
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
        <h1 class="text-lg font-semibold" style="color: var(--adi-text)">Plugins</h1>
        <span class="text-xs" style="color: var(--adi-text-muted)">${filtered.length} plugin${filtered.length !== 1 ? 's' : ''}</span>
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
          class="w-full px-3 py-2 rounded-lg text-sm focus:outline-none"
          style="background: var(--adi-surface-alt); border: 1px solid var(--adi-border); color: var(--adi-text);"
        />
      </div>

      <div class="flex gap-1 mb-4">
        ${TABS.map(t => html`
          <button
            class="plugins-tab ${props.filter === t.key ? 'plugins-tab--active' : ''}"
            @click=${() => props.onFilterChange(t.key)}
          >${t.label}</button>
        `)}
      </div>

      ${props.error
        ? html`<div class="p-3 rounded-lg text-sm mb-4" style="color: var(--adi-error);">${props.error}</div>`
        : nothing}

      ${props.loading
        ? html`<div class="text-center py-12 text-sm" style="color: var(--adi-text-muted)">Loading plugins...</div>`
        : filtered.length === 0
          ? html`<div class="text-center py-12 text-sm" style="color: var(--adi-text-muted)">No plugins found</div>`
          : html`
            <div class="plugins-table">
              <div class="plugins-table-header">
                <span class="plugins-col-name">Name</span>
                <span class="plugins-col-version">Version</span>
                <span class="plugins-col-author">Author</span>
                <span class="plugins-col-status">Status</span>
              </div>
              ${filtered.map(item => renderPluginRow({
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
