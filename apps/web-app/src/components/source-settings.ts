/**
 * Source settings component for managing credential sources.
 * 
 * Allows users to:
 * - View connected sources and their status
 * - Add new HTTP sources (local, cloud, custom endpoints)
 * - Remove sources
 * - Test source connectivity
 */

import { LitElement, html } from "lit";
import { customElement, property, state } from "lit/decorators.js";
import { unsafeSVG } from "lit/directives/unsafe-svg.js";
import { StoreController } from "@nanostores/lit";
import {
  createElement,
  Plus,
  Trash2,
  Server,
  Cloud,
  CloudOff,
  CheckCircle,
  AlertCircle,
  RefreshCw,
  Settings,
  Link,
  X,
  Wifi,
  WifiOff,
} from "lucide";
import {
  credentialsStore,
  addCredentialsSource,
  removeCredentialsSource,
} from "../stores/credentials";
import type { SourceStatus, SourceConfig } from "../stores/core";

const icon = (iconData: typeof Settings) => unsafeSVG(createElement(iconData).outerHTML);

/**
 * Preset source configurations for quick setup.
 */
const SOURCE_PRESETS = [
  {
    id: 'local',
    name: 'Local Server',
    baseUrl: 'http://localhost:8032/credentials',
    description: 'Local development server',
    icon: Server,
  },
  {
    id: 'cloud',
    name: 'Cloud (Production)',
    baseUrl: '/api/credentials/credentials',
    description: 'Main cloud storage',
    icon: Cloud,
  },
] as const;

@customElement("source-settings")
export class SourceSettings extends LitElement {
  @property({ type: Boolean })
  visible = false;

  // ===========================================================================
  // Reactive Store Bindings
  // ===========================================================================
  
  /** Connected sources status */
  private readonly sources = new StoreController(this, credentialsStore.$sources);
  
  /** Online/offline status */
  private readonly online = new StoreController(this, credentialsStore.$online);

  // ===========================================================================
  // Local State
  // ===========================================================================

  @state()
  private showAddForm = false;

  @state()
  private newSource: Partial<SourceConfig> = {
    id: '',
    name: '',
    baseUrl: '',
    priority: 5,
  };

  @state()
  private testing: string | null = null;

  @state()
  private testResults: Map<string, boolean> = new Map();

  @state()
  private error: string | null = null;

  createRenderRoot() {
    return this;
  }

  // ===========================================================================
  // Event Handlers
  // ===========================================================================

  private handleClose() {
    this.dispatchEvent(
      new CustomEvent("close", { bubbles: true, composed: true })
    );
  }

  private handleAddPreset(preset: typeof SOURCE_PRESETS[number]) {
    // Check if already exists
    const existing = this.sources.value?.find(s => s.id === preset.id);
    if (existing) {
      this.error = `Source "${preset.name}" is already connected`;
      return;
    }

    addCredentialsSource({
      id: preset.id,
      name: preset.name,
      baseUrl: preset.baseUrl,
      priority: preset.id === 'cloud' ? 10 : 5,
    });

    // Refresh to fetch from new source
    void credentialsStore.refresh();
  }

  private handleAddCustom() {
    this.error = null;

    // Validate
    if (!this.newSource.id?.trim()) {
      this.error = 'Source ID is required';
      return;
    }
    if (!this.newSource.name?.trim()) {
      this.error = 'Source name is required';
      return;
    }
    if (!this.newSource.baseUrl?.trim()) {
      this.error = 'Base URL is required';
      return;
    }

    // Check if already exists
    const existing = this.sources.value?.find(s => s.id === this.newSource.id);
    if (existing) {
      this.error = `Source with ID "${this.newSource.id}" already exists`;
      return;
    }

    // Validate URL format
    try {
      new URL(this.newSource.baseUrl, window.location.origin);
    } catch {
      this.error = 'Invalid URL format';
      return;
    }

    addCredentialsSource({
      id: this.newSource.id,
      name: this.newSource.name,
      baseUrl: this.newSource.baseUrl,
      priority: this.newSource.priority ?? 5,
    });

    // Reset form
    this.newSource = { id: '', name: '', baseUrl: '', priority: 5 };
    this.showAddForm = false;

    // Refresh to fetch from new source
    void credentialsStore.refresh();
  }

  private handleRemove(sourceId: string) {
    const source = this.sources.value?.find(s => s.id === sourceId);
    if (!source) return;

    if (!confirm(`Remove source "${source.name}"? Credentials from this source will no longer be visible.`)) {
      return;
    }

    removeCredentialsSource(sourceId);
  }

