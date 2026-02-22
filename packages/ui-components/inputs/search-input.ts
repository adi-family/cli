import { LitElement, html } from "lit";
import { customElement, property, query } from "lit/decorators.js";

/// Search input with debounce. Sizing via ADID AX system (--l, --t, --r).
@customElement("adi-search-input")
export class AdiSearchInput extends LitElement {
  @property({ type: String }) value = "";
  @property({ type: String }) placeholder = "Search...";
  @property({ type: Boolean }) disabled = false;
  @property({ type: Boolean }) loading = false;
  @property({ type: Number }) debounceMs = 0;

  @query("input") private inputEl!: HTMLInputElement;

  private debounceTimeout: number | null = null;

  createRenderRoot() { return this; }

  disconnectedCallback() {
    super.disconnectedCallback();
    if (this.debounceTimeout) clearTimeout(this.debounceTimeout);
  }

  private handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    this.value = target.value;

    if (this.debounceMs > 0) {
      if (this.debounceTimeout) clearTimeout(this.debounceTimeout);
      this.debounceTimeout = window.setTimeout(() => {
        this.dispatchEvent(new CustomEvent("search", { detail: this.value }));
      }, this.debounceMs);
    }

    this.dispatchEvent(new CustomEvent("value-change", { detail: this.value }));
  }

  private handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      if (this.value) { this.handleClear(); }
      else { this.inputEl?.blur(); }
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (this.debounceTimeout) clearTimeout(this.debounceTimeout);
      this.dispatchEvent(new CustomEvent("search", { detail: this.value }));
      this.dispatchEvent(new CustomEvent("submit", { detail: this.value }));
    }
  }

  private handleClear() {
    this.value = "";
    this.inputEl?.focus();
    if (this.debounceTimeout) clearTimeout(this.debounceTimeout);
    this.dispatchEvent(new CustomEvent("value-change", { detail: "" }));
    this.dispatchEvent(new CustomEvent("search", { detail: "" }));
    this.dispatchEvent(new CustomEvent("clear"));
  }

  public focus() { this.inputEl?.focus(); }
  public blur() { this.inputEl?.blur(); }
  public select() { this.inputEl?.select(); }

  render() {
    const showClear = this.value && !this.loading && !this.disabled;

    return html`
      <div style="display:flex;width:100%;">
        <div style="position:relative;display:flex;align-items:center;width:100%;">
          <span style="
            position: absolute;
            left: calc(var(--l) * 0.75);
            color: var(--adi-text-muted);
            pointer-events: none;
            display: flex;
            align-items: center;
            font-size: calc(var(--t) * 0.875);
            transition: color 150ms;
          ">
            <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
            </svg>
          </span>
          <input
            type="search"
            style="
              width: 100%;
              box-sizing: border-box;
              border: 1px solid var(--adi-border);
              border-radius: var(--r);
              padding: calc(var(--l) * 0.625) calc(var(--l) * 2.5) calc(var(--l) * 0.625) calc(var(--l) * 2.5);
              font-size: calc(var(--t) * 0.875);
              font-family: inherit;
              background: color-mix(in srgb, var(--adi-text) 3%, transparent);
              color: var(--adi-text);
              outline: none;
              transition: border-color 200ms, box-shadow 200ms, background 200ms;
              ${this.disabled ? "opacity: 0.5; cursor: not-allowed;" : ""}
            "
            .value=${this.value}
            placeholder=${this.placeholder}
            ?disabled=${this.disabled}
            @input=${this.handleInput}
            @keydown=${this.handleKeyDown}
            autocomplete="off"
            spellcheck="false"
          />
          <div style="position:absolute;right:calc(var(--l) * 0.5);display:flex;align-items:center;gap:calc(var(--l) * 0.25);">
            ${this.loading
              ? html`<span style="
                  width: calc(var(--t) * 0.875);
                  height: calc(var(--t) * 0.875);
                  border: 2px solid color-mix(in srgb, var(--adi-accent) 30%, transparent);
                  border-top-color: var(--adi-accent);
                  border-radius: 50%;
                  animation: adi-spin 1s linear infinite;
                "></span>`
              : showClear
                ? html`
                    <button
                      style="
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
                      @click=${this.handleClear}
                      title="Clear (Esc)"
                      tabindex="-1"
                    >
                      <svg width="14" height="14" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                      </svg>
                    </button>
                  `
                : !this.value && !this.disabled
                  ? html`<span style="font-size:calc(var(--t) * 0.625);color:var(--adi-text-muted);background:color-mix(in srgb, var(--adi-text) 5%, transparent);padding:calc(var(--l) * 0.125) calc(var(--l) * 0.375);border-radius:calc(var(--r) * 0.5);font-family:monospace;border:1px solid var(--adi-border);">/</span>`
                  : ""
            }
          </div>
        </div>
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "adi-search-input": AdiSearchInput;
  }
}
