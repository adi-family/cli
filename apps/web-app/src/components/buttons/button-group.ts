import { LitElement, html, css } from "lit";
import { customElement, property, queryAll } from "lit/decorators.js";

export interface ButtonGroupOption {
  value: string;
  label: string;
  disabled?: boolean;
}

@customElement("button-group")
export class ButtonGroup extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: String }) value = "";
  @property({ type: Array }) options: ButtonGroupOption[] = [];
  @property({ type: Boolean }) disabled = false;
  @property({ type: String }) variant: "default" | "primary" = "default";

  @queryAll("button") private buttons!: NodeListOf<HTMLButtonElement>;

  static styles = css`
    :host {
      display: inline-flex;
    }

    .button-group {
      display: inline-flex;
      border-radius: 0.5rem;
      overflow: hidden;
      border: 1px solid rgba(255, 255, 255, 0.1);
    }

    .button-group:focus-within {
      box-shadow: 0 0 0 2px rgba(139, 92, 246, 0.3);
    }

    .button-group.disabled {
      opacity: 0.5;
      pointer-events: none;
    }

    button {
      border: none;
      background: rgba(255, 255, 255, 0.03);
      color: #9ca3af;
      font-family: inherit;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.15s ease;
      position: relative;
    }

    button:focus {
      outline: none;
      z-index: 1;
    }

    button:focus-visible {
      box-shadow: inset 0 0 0 2px #8b5cf6;
    }

    button:not(:last-child) {
      border-right: 1px solid rgba(255, 255, 255, 0.1);
    }

    button:hover:not(:disabled):not(.active) {
      background: rgba(255, 255, 255, 0.08);
      color: white;
    }

    button:disabled {
      cursor: not-allowed;
      opacity: 0.5;
    }

    /* Default variant active state */
    .default button.active {
      background: rgba(255, 255, 255, 0.12);
      color: white;
    }

    /* Primary variant active state */
    .primary button.active {
      background: #8b5cf6;
      color: white;
    }

    .primary button.active:hover {
      background: #7c3aed;
    }

    /* Sizes */
    button.sm {
      padding: 0.375rem 0.75rem;
      font-size: 0.75rem;
      min-height: 28px;
    }

    button.md {
      padding: 0.5rem 1rem;
      font-size: 0.875rem;
      min-height: 36px;
    }

    button.lg {
      padding: 0.75rem 1.5rem;
      font-size: 1rem;
      min-height: 44px;
    }
  `;

  private getEnabledOptions(): { option: ButtonGroupOption; index: number }[] {
    return this.options
      .map((option, index) => ({ option, index }))
      .filter(({ option }) => !option.disabled);
  }

  private getCurrentIndex(): number {
    return this.options.findIndex(o => o.value === this.value);
  }

  private focusButton(index: number) {
    this.updateComplete.then(() => {
      const button = this.buttons[index];
      button?.focus();
    });
  }

  private selectAndFocus(option: ButtonGroupOption, index: number) {
    this.value = option.value;
    this.focusButton(index);
    this.dispatchEvent(new CustomEvent("value-change", { detail: option.value }));
  }

  private handleKeyDown(e: KeyboardEvent, currentIndex: number) {
    if (this.disabled) return;

    const enabledOptions = this.getEnabledOptions();
    const currentEnabledIdx = enabledOptions.findIndex(({ index }) => index === currentIndex);

    switch (e.key) {
      case "ArrowRight":
      case "ArrowDown":
        e.preventDefault();
        if (currentEnabledIdx < enabledOptions.length - 1) {
          const next = enabledOptions[currentEnabledIdx + 1];
          this.selectAndFocus(next.option, next.index);
        } else {
          // Wrap to first
          const first = enabledOptions[0];
          if (first) this.selectAndFocus(first.option, first.index);
        }
        break;

      case "ArrowLeft":
      case "ArrowUp":
        e.preventDefault();
        if (currentEnabledIdx > 0) {
          const prev = enabledOptions[currentEnabledIdx - 1];
          this.selectAndFocus(prev.option, prev.index);
        } else {
          // Wrap to last
          const last = enabledOptions[enabledOptions.length - 1];
          if (last) this.selectAndFocus(last.option, last.index);
        }
        break;

      case "Home":
        e.preventDefault();
        const first = enabledOptions[0];
        if (first) this.selectAndFocus(first.option, first.index);
        break;

      case "End":
        e.preventDefault();
        const last = enabledOptions[enabledOptions.length - 1];
        if (last) this.selectAndFocus(last.option, last.index);
        break;

      case " ":
      case "Enter":
        e.preventDefault();
        // Already selected via focus, but ensure event fires
        const option = this.options[currentIndex];
        if (option && !option.disabled) {
          this.value = option.value;
          this.dispatchEvent(new CustomEvent("value-change", { detail: option.value }));
        }
        break;
    }
  }

  private selectOption(option: ButtonGroupOption) {
    if (!option.disabled && !this.disabled) {
      this.value = option.value;
      this.dispatchEvent(new CustomEvent("value-change", { detail: option.value }));
    }
  }

  render() {
    const currentIndex = this.getCurrentIndex();
    
    return html`
      <div 
        class="button-group ${this.variant} ${this.disabled ? "disabled" : ""}"
        role="radiogroup"
      >
        ${this.options.map(
          (option, index) => html`
            <button
              class="${this.size} ${option.value === this.value ? "active" : ""}"
              ?disabled=${option.disabled || this.disabled}
              @click=${() => this.selectOption(option)}
              @keydown=${(e: KeyboardEvent) => this.handleKeyDown(e, index)}
              role="radio"
              aria-checked=${option.value === this.value}
              tabindex=${option.value === this.value || (this.value === "" && index === 0) ? 0 : -1}
            >
              ${option.label}
            </button>
          `
        )}
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "button-group": ButtonGroup;
  }
}