  private async handleTest(sourceId: string) {
    this.testing = sourceId;
    
    try {
      const source = credentialsStore.getSource(sourceId);
      if (!source) {
        this.testResults.set(sourceId, false);
        return;
      }
      
      const healthy = await source.healthCheck();
      this.testResults.set(sourceId, healthy);
    } catch {
      this.testResults.set(sourceId, false);
    } finally {
      this.testing = null;
      this.requestUpdate();
    }
  }

  private handleRefresh() {
    void credentialsStore.refresh();
  }

  private updateNewSource(field: keyof SourceConfig, value: string | number) {
    this.newSource = { ...this.newSource, [field]: value };
  }

  // ===========================================================================
  // Render Helpers
  // ===========================================================================

  private getSourceIcon(sourceId: string) {
    if (sourceId.includes('cloud')) return Cloud;
    if (sourceId.includes('local')) return Server;
    return Link;
  }

  private getStatusIcon(source: SourceStatus) {
    if (!this.online.value) return WifiOff;
    if (source.connected) return CheckCircle;
    return AlertCircle;
  }

  private getStatusClass(source: SourceStatus): string {
    if (!this.online.value) return 'settings-source__status--offline';
    if (source.connected) return 'settings-source__status--connected';
    return 'settings-source__status--error';
  }

  private renderSourceCard(source: SourceStatus) {
    const SourceIcon = this.getSourceIcon(source.id);
    const StatusIcon = this.getStatusIcon(source);
    const testResult = this.testResults.get(source.id);
    const isTesting = this.testing === source.id;

    return html`
      <div class="settings-source">
        <div class="settings-source__main">
          <div class="settings-source__icon">
            ${icon(SourceIcon)}
          </div>
          <div class="settings-source__info">
            <div class="settings-source__name">${source.name}</div>
            <div class="settings-source__id">${source.id}</div>
          </div>
          <div class="settings-source__status ${this.getStatusClass(source)}">
            ${icon(StatusIcon)}
            <span>
              ${!this.online.value ? 'Offline' : source.connected ? 'Connected' : 'Error'}
            </span>
          </div>
        </div>
        
        ${source.error ? html`
          <div class="settings-source__error">
            ${source.error}
          </div>
        ` : ''}
        
        ${testResult !== undefined ? html`
          <div class="settings-source__test-result ${testResult ? 'settings-source__test-result--success' : 'settings-source__test-result--error'}">
            ${testResult ? 'Connection test passed' : 'Connection test failed'}
          </div>
        ` : ''}
        
        <div class="settings-source__actions">
          <button
            class="settings-btn settings-btn--secondary settings-btn--small"
            @click=${() => this.handleTest(source.id)}
            ?disabled=${isTesting || !this.online.value}
          >
            ${icon(isTesting ? RefreshCw : Wifi)}
            ${isTesting ? 'Testing...' : 'Test'}
          </button>
          <button
            class="settings-btn settings-btn--danger settings-btn--small"
            @click=${() => this.handleRemove(source.id)}
            title="Remove source"
          >
            ${icon(Trash2)}
          </button>
        </div>
      </div>
    `;
  }

  private renderPresetCard(preset: typeof SOURCE_PRESETS[number]) {
    const isConnected = this.sources.value?.some(s => s.id === preset.id);
    const PresetIcon = preset.icon;

    return html`
      <div class="settings-preset ${isConnected ? 'settings-preset--connected' : ''}">
        <div class="settings-preset__icon">
          ${icon(PresetIcon)}
        </div>
        <div class="settings-preset__info">
          <div class="settings-preset__name">${preset.name}</div>
          <div class="settings-preset__desc">${preset.description}</div>
        </div>
        ${isConnected
          ? html`
              <span class="settings-preset__badge">
                ${icon(CheckCircle)} Connected
              </span>
            `
          : html`
              <button
                class="settings-btn settings-btn--primary settings-btn--small"
                @click=${() => this.handleAddPreset(preset)}
              >
                ${icon(Plus)} Add
              </button>
            `
        }
      </div>
    `;
  }

