/**
 * Credentials list component with multi-source reactive store.
 * 
 * Features:
 * - Displays credentials from all connected sources
 * - Source badge shows origin of each credential
 * - Offline indicator and pending sync count
 * - Automatic refresh on store changes via StoreController
 */

import { LitElement, html } from "lit";
import { customElement, property, state } from "lit/decorators.js";
import { unsafeSVG } from "lit/directives/unsafe-svg.js";
import { StoreController } from "@nanostores/lit";
import {
  createElement,
  Key,
  Plus,
  RefreshCw,
  Trash2,
  Eye,
  EyeOff,
  Clock,
  AlertTriangle,
  CheckCircle,
  Copy,
  MoreVertical,
  Cloud,
  CloudOff,
  Server,
} from "lucide";
import {
  credentialsStore,
  type Credential,
  type StoreItem,
} from "../stores/credentials";

const icon = (iconData: typeof Key) => unsafeSVG(createElement(iconData).outerHTML);

@customElement("credentials-list")
export class CredentialsList extends LitElement {
  @property({ type: Boolean })
  visible = false;

  // ===========================================================================
  // Reactive Store Bindings
  // ===========================================================================
  
  /** All credentials from all sources */
  private readonly credentials = new StoreController(this, credentialsStore.$items);
  
  /** Loading state */
  private readonly loading = new StoreController(this, credentialsStore.$loading);
  
  /** Error state */
  private readonly error = new StoreController(this, credentialsStore.$error);
  
  /** Online/offline status */
  private readonly online = new StoreController(this, credentialsStore.$online);
  
  /** Pending sync count */
  private readonly pendingCount = new StoreController(this, credentialsStore.$pendingCount);
  
  /** Connected sources status */
  private readonly sources = new StoreController(this, credentialsStore.$sources);

  // ===========================================================================
  // Local State
  // ===========================================================================

  @state()
  private selectedId: string | null = null;

  @state()
  private revealedIds = new Set<string>();

  createRenderRoot() {
    return this;
  }

  connectedCallback() {
    super.connectedCallback();
    // Initial load from all sources
    void credentialsStore.refresh();
  }

  // ===========================================================================
  // Event Handlers
  // ===========================================================================

  private handleAdd() {
    this.dispatchEvent(
      new CustomEvent("credential-add", { bubbles: true, composed: true })
    );
  }

  private handleEdit(credential: Credential) {
    this.dispatchEvent(
      new CustomEvent("credential-edit", {
        detail: { credential },
        bubbles: true,
        composed: true,
      })
    );
  }

  private async handleDelete(credential: Credential) {
    if (!confirm(`Delete credential "${credential.name}"? This cannot be undone.`)) {
      return;
    }

    try {
      await credentialsStore.delete(credential.id);
    } catch (e) {
      alert(e instanceof Error ? e.message : "Failed to delete");
    }
  }

  private toggleReveal(id: string) {
    if (this.revealedIds.has(id)) {
      this.revealedIds.delete(id);
    } else {
      this.revealedIds.add(id);
    }
    this.requestUpdate();
  }

  private async copyToClipboard(credential: Credential) {
    try {
      // Note: For sensitive data, we'd need to fetch from the source
      // This is a simplified version that copies the credential name
      await navigator.clipboard.writeText(credential.name);
      this.dispatchEvent(
        new CustomEvent("toast", {
          detail: { message: "Copied to clipboard" },
          bubbles: true,
          composed: true,
        })
      );
    } catch (e) {
      alert("Failed to copy: " + (e instanceof Error ? e.message : "Unknown error"));
    }
  }

  // ===========================================================================
  // Formatting Helpers
  // ===========================================================================

  private formatDate(dateStr: string | null): string {
    if (!dateStr) return "Never";
    const date = new Date(dateStr);
    return date.toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  }

