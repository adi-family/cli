import { LitElement, html } from "lit";
import { customElement, property, state } from "lit/decorators.js";
import { unsafeSVG } from "lit/directives/unsafe-svg.js";
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
} from "lucide";
import {
  type Credential,
  credentialsApi,
} from "../services/credentials-api";
import { withMinLoadingTime } from "../config";

const icon = (iconData: typeof Key) => unsafeSVG(createElement(iconData).outerHTML);

@customElement("credentials-list")
export class CredentialsList extends LitElement {
  @property({ type: Boolean })
  visible = false;

  @state()
  private credentials: Credential[] = [];

  @state()
  private loading = false;

  @state()
  private error: string | null = null;

  @state()
  private selectedId: string | null = null;

  @state()
  private revealedIds = new Set<string>();

  createRenderRoot() {
    return this;
  }

  connectedCallback() {
    super.connectedCallback();
    this.loadCredentials();
  }

  async loadCredentials() {
    this.loading = true;
    this.error = null;

    try {
      this.credentials = await withMinLoadingTime(credentialsApi.list());
    } catch (e) {
      this.error = e instanceof Error ? e.message : "Failed to load credentials";
      this.credentials = [];
    } finally {
      this.loading = false;
    }
  }

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
      await credentialsApi.delete(credential.id);
      await this.loadCredentials();
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
      const data = await credentialsApi.getWithData(credential.id);
      const text = typeof data.data === "string" ? data.data : JSON.stringify(data.data);
      await navigator.clipboard.writeText(text);
      // Show toast notification
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

  private renderCredentialCard(credential: Credential) {
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
          ${this.getStatusBadge(credential)}
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

  render() {
    if (!this.visible) return null;

    return html`
      <div class="credentials-container">
        <div class="credentials-header">
          <div class="credentials-header__left">
            <h2 class="credentials-title">Credentials</h2>
            <span class="credentials-count">${this.credentials.length} items</span>
          </div>
          <div class="credentials-header__actions">
            <button
              class="credentials-btn credentials-btn--secondary"
              @click=${() => this.loadCredentials()}
              ?disabled=${this.loading}
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

        ${this.error
          ? html`
              <div class="credentials-error">
                ${icon(AlertTriangle)}
                ${this.error}
              </div>
            `
          : ""}

        ${this.loading
          ? html`
              <div class="credentials-loading">
                <div class="credentials-spinner"></div>
                Loading credentials...
              </div>
            `
          : html`
              <div class="credentials-grid">
                ${this.credentials.length === 0
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
                  : this.credentials.map((cred) => this.renderCredentialCard(cred))}
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
