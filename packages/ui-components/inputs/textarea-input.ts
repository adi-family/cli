import { LitElement, html } from "lit";
import { customElement, property, query } from "lit/decorators.js";

/// Multi-line textarea. Sizing via ADID AX system (--l, --t, --r).
@customElement("adi-textarea-input")
export class AdiTextareaInput extends LitElement {
  @property({ type: String }) value = "";
  @property({ type: String }) placeholder = "Enter text...";
  @property({ type: String }) label = "";
  @property({ type: Boolean }) disabled = false;
  @property({ type: Boolean }) error = false;
  @property({ type: String }) errorMessage = "";
  @property({ type: Number }) rows = 4;
  @property({ type: Number }) maxLength: number | undefined = undefined;
  @property({ type: Boolean }) autoResize = false;

  @query("textarea") private textareaEl!: HTMLTextAreaElement;

  createRenderRoot() { return this; }

  private handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    this.value = target.value;

    if (this.autoResize) {
      this.adjustHeight();
    }

    this.dispatchEvent(new CustomEvent("value-change", { detail: this.value }));
  }

  private handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      this.dispatchEvent(new CustomEvent("submit", { detail: this.value }));
    } else if (e.key === "Escape") {
      this.textareaEl?.blur();
    }
  }

  private adjustHeight() {
    const textarea = this.textareaEl;
    if (textarea) {
      textarea.style.height = "auto";
      textarea.style.height = `${textarea.scrollHeight}px`;
    }
  }

  updated(changedProperties: Map<string, unknown>) {
    if (changedProperties.has("value") && this.autoResize) {
      this.adjustHeight();
    }
  }

  public focus() { this.textareaEl?.focus(); }
  public blur() { this.textareaEl?.blur(); }
  public select() { this.textareaEl?.select(); }

  private getCharCountColor(): string {
    if (!this.maxLength) return "";
    if (this.value.length >= this.maxLength) return "var(--adi-error)";
    if (this.value.length >= this.maxLength * 0.9) return "var(--adi-warning)";
    return "";
  }

  render() {
    const borderColor = this.error ? "var(--adi-error)" : "var(--adi-border)";

    return html`
      <div style="display:flex;flex-direction:column;gap:calc(var(--l) * 0.375);">
        ${this.label ? html`<label style="font-size:calc(var(--t) * 0.75);font-weight:500;color:var(--adi-text-muted);text-transform:uppercase;letter-spacing:0.05em;">${this.label}</label>` : ""}
        <div style="position:relative;">
          <textarea
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
              min-height: calc(var(--l) * 5);
              ${this.autoResize ? "resize: none; overflow: hidden;" : "resize: vertical;"}
              ${this.disabled ? "opacity: 0.5; cursor: not-allowed; resize: none;" : ""}
            "
            .value=${this.value}
            placeholder=${this.placeholder}
            ?disabled=${this.disabled}
            rows=${this.rows}
            maxlength=${this.maxLength || ""}
            @input=${this.handleInput}
            @keydown=${this.handleKeyDown}
            aria-invalid=${this.error}
            aria-describedby=${this.error && this.errorMessage ? "error-msg" : ""}
          ></textarea>
        </div>
        <div style="display:flex;justify-content:space-between;align-items:center;">
          <div>
            ${this.error && this.errorMessage
              ? html`
                  <span id="error-msg" style="font-size:calc(var(--t) * 0.75);color:var(--adi-error);display:flex;align-items:center;gap:calc(var(--l) * 0.25);">
                    <svg width="14" height="14" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="flex-shrink:0;">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                    ${this.errorMessage}
                  </span>
                `
              : html`<span style="font-size:calc(var(--t) * 0.625);color:var(--adi-text-muted);">Cmd+Enter to submit</span>`}
          </div>
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
    "adi-textarea-input": AdiTextareaInput;
  }
}
