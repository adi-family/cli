import { LitElement, html, css } from "lit";
import { customElement, property, state, query } from "lit/decorators.js";

export interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

@customElement("select-input")
export class SelectInput extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: String }) value = "";
  @property({ type: String }) placeholder = "Select option...";
  @property({ type: String }) label = "";
  @property({ type: Boolean }) disabled = false;
  @property({ type: Array }) options: SelectOption[] = [];
  
  @state() private isOpen = false;
  @state() private highlightedIndex = -1;
  @state() private typeaheadBuffer = "";
  
  @query(".select-trigger") private triggerEl!: HTMLButtonElement;
  @query(".dropdown") private dropdownEl!: HTMLDivElement;

  private typeaheadTimeout: number | null = null;

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

    .select-wrapper {
      position: relative;
    }

    .select-trigger {
      width: 100%;
      box-sizing: border-box;
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 0.5rem;
      font-family: inherit;
      background: rgba(255, 255, 255, 0.03);
      color: white;
      transition: all 0.2s ease;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: space-between;
      text-align: left;
    }

    .select-trigger:hover:not(:disabled) {
      border-color: rgba(255, 255, 255, 0.2);
    }

    .select-trigger:focus {
      outline: none;
      border-color: #8b5cf6;
      box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.15);
    }

    .select-trigger:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    .select-trigger.open {
      border-color: #8b5cf6;
      box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.15);
    }

    .select-trigger.sm {
      padding: 0.375rem 0.75rem;
      font-size: 0.75rem;
      min-height: 28px;
    }

    .select-trigger.md {
      padding: 0.5rem 1rem;
      font-size: 0.875rem;
      min-height: 36px;
    }

    .select-trigger.lg {
      padding: 0.75rem 1.25rem;
      font-size: 1rem;
      min-height: 44px;
    }

    .placeholder {
      color: #6b7280;
    }

    .chevron {
      width: 1em;
      height: 1em;
      transition: transform 0.2s;
      flex-shrink: 0;
      color: #6b7280;
    }

    .chevron.open {
      transform: rotate(180deg);
    }

    .dropdown {
      position: absolute;
      top: calc(100% + 4px);
      left: 0;
      right: 0;
      background: #1a1525;
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 0.5rem;
      max-height: 200px;
      overflow-y: auto;
      z-index: 100;
      box-shadow: 0 10px 25px rgba(0, 0, 0, 0.5);
    }

    .option {
      padding: 0.5rem 1rem;
      cursor: pointer;
      transition: background 0.1s;
      font-size: 0.875rem;
      display: flex;
      align-items: center;
      justify-content: space-between;
    }

    .option:hover:not(.disabled) {
      background: rgba(139, 92, 246, 0.15);
    }

    .option.highlighted {
      background: rgba(139, 92, 246, 0.2);
    }

    .option.selected {
      color: #c4b5fd;
    }

    .option.selected .check {
      display: block;
    }

    .option.disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    .check {
      display: none;
      width: 1em;
      height: 1em;
      color: #8b5cf6;
    }

    .no-options {
      padding: 0.75rem 1rem;
      color: #6b7280;
      font-size: 0.875rem;
      text-align: center;
    }
  `;

  connectedCallback() {
    super.connectedCallback();
    document.addEventListener("click", this.handleOutsideClick);
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    document.removeEventListener("click", this.handleOutsideClick);
    if (this.typeaheadTimeout) {
      clearTimeout(this.typeaheadTimeout);
    }
  }

  private handleOutsideClick = (e: Event) => {
    const path = e.composedPath();
    if (!path.includes(this)) {
      this.closeDropdown();
    }
  };

  private openDropdown() {
    if (!this.disabled && !this.isOpen) {
      this.isOpen = true;
      // Set highlighted to current selection or first enabled option
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
    if (this.isOpen) {
      this.closeDropdown();
    } else {
      this.openDropdown();
    }
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
        if (!this.isOpen) {
          this.openDropdown();
        } else {
          this.highlightedIndex = this.getNextEnabledIndex(this.highlightedIndex);
          this.scrollToHighlighted();
        }
        break;

      case "ArrowUp":
        e.preventDefault();
        if (!this.isOpen) {
          this.openDropdown();
        } else {
          this.highlightedIndex = this.getPrevEnabledIndex(this.highlightedIndex);
          this.scrollToHighlighted();
        }
        break;

      case "Home":
        e.preventDefault();
        if (this.isOpen) {
          this.highlightedIndex = this.getFirstEnabledIndex();
          this.scrollToHighlighted();
        }
        break;

      case "End":
        e.preventDefault();
        if (this.isOpen) {
          this.highlightedIndex = this.getLastEnabledIndex();
          this.scrollToHighlighted();
        }
        break;

      case "Tab":
        if (this.isOpen) {
          this.closeDropdown();
        }
        break;

      default:
        // Type-ahead search
        if (e.key.length === 1 && !e.ctrlKey && !e.metaKey) {
          this.handleTypeahead(e.key);
        }
        break;
    }
  };

  private handleTypeahead(char: string) {
    this.typeaheadBuffer += char.toLowerCase();
    
    if (this.typeaheadTimeout) {
      clearTimeout(this.typeaheadTimeout);
    }
    
    this.typeaheadTimeout = window.setTimeout(() => {
      this.typeaheadBuffer = "";
    }, 500);

    // Find matching option
    const matchIndex = this.options.findIndex(
      (o, i) => !o.disabled && o.label.toLowerCase().startsWith(this.typeaheadBuffer)
    );

    if (matchIndex >= 0) {
      if (this.isOpen) {
        this.highlightedIndex = matchIndex;
        this.scrollToHighlighted();
      } else {
        this.selectOptionByIndex(matchIndex);
      }
    }
  }

  private scrollToHighlighted() {
    this.updateComplete.then(() => {
      const dropdown = this.dropdownEl;
      const highlighted = dropdown?.querySelector(".option.highlighted") as HTMLElement;
      if (dropdown && highlighted) {
        const dropdownRect = dropdown.getBoundingClientRect();
        const highlightedRect = highlighted.getBoundingClientRect();
        
        if (highlightedRect.bottom > dropdownRect.bottom) {
          dropdown.scrollTop += highlightedRect.bottom - dropdownRect.bottom;
        } else if (highlightedRect.top < dropdownRect.top) {
          dropdown.scrollTop -= dropdownRect.top - highlightedRect.top;
        }
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
    if (!this.options[index].disabled) {
      this.highlightedIndex = index;
    }
  }

  private getSelectedLabel() {
    const selected = this.options.find((o) => o.value === this.value);
    return selected?.label;
  }

  render() {
    const selectedLabel = this.getSelectedLabel();

    return html`
      ${this.label ? html`<label id="select-label">${this.label}</label>` : ""}
      <div class="select-wrapper">
        <button
          class="select-trigger ${this.size} ${this.isOpen ? "open" : ""}"
          ?disabled=${this.disabled}
          @click=${this.toggleDropdown}
          @keydown=${this.handleKeyDown}
          role="combobox"
          aria-expanded=${this.isOpen}
          aria-haspopup="listbox"
          aria-labelledby=${this.label ? "select-label" : ""}
        >
          <span class="${!selectedLabel ? "placeholder" : ""}">
            ${selectedLabel || this.placeholder}
          </span>
          <svg class="chevron ${this.isOpen ? "open" : ""}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
          </svg>
        </button>
        ${this.isOpen
          ? html`
              <div class="dropdown" role="listbox">
                ${this.options.length === 0
                  ? html`<div class="no-options">No options available</div>`
                  : this.options.map(
                      (option, index) => html`
                        <div
                          class="option ${option.disabled ? "disabled" : ""} ${option.value === this.value ? "selected" : ""} ${index === this.highlightedIndex ? "highlighted" : ""}"
                          role="option"
                          aria-selected=${option.value === this.value}
                          aria-disabled=${option.disabled || false}
                          @click=${(e: Event) => this.selectOption(e, option)}
                          @mouseenter=${() => this.handleOptionMouseEnter(index)}
                        >
                          ${option.label}
                          <svg class="check" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
                          </svg>
                        </div>
                      `
                    )}
              </div>
            `
          : ""}
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "select-input": SelectInput;
  }
}
