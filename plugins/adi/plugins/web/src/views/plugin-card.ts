import { html, nothing, type TemplateResult } from 'lit';
import type { PluginItem } from '../models.js';

export interface PluginRowProps {
  item: PluginItem;
  onInstallWeb: () => void;
  onInstallCocoon: (cocoonId: string) => void;
  onSelect: () => void;
}

const renderStatus = (item: PluginItem, onInstallWeb: () => void): TemplateResult => {
  switch (item.webStatus.kind) {
    case 'loading':
      return html`<span class="plugins-status-text">Loading...</span>`;
    case 'installing':
      return html`<span class="plugins-status-text">Installing...</span>`;
    case 'installed':
      return html`<span class="plugins-installed-badge">Installed</span>`;
    case 'error':
      return html`<span class="plugins-status-error" title=${item.webStatus.message}>Error</span>`;
    case 'available':
      return html`
        <button
          class="plugins-btn-secondary text-xs"
          @click=${(e: Event) => { e.stopPropagation(); onInstallWeb(); }}
        >Install</button>
      `;
  }
};

export const renderPluginRow = (props: PluginRowProps): TemplateResult => {
  const { item, onInstallWeb, onSelect } = props;
  const { plugin } = item;

  return html`
    <div class="plugins-table-row" @click=${onSelect}>
      <span class="plugins-col-name">
        <span class="plugins-row-title">${plugin.name}</span>
        ${plugin.description
          ? html`<span class="plugins-row-desc">${plugin.description}</span>`
          : nothing}
      </span>
      <span class="plugins-col-version">v${plugin.latestVersion}</span>
      <span class="plugins-col-author">${plugin.author}</span>
      <span class="plugins-col-status" @click=${(e: Event) => e.stopPropagation()}>
        ${renderStatus(item, onInstallWeb)}
      </span>
    </div>
  `;
};
