import { LitElement, html, css } from "lit";
import { customElement, property, query } from "lit/decorators.js";

@customElement("search-input")
export class SearchInput extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: String }) value = "";
  @property({ type: String }) placeholder = "Search...";
  @property({ type: Boolean }) disabled = false;
  @property({ type: Boolean }) loading = false;
  @property({ type: Number }) debounceMs = 0;

  @query("input") private inputEl!: HTMLInputElement;

  private debounceTimeout: number | null = null;

  static styles = css`
    :host {
      display: flex;
    }

    .search-wrapper {
      position: relative;
      display: flex;
      align-items: center;
      width: 100%;
    }

    .search-icon {
      position: absolute;
      left: 0.75rem;
      color: #6b7280;
      pointer-events: none;
      display: flex;
      align-items: center;
      transition: color 0.15s;
    }

    .search-wrapper:focus-within .search-icon {
      color: #8b5cf6;
    }

    .search-icon svg {
      width: 1em;
      height: 1em;
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
    }

    /* Hide native search clear button */
    input[type="search"]::-webkit-search-cancel-button,
    input[type="search"]::-webkit-search-decoration,
    input[type="search"]::-webkit-search-results-button,
    input[type="search"]::-webkit-search-results-decoration {
      -webkit-appearance: none;
      appearance: none;
      display: none;
    }

    input.sm {
      padding: 0.375rem 2rem 0.375rem 2rem;
      font-size: 0.75rem;
      min-height: 28px;
    }

    input.md {
      padding: 0.5rem 2.5rem 0.5rem 2.5rem;
      font-size: 0.875rem;
      min-height: 36px;
    }

    input.lg {
      padding: 0.75rem 3rem 0.75rem 3rem;
      font-size: 1rem;
      min-height: 44px;
    }

    .sm .search-icon { font-size: 0.75rem; left: 0.625rem; }
    .md .search-icon { font-size: 0.875rem; }
    .lg .search-icon { font-size: 1rem; left: 1rem; }

    .actions {
      position: absolute;
      right: 0.5rem;
      display: flex;
      align-items: center;
      gap: 0.25rem;
    }

    .spinner {
      width: 1em;
      height: 1em;
      border: 2px solid rgba(139, 92, 246, 0.3);
      border-top-color: #8b5cf6;
      border-radius: 50%;
      animation: spin 0.6s linear infinite;
    }

    @keyframes spin {
      to { transform: rotate(360deg); }
    }

    .clear-btn {
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

    .shortcut-hint {
      font-size: 0.625rem;
      color: #4b5563;
      background: rgba(255, 255, 255, 0.05);
      padding: 0.125rem 0.375rem;
      border-radius: 0.25rem;
      font-family: monospace;
      border: 1px solid rgba(255, 255, 255, 0.1);
    }
  `;

  disconnectedCallback() {
    super.disconnectedCallback();
    if (this.debounceTimeout) {
      clearTimeout(this.debounceTimeout);
    }
  }

  private handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    this.value = target.value;

    if (this.debounceMs > 0) {
      if (this.debounceTimeout) {
        clearTimeout(this.debounceTimeout);
      }
      this.debounceTimeout = window.setTimeout(() => {
        this.dispatchEvent(new CustomEvent("search", { detail: this.value }));
      }, this.debounceMs);
    }

    this.dispatchEvent(new CustomEvent("value-change", { detail: this.value }));
  }

  private handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      if (this.value) {
        this.handleClear();
      } else {
        this.inputEl?.blur();
      }
    } else if (e.key === "Enter") {
      e.preventDefault();
      // Cancel debounce and fire immediately
      if (this.debounceTimeout) {
        clearTimeout(this.debounceTimeout);
      }
      this.dispatchEvent(new CustomEvent("search", { detail: this.value }));
      this.dispatchEvent(new CustomEvent("submit", { detail: this.value }));
    }
  }

  private handleClear() {
    this.value = "";
    this.inputEl?.focus();
    
    if (this.debounceTimeout) {
      clearTimeout(this.debounceTimeout);
    }
    
    this.dispatchEvent(new CustomEvent("value-change", { detail: "" }));
    this.dispatchEvent(new CustomEvent("search", { detail: "" }));
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
    const showClear = this.value && !this.loading && !this.disabled;
    
    return html`
      <div class="search-wrapper ${this.size}">
        <span class="search-icon">
          <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
          </svg>
        </span>
        <input
          type="search"
          class="${this.size}"
          .value=${this.value}
          placeholder=${this.placeholder}
          ?disabled=${this.disabled}
          @input=${this.handleInput}
          @keydown=${this.handleKeyDown}
          autocomplete="off"
          spellcheck="false"
        />
        <div class="actions">
          ${this.loading 
            ? html`<span class="spinner"></span>` 
            : showClear
              ? html`
                  <button 
                    class="clear-btn" 
                    @click=${this.handleClear} 
                    title="Clear (Esc)"
                    tabindex="-1"
                  >
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                    </svg>
                  </button>
                `
              : !this.value && !this.disabled
                ? html`<span class="shortcut-hint">/</span>`
                : ""
          }
        </div>
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "search-input": SearchInput;
  }
}
