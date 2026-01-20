import { LitElement, html, css } from "lit";
import { customElement, property, query } from "lit/decorators.js";

@customElement("textarea-input")
export class TextareaInput extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
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

    .textarea-wrapper {
      position: relative;
    }

    textarea {
      width: 100%;
      box-sizing: border-box;
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 0.5rem;
      font-family: inherit;
      background: rgba(255, 255, 255, 0.03);
      color: white;
      transition: all 0.2s ease;
      resize: vertical;
      min-height: 80px;
    }

    textarea::placeholder {
      color: #6b7280;
    }

    textarea:focus {
      outline: none;
      border-color: #8b5cf6;
      box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.15);
      background: rgba(139, 92, 246, 0.05);
    }

    textarea:disabled {
      opacity: 0.5;
      cursor: not-allowed;
      background: rgba(255, 255, 255, 0.02);
      resize: none;
    }

    textarea.error {
      border-color: #ef4444;
    }

    textarea.error:focus {
      box-shadow: 0 0 0 3px rgba(239, 68, 68, 0.15);
      background: rgba(239, 68, 68, 0.05);
    }

    textarea.auto-resize {
      resize: none;
      overflow: hidden;
    }

    textarea.sm {
      padding: 0.375rem 0.75rem;
      font-size: 0.75rem;
    }

    textarea.md {
      padding: 0.5rem 1rem;
      font-size: 0.875rem;
    }

    textarea.lg {
      padding: 0.75rem 1.25rem;
      font-size: 1rem;
    }

    .footer {
      display: flex;
      justify-content: space-between;
      align-items: center;
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

    .hint {
      font-size: 0.625rem;
      color: #4b5563;
    }
  `;

  private handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    this.value = target.value;
    
    if (this.autoResize) {
      this.adjustHeight();
    }
    
    this.dispatchEvent(new CustomEvent("value-change", { detail: this.value }));
  }

  private handleKeyDown(e: KeyboardEvent) {
    // Cmd/Ctrl + Enter to submit
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      this.dispatchEvent(new CustomEvent("submit", { detail: this.value }));
    }
    // Escape to blur
    else if (e.key === "Escape") {
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

  public focus() {
    this.textareaEl?.focus();
  }

  public blur() {
    this.textareaEl?.blur();
  }

  public select() {
    this.textareaEl?.select();
  }

  render() {
    const charCountClass = this.maxLength 
      ? this.value.length >= this.maxLength 
        ? "error" 
        : this.value.length >= this.maxLength * 0.9 
          ? "warning" 
          : ""
      : "";

    return html`
      ${this.label ? html`<label>${this.label}</label>` : ""}
      <div class="textarea-wrapper">
        <textarea
          class="${this.size} ${this.error ? "error" : ""} ${this.autoResize ? "auto-resize" : ""}"
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
      <div class="footer">
        <div>
          ${this.error && this.errorMessage
            ? html`
                <span class="error-message" id="error-msg">
                  <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                  </svg>
                  ${this.errorMessage}
                </span>
              `
            : html`<span class="hint">Cmd+Enter to submit</span>`}
        </div>
        ${this.maxLength
          ? html`<span class="char-count ${charCountClass}">${this.value.length}/${this.maxLength}</span>`
          : ""}
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "textarea-input": TextareaInput;
  }
}
