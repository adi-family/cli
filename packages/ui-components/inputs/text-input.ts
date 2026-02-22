import { LitElement, html } from "lit";
import { customElement, property, query } from "lit/decorators.js";

/// Text input field. Sizing via ADID AX system (--l, --t, --r).
@customElement("adi-text-input")
export class AdiTextInput extends LitElement {
  @property({ type: String }) value = "";
  @property({ type: String }) placeholder = "Enter text...";
  @property({ type: String }) label = "";
  @property({ type: Boolean }) disabled = false;
  @property({ type: Boolean }) error = false;
  @property({ type: String }) errorMessage = "";
  @property({ type: String }) type: "text" | "email" | "password" | "url" | "tel" = "text";
  @property({ type: Boolean }) clearable = false;
  @property({ type: Number }) maxLength: number | undefined = undefined;

  @query("input") private inputEl!: HTMLInputElement;

  createRenderRoot() { return this; }

  private handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    this.value = target.value;
    this.dispatchEvent(new CustomEvent("value-change", { detail: this.value }));
  }

  private handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (this.value && this.clearable) {
        e.preventDefault();
        this.clearValue();
      } else {
        this.inputEl?.blur();
      }
    } else if (e.key === "Enter") {
      this.dispatchEvent(new CustomEvent("submit", { detail: this.value }));
    }
  }

  private clearValue() {
    this.value = "";
    this.inputEl?.focus();
    this.dispatchEvent(new CustomEvent("value-change", { detail: "" }));
    this.dispatchEvent(new CustomEvent("clear"));
  }

  public focus() { this.inputEl?.focus(); }
  public blur() { this.inputEl?.blur(); }
  public select() { this.inputEl?.select(); }

  private getCharCountColor(): string {
    if (!this.maxLength) return "";
    if (this.value.length >= this.maxLength) return "var(--adi-error)";
    if (this.value.length >= this.maxLength * 0.9) return "var(--adi-warning)";
    return "";
  }

  render() {
    const showClear = this.clearable && this.value && !this.disabled;
    const borderColor = this.error ? "var(--adi-error)" : "var(--adi-border)";

    return html`
      <div style="display:flex;flex-direction:column;gap:calc(var(--l) * 0.375);">
        ${this.label ? html`<label style="font-size:calc(var(--t) * 0.75);font-weight:500;color:var(--adi-text-muted);text-transform:uppercase;letter-spacing:0.05em;">${this.label}</label>` : ""}
        <div style="position:relative;display:flex;align-items:center;">
          <input
            type=${this.type}
            style="
              width: 100%;
              box-sizing: border-box;
              border: 1px solid ${borderColor};
              border-radius: var(--r);
              padding: calc(var(--l) * 0.625) calc(var(--l) * 0.875);
              font-size: calc(var(--t) * 0.875);
              font-family: inherit;
              background: color-mix(in srgb, var(--adi-text) 3%, transparent);
              color: var(--adi-text);
              outline: none;
              transition: border-color 200ms, box-shadow 200ms, background 200ms;
              ${showClear ? `padding-right: calc(var(--l) * 2.5);` : ""}
              ${this.disabled ? "opacity: 0.5; cursor: not-allowed;" : ""}
            "
            .value=${this.value}
            placeholder=${this.placeholder}
            ?disabled=${this.disabled}
            maxlength=${this.maxLength || ""}
            @input=${this.handleInput}
            @keydown=${this.handleKeyDown}
            aria-invalid=${this.error}
            aria-describedby=${this.error && this.errorMessage ? "error-msg" : ""}
          />
          ${showClear
            ? html`
                <button
                  style="
                    position: absolute;
                    right: calc(var(--l) * 0.5);
                    background: transparent;
                    border: none;
                    color: var(--adi-text-muted);
                    cursor: pointer;
                    padding: calc(var(--l) * 0.25);
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    border-radius: calc(var(--r) * 0.5);
                    transition: color 150ms, background 150ms;
                  "
                  @click=${this.clearValue}
                  title="Clear (Esc)"
                  tabindex="-1"
                >
                  <svg width="14" height="14" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                  </svg>
                </button>
              `
            : ""}
        </div>
        <div style="display:flex;justify-content:space-between;align-items:center;">
          ${this.error && this.errorMessage
            ? html`
                <span id="error-msg" style="font-size:calc(var(--t) * 0.75);color:var(--adi-error);display:flex;align-items:center;gap:calc(var(--l) * 0.25);">
                  <svg width="14" height="14" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="flex-shrink:0;">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                  </svg>
                  ${this.errorMessage}
                </span>
              `
            : html`<span></span>`}
          ${this.maxLength
            ? html`<span style="font-size:calc(var(--t) * 0.75);color:${this.getCharCountColor() || "var(--adi-text-muted)"};text-align:right;">${this.value.length}/${this.maxLength}</span>`
            : ""}
        </div>
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "adi-text-input": AdiTextInput;
  }
}
