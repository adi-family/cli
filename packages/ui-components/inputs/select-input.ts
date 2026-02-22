import { LitElement, html } from "lit";
import { customElement, property, state, query } from "lit/decorators.js";

export interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

/// Custom select dropdown. Sizing via ADID AX system (--l, --t, --r).
@customElement("adi-select-input")
export class AdiSelectInput extends LitElement {
  @property({ type: String }) value = "";
  @property({ type: String }) placeholder = "Select option...";
  @property({ type: String }) label = "";
  @property({ type: Boolean }) disabled = false;
  @property({ type: Array }) options: SelectOption[] = [];

  @state() private isOpen = false;
  @state() private highlightedIndex = -1;
  @state() private typeaheadBuffer = "";

  @query(".adi-select-trigger") private triggerEl!: HTMLButtonElement;
  @query(".adi-select-dropdown") private dropdownEl!: HTMLDivElement;

  private typeaheadTimeout: number | null = null;

  createRenderRoot() { return this; }

  connectedCallback() {
    super.connectedCallback();
    document.addEventListener("click", this.handleOutsideClick);
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    document.removeEventListener("click", this.handleOutsideClick);
    if (this.typeaheadTimeout) clearTimeout(this.typeaheadTimeout);
  }

  private handleOutsideClick = (e: Event) => {
    if (!e.composedPath().includes(this)) this.closeDropdown();
  };

  private openDropdown() {
    if (!this.disabled && !this.isOpen) {
      this.isOpen = true;
      const currentIndex = this.options.findIndex(o => o.value === this.value);
      this.highlightedIndex = currentIndex >= 0 ? currentIndex : this.getFirstEnabledIndex();
      this.scrollToHighlighted();
    }
  }

  private closeDropdown() {
    this.isOpen = false;
    this.highlightedIndex = -1;
    this.typeaheadBuffer = "";
  }

  private toggleDropdown(e: Event) {
    e.stopPropagation();
    this.isOpen ? this.closeDropdown() : this.openDropdown();
  }

  private getFirstEnabledIndex(): number {
    return this.options.findIndex(o => !o.disabled);
  }

  private getLastEnabledIndex(): number {
    for (let i = this.options.length - 1; i >= 0; i--) {
      if (!this.options[i].disabled) return i;
    }
    return -1;
  }

  private getNextEnabledIndex(fromIndex: number): number {
    for (let i = fromIndex + 1; i < this.options.length; i++) {
      if (!this.options[i].disabled) return i;
    }
    return fromIndex;
  }

  private getPrevEnabledIndex(fromIndex: number): number {
    for (let i = fromIndex - 1; i >= 0; i--) {
      if (!this.options[i].disabled) return i;
    }
    return fromIndex;
  }

  private handleKeyDown = (e: KeyboardEvent) => {
    if (this.disabled) return;

    switch (e.key) {
      case "Enter":
      case " ":
        e.preventDefault();
        if (this.isOpen && this.highlightedIndex >= 0) {
          this.selectOptionByIndex(this.highlightedIndex);
        } else {
          this.openDropdown();
        }
        break;
      case "Escape":
        e.preventDefault();
        this.closeDropdown();
        this.triggerEl?.focus();
        break;
      case "ArrowDown":
        e.preventDefault();
        if (!this.isOpen) { this.openDropdown(); }
        else { this.highlightedIndex = this.getNextEnabledIndex(this.highlightedIndex); this.scrollToHighlighted(); }
        break;
      case "ArrowUp":
        e.preventDefault();
        if (!this.isOpen) { this.openDropdown(); }
        else { this.highlightedIndex = this.getPrevEnabledIndex(this.highlightedIndex); this.scrollToHighlighted(); }
        break;
      case "Home":
        e.preventDefault();
        if (this.isOpen) { this.highlightedIndex = this.getFirstEnabledIndex(); this.scrollToHighlighted(); }
        break;
      case "End":
        e.preventDefault();
        if (this.isOpen) { this.highlightedIndex = this.getLastEnabledIndex(); this.scrollToHighlighted(); }
        break;
      case "Tab":
        if (this.isOpen) this.closeDropdown();
        break;
      default:
        if (e.key.length === 1 && !e.ctrlKey && !e.metaKey) this.handleTypeahead(e.key);
        break;
    }
  };

  private handleTypeahead(char: string) {
    this.typeaheadBuffer += char.toLowerCase();
    if (this.typeaheadTimeout) clearTimeout(this.typeaheadTimeout);
    this.typeaheadTimeout = window.setTimeout(() => { this.typeaheadBuffer = ""; }, 500);

    const matchIndex = this.options.findIndex(
      (o) => !o.disabled && o.label.toLowerCase().startsWith(this.typeaheadBuffer)
    );

    if (matchIndex >= 0) {
      if (this.isOpen) { this.highlightedIndex = matchIndex; this.scrollToHighlighted(); }
      else { this.selectOptionByIndex(matchIndex); }
    }
  }