  private formatRelativeTime(dateStr: string | null): string {
    if (!dateStr) return "Never";
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays === 0) return "Today";
    if (diffDays === 1) return "Yesterday";
    if (diffDays < 7) return `${diffDays} days ago`;
    if (diffDays < 30) return `${Math.floor(diffDays / 7)} weeks ago`;
    return this.formatDate(dateStr);
  }

  private isExpired(credential: Credential): boolean {
    if (!credential.expiresAt) return false;
    return new Date(credential.expiresAt) < new Date();
  }

  private isExpiringSoon(credential: Credential): boolean {
    if (!credential.expiresAt) return false;
    const expires = new Date(credential.expiresAt);
    const now = new Date();
    const diffDays = (expires.getTime() - now.getTime()) / (1000 * 60 * 60 * 24);
    return diffDays > 0 && diffDays <= 30;
  }

  // ===========================================================================
  // Render Methods
  // ===========================================================================

  private getStatusBadge(credential: Credential) {
    if (this.isExpired(credential)) {
      return html`
        <span class="credentials-badge credentials-badge--error">
          ${icon(AlertTriangle)} Expired
        </span>
      `;
    }
    if (this.isExpiringSoon(credential)) {
      return html`
        <span class="credentials-badge credentials-badge--warning">
          ${icon(Clock)} Expiring soon
        </span>
      `;
    }
    return html`
      <span class="credentials-badge credentials-badge--success">
        ${icon(CheckCircle)} Valid
      </span>
    `;
  }

  /**
   * Render source indicator badge.
   */
  private renderSourceBadge(sourceId: string, syncStatus: string) {
    const sourceStatus = this.sources.value?.find(s => s.id === sourceId);
    const sourceName = sourceStatus?.name ?? sourceId;
    
    // Icon based on source type
    const sourceIcon = sourceId.includes('cloud') ? Cloud : Server;
    
    // Style based on sync status
    const statusClass = syncStatus === 'synced' 
      ? 'credentials-badge--muted' 
      : 'credentials-badge--warning';
    
    return html`
      <span class="credentials-badge ${statusClass}" title="Source: ${sourceName}">
        ${icon(sourceIcon)}
        ${sourceName}
        ${syncStatus === 'pending' ? html`<span class="ml-1">(pending)</span>` : ''}
      </span>
    `;
  }

  private renderCredentialCard(item: StoreItem<Credential>) {
    const credential = item.data;
    const isSelected = this.selectedId === credential.id;

    return html`
      <div
        class="credentials-card ${isSelected ? "credentials-card--selected" : ""}"
        @click=${() => (this.selectedId = isSelected ? null : credential.id)}
      >
        <div class="credentials-card__header">
          <div class="credentials-card__icon">
            ${icon(Key)}
          </div>
          <div class="credentials-card__info">
            <h3 class="credentials-card__name">${credential.name}</h3>
            ${credential.provider ? html`<span class="credentials-card__type">${credential.provider}</span>` : ""}
          </div>
          <div class="credentials-card__badges">
            ${this.renderSourceBadge(item._meta.source, item._meta.syncStatus)}
            ${this.getStatusBadge(credential)}
          </div>
        </div>

        ${credential.description
          ? html`<p class="credentials-card__desc">${credential.description}</p>`
          : ""}

        <div class="credentials-card__meta">
          <span class="credentials-card__meta-item">
            ${icon(Clock)}
            Last used: ${this.formatRelativeTime(credential.lastUsedAt ?? null)}
          </span>
          ${credential.expiresAt
            ? html`
                <span class="credentials-card__meta-item">
                  Expires: ${this.formatDate(credential.expiresAt)}
                </span>
              `
            : ""}
        </div>

        <div class="credentials-card__actions">
          <button
            class="credentials-btn credentials-btn--icon"
            title="Copy value"
            @click=${(e: Event) => {
              e.stopPropagation();
              this.copyToClipboard(credential);
            }}
          >
            ${icon(Copy)}
          </button>
          <button
            class="credentials-btn credentials-btn--icon"
            title=${this.revealedIds.has(credential.id) ? "Hide" : "Reveal"}
            @click=${(e: Event) => {
              e.stopPropagation();
              this.toggleReveal(credential.id);
            }}
          >
            ${icon(this.revealedIds.has(credential.id) ? EyeOff : Eye)}
          </button>
          <button
            class="credentials-btn credentials-btn--icon"
            title="Edit"
            @click=${(e: Event) => {
              e.stopPropagation();
              this.handleEdit(credential);
            }}
          >
            ${icon(MoreVertical)}
          </button>
          <button
            class="credentials-btn credentials-btn--icon credentials-btn--danger"
            title="Delete"
            @click=${(e: Event) => {
              e.stopPropagation();
              this.handleDelete(credential);
            }}
          >
            ${icon(Trash2)}
          </button>
        </div>
      </div>
    `;
  }

  /**
   * Render offline/sync status bar.
   */
  private renderStatusBar() {
    const isOnline = this.online.value;
    const pending = this.pendingCount.value ?? 0;
    
    if (isOnline && pending === 0) {
      return null;
    }
    
    return html`
      <div class="credentials-status-bar ${!isOnline ? 'credentials-status-bar--offline' : ''}">
        ${!isOnline 
          ? html`
              <span class="credentials-status-bar__item">
                ${icon(CloudOff)}
                Offline - changes will sync when connected
              </span>
            `
          : ''}
        ${pending > 0
          ? html`
              <span class="credentials-status-bar__item">
                ${icon(RefreshCw)}
                ${pending} pending change${pending > 1 ? 's' : ''}
                <button 
                  class="credentials-btn credentials-btn--small"
                  @click=${() => credentialsStore.syncPending()}
                  ?disabled=${!isOnline}
                >
                  Sync now
                </button>
              </span>
            `
          : ''}
      </div>
    `;
  }

  render() {
    if (!this.visible) return null;

    const items = this.credentials.value ?? [];
    const isLoading = this.loading.value ?? false;
    const currentError = this.error.value;

    return html`
      <div class="page-container credentials-container">
        ${this.renderStatusBar()}
        
        <div class="credentials-header">
          <div class="credentials-header__left">
            <h2 class="credentials-title">Credentials</h2>
            <span class="credentials-count">${items.length} items</span>
          </div>
          <div class="credentials-header__actions">
            <button
              class="credentials-btn credentials-btn--secondary"
              @click=${() => credentialsStore.refresh()}
              ?disabled=${isLoading}
            >
              ${icon(RefreshCw)}
              Refresh
            </button>
            <button class="credentials-btn credentials-btn--primary" @click=${this.handleAdd}>
              ${icon(Plus)}
              Add Credential
            </button>
          </div>
        </div>

        ${currentError
          ? html`
              <div class="credentials-error">
                ${icon(AlertTriangle)}
                ${currentError.message}
              </div>
            `
          : ""}

        ${isLoading
          ? html`
              <div class="credentials-loading">
                <div class="credentials-spinner"></div>
                Loading credentials...
              </div>
            `
          : html`
              <div class="credentials-grid">
                ${items.length === 0
                  ? html`
                      <div class="credentials-empty">
                        ${icon(Key)}
                        <p>No credentials yet</p>
                        <button
                          class="credentials-btn credentials-btn--primary"
                          @click=${this.handleAdd}
                        >
                          Add your first credential
                        </button>
                      </div>
                    `
                  : items.map((item) => this.renderCredentialCard(item))}
              </div>
            `}
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "credentials-list": CredentialsList;
  }
}
