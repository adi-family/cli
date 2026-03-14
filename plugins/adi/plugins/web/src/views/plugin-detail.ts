import { html, nothing, type TemplateResult } from 'lit';
import type { CocoonInstallStatus, PluginItem } from '../types.js';

export interface PluginDetailProps {
  item: PluginItem;
  onBack: () => void;
  onInstallWeb: () => void;
  onInstallCocoon: (cocoonId: string) => void;
}

const statusRow = (
  status: CocoonInstallStatus,
  onInstall: (cocoonId: string) => void,
): TemplateResult => html`
  <div class="flex items-center justify-between py-2 border-b border-white/5 last:border-0">
    <div class="flex items-center gap-2">
      <span class="w-2 h-2 rounded-full ${status.installed ? 'bg-green-400' : 'bg-gray-600'}"></span>
      <span class="text-sm text-gray-300">${status.cocoonName}</span>
      ${status.installedVersion
        ? html`<span class="text-xs text-gray-500">v${status.installedVersion}</span>`
        : nothing}
    </div>
    <div>
      ${status.installing
        ? html`<span class="text-xs text-gray-400">Installing...</span>`
        : status.installed
          ? html`<span class="plugins-installed-badge">Installed</span>`
          : html`<button class="plugins-btn-primary text-xs" @click=${() => onInstall(status.cocoonId)}>Install</button>`}
    </div>
  </div>
`;

export const renderPluginDetail = (props: PluginDetailProps): TemplateResult => {
  const { item, onBack, onInstallWeb, onInstallCocoon } = props;
  const { plugin } = item;
  const hasWeb = plugin.pluginTypes.some(t => t === 'web' || t === 'extension');
  const hasCocoon = plugin.pluginTypes.some(t => t !== 'web' && t !== 'extension');

  return html`
    <div class="p-4 max-w-3xl mx-auto">
      <button
        class="flex items-center gap-1 text-sm text-gray-400 hover:text-gray-200 mb-4"
        @click=${onBack}
      >Back to list</button>

      <div class="plugins-card">
        <div class="flex items-start justify-between gap-4 mb-4">
          <div>
            <h1 class="text-xl font-semibold text-gray-200">${plugin.name}</h1>
            <p class="text-sm text-gray-500 mt-1">${plugin.id}</p>
          </div>
          <span class="px-2 py-1 rounded-md bg-white/5 text-sm text-gray-400">v${plugin.latestVersion}</span>
        </div>

        <p class="text-sm text-gray-300 mb-4">${plugin.description || 'No description provided.'}</p>

        <div class="flex flex-wrap gap-4 text-xs text-gray-500 mb-4">
          <span>Author: ${plugin.author}</span>
          <span>Downloads: ${plugin.downloads.toLocaleString()}</span>
          <span>Types: ${plugin.pluginTypes.join(', ')}</span>
        </div>

        ${plugin.tags.length > 0
          ? html`<div class="flex flex-wrap gap-1 mb-4">${plugin.tags.map(t => html`<span class="plugins-tag">${t}</span>`)}</div>`
          : nothing}
      </div>

      ${hasWeb ? html`
        <div class="plugins-card mt-3">
          <h2 class="text-sm font-semibold text-gray-300 mb-3">Web Plugin</h2>
          <div class="flex items-center justify-between">
            <span class="text-sm text-gray-400">Load plugin in browser</span>
            ${item.webInstalled
              ? html`<span class="plugins-installed-badge">Loaded</span>`
              : item.webInstalling
                ? html`<span class="text-xs text-gray-400">Loading...</span>`
                : html`<button class="plugins-btn-primary" @click=${onInstallWeb}>Install Web Plugin</button>`}
          </div>
        </div>
      ` : nothing}

      ${hasCocoon ? html`
        <div class="plugins-card mt-3">
          <h2 class="text-sm font-semibold text-gray-300 mb-3">Cocoon Installations</h2>
          ${item.cocoonStatuses.length > 0
            ? item.cocoonStatuses.map(s => statusRow(s, onInstallCocoon))
            : html`<p class="text-sm text-gray-500 text-center py-4">No cocoons connected. Connect a cocoon to install this plugin.</p>`}
        </div>
      ` : nothing}
    </div>
  `;
};
