import { html, nothing, type TemplateResult } from 'lit';
import type { CocoonInstallStatus, PluginItem } from '../types.js';

export interface PluginCardProps {
  item: PluginItem;
  onInstallWeb: () => void;
  onInstallCocoon: (cocoonId: string) => void;
  onSelect: () => void;
}

const cocoonInstallButton = (
  status: CocoonInstallStatus,
  onInstall: (cocoonId: string) => void,
): TemplateResult => {
  if (status.installing) {
    return html`<span class="text-xs text-gray-400">Installing...</span>`;
  }
  if (status.installed) {
    return html`<span class="plugins-installed-badge">Installed ${status.installedVersion ?? ''}</span>`;
  }
  return html`
    <button
      class="plugins-btn-secondary text-xs"
      @click=${(e: Event) => { e.stopPropagation(); onInstall(status.cocoonId); }}
    >Install</button>
  `;
};

export const renderPluginCard = (props: PluginCardProps): TemplateResult => {
  const { item, onInstallWeb, onInstallCocoon, onSelect } = props;
  const { plugin } = item;
  const hasWeb = plugin.pluginTypes.some(t => t === 'web' || t === 'extension');
  const hasCocoon = plugin.pluginTypes.some(t => t !== 'web' && t !== 'extension');

  return html`
    <div class="plugins-card cursor-pointer" @click=${onSelect}>
      <div class="flex items-start justify-between gap-3 mb-2">
        <div class="min-w-0">
          <h3 class="text-sm font-semibold text-gray-200 truncate">${plugin.name}</h3>
          <p class="text-xs text-gray-500 truncate">${plugin.id}</p>
        </div>
        <span class="text-xs text-gray-500 whitespace-nowrap">v${plugin.latestVersion}</span>
      </div>

      <p class="text-xs text-gray-400 mb-3 line-clamp-2" style="display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;">
        ${plugin.description || 'No description'}
      </p>

      ${plugin.tags.length > 0
        ? html`<div class="flex flex-wrap gap-1 mb-3">${plugin.tags.slice(0, 3).map(t => html`<span class="plugins-tag">${t}</span>`)}</div>`
        : nothing}

      <div class="flex items-center justify-between gap-2 pt-2 border-t border-white/5">
        <span class="text-xs text-gray-500">${plugin.author}</span>
        <div class="flex items-center gap-2" @click=${(e: Event) => e.stopPropagation()}>
          ${hasWeb
            ? item.webInstalled
              ? html`<span class="plugins-installed-badge">Web</span>`
              : item.webInstalling
                ? html`<span class="text-xs text-gray-400">Loading...</span>`
                : html`<button class="plugins-btn-primary text-xs" @click=${onInstallWeb}>Install Web</button>`
            : nothing}
          ${hasCocoon && item.cocoonStatuses.length > 0
            ? html`
              <div class="flex items-center gap-1">
                ${item.cocoonStatuses.map(s => html`
                  <div class="flex items-center gap-1 text-xs">
                    <span class="text-gray-500 truncate max-w-[60px]" title=${s.cocoonName}>${s.cocoonName}</span>
                    ${cocoonInstallButton(s, onInstallCocoon)}
                  </div>
                `)}
              </div>
            `
            : nothing}
        </div>
      </div>
    </div>
  `;
};
