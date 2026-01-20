import { LitElement, html, css } from "lit";
import { customElement, property, query } from "lit/decorators.js";

@customElement("text-input")
export class TextInput extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
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

  static styles = css`
    :host {
      display: flex;
      flex-direction: column;
      gap: 0.375rem;
    }

    label {
      font-size: 0.75rem;
      font-weight: 500;
      color: #9ca3af;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }

    .input-wrapper {
      position: relative;
      display: flex;
      align-items: center;
    }

    input {
      width: 100%;
      box-sizing: border-box;
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 0.5rem;
      font-family: inherit;
      background: rgba(255, 255, 255, 0.03);
      color: white;
      transition: all 0.2s ease;
    }

    input::placeholder {
      color: #6b7280;
    }

    input:focus {
      outline: none;
      border-color: #8b5cf6;
      box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.15);
      background: rgba(139, 92, 246, 0.05);
    }

    input:disabled {
      opacity: 0.5;
      cursor: not-allowed;
      background: rgba(255, 255, 255, 0.02);
    }

    input.error {
      border-color: #ef4444;
    }

    input.error:focus {
      box-shadow: 0 0 0 3px rgba(239, 68, 68, 0.15);
      background: rgba(239, 68, 68, 0.05);
    }

    input.sm {
      padding: 0.375rem 0.75rem;
      font-size: 0.75rem;
      min-height: 28px;
    }

    input.md {
      padding: 0.5rem 1rem;
      font-size: 0.875rem;
      min-height: 36px;
    }

    input.lg {
      padding: 0.75rem 1.25rem;
      font-size: 1rem;
      min-height: 44px;
    }

    input.has-clear {
      padding-right: 2.5rem;
    }

    .clear-btn {
      position: absolute;
      right: 0.5rem;
      background: none;
      border: none;
      color: #6b7280;
      cursor: pointer;
      padding: 0.25rem;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: 0.25rem;
      transition: all 0.15s;
    }

    .clear-btn:hover {
      color: white;
      background: rgba(255, 255, 255, 0.1);
    }

    .clear-btn:focus {
      outline: none;
      box-shadow: 0 0 0 2px rgba(139, 92, 246, 0.3);
    }

    .clear-btn svg {
      width: 0.875em;
      height: 0.875em;
    }

    .error-message {
      font-size: 0.75rem;
      color: #f87171;
      display: flex;
      align-items: center;
      gap: 0.25rem;
    }

    .error-message svg {
      width: 0.875rem;
      height: 0.875rem;
      flex-shrink: 0;
    }

    .char-count {
      font-size: 0.75rem;
      color: #6b7280;
      text-align: right;
    }

    .char-count.warning {
      color: #fbbf24;
    }

    .char-count.error {
      color: #f87171;
    }

    .footer {
      display: flex;
      justify-content: space-between;
      align-items: center;
    }
  `;

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
        // Blur on escape if nothing to clear
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

  public focus() {
    this.inputEl?.focus();
  }

  public blur() {
    this.inputEl?.blur();
  }

  public select() {
    this.inputEl?.select();
  }

  render() {
    const showClear = this.clearable && this.value && !this.disabled;
    const charCountClass = this.maxLength 
      ? this.value.length >= this.maxLength 
        ? "error" 
        : this.value.length >= this.maxLength * 0.9 
          ? "warning" 
          : ""
      : "";

    return html`
      ${this.label ? html`<label>${this.label}</label>` : ""}
      <div class="input-wrapper">
        <input
          type=${this.type}
          class="${this.size} ${this.error ? "error" : ""} ${showClear ? "has-clear" : ""}"
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
                class="clear-btn" 
                @click=${this.clearValue} 
                title="Clear (Esc)"
                tabindex="-1"
              >
                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                </svg>
              </button>
            `
          : ""}
      </div>
      <div class="footer">
        ${this.error && this.errorMessage
          ? html`
              <span class="error-message" id="error-msg">
                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                </svg>
                ${this.errorMessage}
              </span>
            `
          : html`<span></span>`}
        ${this.maxLength
          ? html`<span class="char-count ${charCountClass}">${this.value.length}/${this.maxLength}</span>`
          : ""}
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "text-input": TextInput;
  }
}