  private scrollToHighlighted() {
    this.updateComplete.then(() => {
      const dropdown = this.dropdownEl;
      const highlighted = dropdown?.querySelector("[data-highlighted]") as HTMLElement;
      if (dropdown && highlighted) {
        const dRect = dropdown.getBoundingClientRect();
        const hRect = highlighted.getBoundingClientRect();
        if (hRect.bottom > dRect.bottom) dropdown.scrollTop += hRect.bottom - dRect.bottom;
        else if (hRect.top < dRect.top) dropdown.scrollTop -= dRect.top - hRect.top;
      }
    });
  }

  private selectOptionByIndex(index: number) {
    const option = this.options[index];
    if (option && !option.disabled) {
      this.value = option.value;
      this.closeDropdown();
      this.dispatchEvent(new CustomEvent("value-change", { detail: option.value }));
    }
  }

  private selectOption(e: Event, option: SelectOption) {
    e.stopPropagation();
    if (!option.disabled) {
      this.value = option.value;
      this.closeDropdown();
      this.triggerEl?.focus();
      this.dispatchEvent(new CustomEvent("value-change", { detail: option.value }));
    }
  }

  private handleOptionMouseEnter(index: number) {
    if (!this.options[index].disabled) this.highlightedIndex = index;
  }

  private getSelectedLabel() {
    return this.options.find((o) => o.value === this.value)?.label;
  }

  render() {
    const selectedLabel = this.getSelectedLabel();

    return html`
      <div style="display:flex;flex-direction:column;gap:calc(var(--l) * 0.375);">
        ${this.label ? html`<label id="select-label" style="font-size:calc(var(--t) * 0.75);font-weight:500;color:var(--adi-text-muted);text-transform:uppercase;letter-spacing:0.05em;">${this.label}</label>` : ""}
        <div style="position:relative;">
          <button
            class="adi-select-trigger"
            style="
              width: 100%;
              box-sizing: border-box;
              border: 1px solid ${this.isOpen ? "var(--adi-accent)" : "var(--adi-border)"};
              border-radius: var(--r);
              padding: calc(var(--l) * 0.625) calc(var(--l) * 0.875);
              font-size: calc(var(--t) * 0.875);
              font-family: inherit;
              background: color-mix(in srgb, var(--adi-text) 3%, transparent);
              color: var(--adi-text);
              cursor: pointer;
              display: flex;
              align-items: center;
              justify-content: space-between;
              text-align: left;
              outline: none;
              transition: border-color 200ms, box-shadow 200ms;
              ${this.disabled ? "opacity: 0.5; cursor: not-allowed;" : ""}
            "
            ?disabled=${this.disabled}
            @click=${this.toggleDropdown}
            @keydown=${this.handleKeyDown}
            role="combobox"
            aria-expanded=${this.isOpen}
            aria-haspopup="listbox"
            aria-labelledby=${this.label ? "select-label" : ""}
          >
            <span style="${!selectedLabel ? `color: var(--adi-text-muted);` : ""}">
              ${selectedLabel || this.placeholder}
            </span>
            <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="flex-shrink:0;color:var(--adi-text-muted);transition:transform 200ms;${this.isOpen ? "transform:rotate(180deg);" : ""}">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
            </svg>
          </button>
          ${this.isOpen
            ? html`
                <div
                  class="adi-select-dropdown"
                  role="listbox"
                  style="
                    position: absolute;
                    top: calc(100% + calc(var(--l) * 0.25));
                    left: 0;
                    right: 0;
                    background: var(--adi-surface);
                    border: 1px solid var(--adi-border);
                    border-radius: var(--r);
                    max-height: calc(var(--l) * 12.5);
                    overflow-y: auto;
                    z-index: 100;
                    box-shadow: 0 10px 25px rgba(0,0,0,0.5);
                  "
                >
                  ${this.options.length === 0
                    ? html`<div style="padding:calc(var(--l) * 0.75) var(--l);color:var(--adi-text-muted);font-size:calc(var(--t) * 0.875);text-align:center;">No options available</div>`
                    : this.options.map(
                        (option, index) => html`
                          <div
                            style="
                              padding: calc(var(--l) * 0.5) var(--l);
                              cursor: pointer;
                              transition: background-color 100ms;
                              font-size: calc(var(--t) * 0.875);
                              display: flex;
                              align-items: center;
                              justify-content: space-between;
                              ${option.disabled ? "opacity: 0.5; cursor: not-allowed;" : ""}
                              ${option.value === this.value ? `color: var(--adi-accent);` : ""}
                              ${index === this.highlightedIndex ? `background: color-mix(in srgb, var(--adi-accent) 15%, transparent);` : ""}
                            "
                            role="option"
                            aria-selected=${option.value === this.value}
                            aria-disabled=${option.disabled || false}
                            ?data-highlighted=${index === this.highlightedIndex}
                            @click=${(e: Event) => this.selectOption(e, option)}
                            @mouseenter=${() => this.handleOptionMouseEnter(index)}
                          >
                            ${option.label}
                            <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color:var(--adi-accent);${option.value === this.value ? "display:block;" : "display:none;"}">
                              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
                            </svg>
                          </div>
                        `
                      )}
                </div>
              `
            : ""}
        </div>
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "adi-select-input": AdiSelectInput;
  }
}
