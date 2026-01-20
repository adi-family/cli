import { LitElement, html } from "lit";
import { customElement, property, state } from "lit/decorators.js";
import { unsafeSVG } from "lit/directives/unsafe-svg.js";
import { createElement, X, Save, Key, AlertCircle } from "lucide";
import {
  type Credential,
  type CreateCredential,
  type UpdateCredential,
  credentialsApi,
} from "../services/credentials-api";

const icon = (iconData: typeof Key) => unsafeSVG(createElement(iconData).outerHTML);

@customElement("credentials-form")
export class CredentialsForm extends LitElement {
  @property({ type: Boolean, reflect: true })
  open = false;

  @property({ type: Object })
  credential: Credential | null = null;

  @state()
  private name = "";

  @state()
  private description = "";

  @state()
  private data = "";

  @state()
  private provider = "";

  @state()
  private expiresAt = "";

  @state()
  private loading = false;

  @state()
  private error: string | null = null;

  createRenderRoot() {
    return this;
  }

  updated(changedProperties: Map<string, unknown>) {
    if (changedProperties.has("credential") || changedProperties.has("open")) {
      if (this.open) {
        this.resetForm();
      }
    }
  }

  private resetForm() {
    if (this.credential) {
      this.name = this.credential.name;
      this.description = this.credential.description || "";
      this.provider = this.credential.provider || "";
      this.expiresAt = this.credential.expiresAt
        ? this.credential.expiresAt.split("T")[0]
        : "";
      this.data = ""; // Don't prefill sensitive data
    } else {
      this.name = "";
      this.description = "";
      this.data = "";
      this.provider = "";
      this.expiresAt = "";
    }
    this.error = null;
  }

  private handleClose() {
    this.open = false;
    this.dispatchEvent(
      new CustomEvent("close", { bubbles: true, composed: true })
    );
  }

  private async handleSubmit(e: Event) {
    e.preventDefault();

    if (!this.name.trim()) {
      this.error = "Name is required";
      return;
    }

    if (!this.credential && !this.data.trim()) {
      this.error = "Credential data is required";
      return;
    }

    this.loading = true;
    this.error = null;

    try {
      let parsedData: Record<string, unknown>;
      if (this.data.trim()) {
        try {
          parsedData = JSON.parse(this.data);
        } catch {
          // If not JSON, wrap in object
          parsedData = { value: this.data };
        }
      } else {
        parsedData = {};
      }

      if (this.credential) {
        // Update
        const input: UpdateCredential = {
          name: this.name.trim(),
          description: this.description.trim() || undefined,
          provider: this.provider.trim() || undefined,
          expiresAt: this.expiresAt ? new Date(this.expiresAt).toISOString() : undefined,
        };
        if (this.data.trim()) {
          input.data = parsedData;
        }
        await credentialsApi.update(this.credential.id, input);
      } else {
        // Create - credentialType is required but we default to 'custom'
        const input: CreateCredential = {
          name: this.name.trim(),
          description: this.description.trim() || undefined,
          credentialType: "custom" as any,
          data: parsedData,
          provider: this.provider.trim() || undefined,
          expiresAt: this.expiresAt ? new Date(this.expiresAt).toISOString() : undefined,
        };
        await credentialsApi.create(input);
      }

      this.dispatchEvent(
        new CustomEvent("saved", { bubbles: true, composed: true })
      );
      this.handleClose();
    } catch (e) {
      this.error = e instanceof Error ? e.message : "Failed to save";
    } finally {
      this.loading = false;
    }
  }

  private handleBackdropClick(e: Event) {
    if ((e.target as HTMLElement).classList.contains("credentials-form__backdrop")) {
      this.handleClose();
    }
  }

  render() {
    if (!this.open) return null;

    const isEdit = !!this.credential;

    return html`
      <div class="credentials-form__backdrop" @click=${this.handleBackdropClick}>
        <div class="credentials-form">
          <div class="credentials-form__header">
            <h2 class="credentials-form__title">
              ${icon(Key)}
              ${isEdit ? "Edit Credential" : "Add Credential"}
            </h2>
            <button
              class="credentials-btn credentials-btn--icon"
              @click=${this.handleClose}
              title="Close"
            >
              ${icon(X)}
            </button>
          </div>

          ${this.error
            ? html`
                <div class="credentials-form__error">
                  ${icon(AlertCircle)}
                  ${this.error}
                </div>
              `
            : ""}

          <form class="credentials-form__body" @submit=${this.handleSubmit}>
            <div class="credentials-form__field">
              <label class="credentials-form__label">Name *</label>
              <input
                type="text"
                class="credentials-form__input"
                placeholder="My API Key"
                .value=${this.name}
                @input=${(e: Event) => (this.name = (e.target as HTMLInputElement).value)}
                required
              />
            </div>

            <div class="credentials-form__field">
              <label class="credentials-form__label">Description</label>
              <input
                type="text"
                class="credentials-form__input"
                placeholder="Optional description"
                .value=${this.description}
                @input=${(e: Event) =>
                  (this.description = (e.target as HTMLInputElement).value)}
              />
            </div>

            <div class="credentials-form__field">
              <label class="credentials-form__label">
                ${isEdit ? "New Value (leave empty to keep current)" : "Value *"}
              </label>
              <textarea
                class="credentials-form__textarea"
                placeholder=${isEdit
                  ? "Enter new value to update..."
                  : "Enter credential value (API key, token, JSON, etc.)"}
                .value=${this.data}
                @input=${(e: Event) => (this.data = (e.target as HTMLTextAreaElement).value)}
                rows="4"
              ></textarea>
              <span class="credentials-form__hint">
                Plain text or JSON. Will be encrypted before storage.
              </span>
            </div>

            <div class="credentials-form__row">
              <div class="credentials-form__field">
                <label class="credentials-form__label">Provider</label>
                <input
                  type="text"
                  class="credentials-form__input"
                  placeholder="e.g., github.com, openai.com"
                  .value=${this.provider}
                  @input=${(e: Event) =>
                    (this.provider = (e.target as HTMLInputElement).value)}
                />
              </div>

              <div class="credentials-form__field">
                <label class="credentials-form__label">Expires</label>
                <input
                  type="date"
                  class="credentials-form__input"
                  .value=${this.expiresAt}
                  @input=${(e: Event) =>
                    (this.expiresAt = (e.target as HTMLInputElement).value)}
                />
              </div>
            </div>

            <div class="credentials-form__actions">
              <button
                type="button"
                class="credentials-btn credentials-btn--secondary"
                @click=${this.handleClose}
              >
                Cancel
              </button>
              <button
                type="submit"
                class="credentials-btn credentials-btn--primary"
                ?disabled=${this.loading}
              >
                ${icon(Save)}
                ${this.loading ? "Saving..." : isEdit ? "Update" : "Create"}
              </button>
            </div>
          </form>
        </div>
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "credentials-form": CredentialsForm;
  }
}