  private renderAddForm() {
    return html`
      <div class="settings-add-form">
        <div class="settings-add-form__header">
          <h4 class="settings-add-form__title">Add Custom Source</h4>
          <button
            class="settings-btn settings-btn--icon"
            @click=${() => this.showAddForm = false}
          >
            ${icon(X)}
          </button>
        </div>
        
        <div class="settings-add-form__body">
          <div class="settings-add-form__field">
            <label class="settings-add-form__label">Source ID</label>
            <input
              type="text"
              class="settings-add-form__input"
              placeholder="e.g., cloud-staging"
              .value=${this.newSource.id ?? ''}
              @input=${(e: Event) => this.updateNewSource('id', (e.target as HTMLInputElement).value)}
            />
            <span class="settings-add-form__hint">Unique identifier (no spaces)</span>
          </div>
          
          <div class="settings-add-form__field">
            <label class="settings-add-form__label">Display Name</label>
            <input
              type="text"
              class="settings-add-form__input"
              placeholder="e.g., Cloud (Staging)"
              .value=${this.newSource.name ?? ''}
              @input=${(e: Event) => this.updateNewSource('name', (e.target as HTMLInputElement).value)}
            />
          </div>
          
          <div class="settings-add-form__field">
            <label class="settings-add-form__label">Base URL</label>
            <input
              type="text"
              class="settings-add-form__input"
              placeholder="e.g., https://staging.api.example.com/credentials"
              .value=${this.newSource.baseUrl ?? ''}
              @input=${(e: Event) => this.updateNewSource('baseUrl', (e.target as HTMLInputElement).value)}
            />
            <span class="settings-add-form__hint">API endpoint for credentials</span>
          </div>
          
          <div class="settings-add-form__field">
            <label class="settings-add-form__label">Priority</label>
            <input
              type="number"
              class="settings-add-form__input"
              min="1"
              max="100"
              .value=${String(this.newSource.priority ?? 5)}
              @input=${(e: Event) => this.updateNewSource('priority', parseInt((e.target as HTMLInputElement).value, 10))}
            />
            <span class="settings-add-form__hint">Higher priority sources win conflicts (1-100)</span>
          </div>
        </div>
        
        <div class="settings-add-form__actions">
          <button
            class="settings-btn settings-btn--secondary"
            @click=${() => this.showAddForm = false}
          >
            Cancel
          </button>
          <button
            class="settings-btn settings-btn--primary"
            @click=${() => this.handleAddCustom()}
          >
            ${icon(Plus)} Add Source
          </button>
        </div>
      </div>
    `;
  }

  render() {
    if (!this.visible) return null;

    const connectedSources = this.sources.value ?? [];
    const isOnline = this.online.value;

    return html`
      <div class="settings-backdrop" @click=${this.handleClose}>
        <div class="settings-panel" @click=${(e: Event) => e.stopPropagation()}>
          <div class="settings-header">
            <h2 class="settings-title">
              ${icon(Settings)}
              Source Settings
            </h2>
            <div class="settings-header__actions">
              <button
                class="settings-btn settings-btn--secondary settings-btn--small"
                @click=${this.handleRefresh}
                ?disabled=${!isOnline}
              >
                ${icon(RefreshCw)} Refresh All
              </button>
              <button
                class="settings-btn settings-btn--icon"
                @click=${this.handleClose}
              >
                ${icon(X)}
              </button>
            </div>
          </div>
          
          ${!isOnline ? html`
            <div class="settings-offline-banner">
              ${icon(CloudOff)}
              You're offline. Source management is limited.
            </div>
          ` : ''}
          
          ${this.error ? html`
            <div class="settings-error">
              ${icon(AlertCircle)}
              ${this.error}
              <button class="settings-error__close" @click=${() => this.error = null}>
                ${icon(X)}
              </button>
            </div>
          ` : ''}
          
          <div class="settings-content">
            <!-- Connected Sources -->
            <section class="settings-section">
              <h3 class="settings-section__title">Connected Sources</h3>
              ${connectedSources.length === 0
                ? html`
                    <div class="settings-empty">
                      ${icon(Server)}
                      <p>No sources connected</p>
                      <span>Add a source below to start syncing credentials</span>
                    </div>
                  `
                : html`
                    <div class="settings-sources">
                      ${connectedSources.map(source => this.renderSourceCard(source))}
                    </div>
                  `
              }
            </section>
            
            <!-- Quick Add Presets -->
            <section class="settings-section">
              <h3 class="settings-section__title">Quick Add</h3>
              <div class="settings-presets">
                ${SOURCE_PRESETS.map(preset => this.renderPresetCard(preset))}
              </div>
            </section>
            
            <!-- Custom Source -->
            <section class="settings-section">
              <h3 class="settings-section__title">Custom Source</h3>
              ${this.showAddForm
                ? this.renderAddForm()
                : html`
                    <button
                      class="settings-btn settings-btn--secondary settings-btn--full"
                      @click=${() => this.showAddForm = true}
                    >
                      ${icon(Plus)} Add Custom Source
                    </button>
                  `
              }
            </section>
          </div>
        </div>
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "source-settings": SourceSettings;
  }
}
