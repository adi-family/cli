import { LitElement, html } from "lit";
import { customElement, property, queryAll } from "lit/decorators.js";

export interface ButtonGroupOption {
  value: string;
  label: string;
  disabled?: boolean;
}

/// Segmented radio-group button. Sizing via ADID AX system.
@customElement("adi-button-group")
export class AdiButtonGroup extends LitElement {
  @property({ type: String }) value = "";
  @property({ type: Array }) options: ButtonGroupOption[] = [];
  @property({ type: Boolean }) disabled = false;
  @property({ type: String }) variant: "default" | "primary" = "default";

  @queryAll("button") private buttons!: NodeListOf<HTMLButtonElement>;

  createRenderRoot() {
    return this;
  }

  private getActiveStyles(): string {
    if (this.variant === "primary") {
      return "background: var(--adi-accent); color: white;";
    }
    return "background: color-mix(in srgb, var(--adi-text) 12%, transparent); color: var(--adi-text);";
  }

  private getEnabledOptions(): { option: ButtonGroupOption; index: number }[] {
    return this.options
      .map((option, index) => ({ option, index }))
      .filter(({ option }) => !option.disabled);
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
    const currentEnabledIdx = enabledOptions.findIndex(
      ({ index }) => index === currentIndex
    );

    switch (e.key) {
      case "ArrowRight":
      case "ArrowDown": {
        e.preventDefault();
        if (currentEnabledIdx < enabledOptions.length - 1) {
          const next = enabledOptions[currentEnabledIdx + 1];
          this.selectAndFocus(next.option, next.index);
        } else {
          const first = enabledOptions[0];
          if (first) this.selectAndFocus(first.option, first.index);
        }
        break;
      }

      case "ArrowLeft":
      case "ArrowUp": {
        e.preventDefault();
        if (currentEnabledIdx > 0) {
          const prev = enabledOptions[currentEnabledIdx - 1];
          this.selectAndFocus(prev.option, prev.index);
        } else {
          const last = enabledOptions[enabledOptions.length - 1];
          if (last) this.selectAndFocus(last.option, last.index);
        }
        break;
      }

      case "Home": {
        e.preventDefault();
        const first = enabledOptions[0];
        if (first) this.selectAndFocus(first.option, first.index);
        break;
      }

      case "End": {
        e.preventDefault();
        const last = enabledOptions[enabledOptions.length - 1];
        if (last) this.selectAndFocus(last.option, last.index);
        break;
      }

      case " ":
      case "Enter": {
        e.preventDefault();
        const option = this.options[currentIndex];
        if (option && !option.disabled) {
          this.value = option.value;
          this.dispatchEvent(new CustomEvent("value-change", { detail: option.value }));
        }
        break;
      }
    }
  }

  private selectOption(option: ButtonGroupOption) {
    if (!option.disabled && !this.disabled) {
      this.value = option.value;
      this.dispatchEvent(new CustomEvent("value-change", { detail: option.value }));
    }
  }

  render() {
    return html`
      <div
        role="radiogroup"
        style="
          display: inline-flex;
          border-radius: var(--r);
          overflow: hidden;
          border: 1px solid var(--adi-border);
          ${this.disabled ? "opacity: 0.5; pointer-events: none;" : ""}
        "
      >
        ${this.options.map(
          (option, index) => html`
            <button
              style="
                border: none;
                background: color-mix(in srgb, var(--adi-text) 3%, transparent);
                color: var(--adi-text-muted);
                font-weight: 500;
                font-size: calc(var(--t) * 0.875);
                padding: calc(var(--l) * 0.75) calc(var(--l) * 1.75);
                cursor: pointer;
                transition: background-color 150ms, color 150ms;
                position: relative;
                ${index < this.options.length - 1 ? `border-right: 1px solid var(--adi-border);` : ""}
                ${option.value === this.value ? this.getActiveStyles() : ""}
                ${option.disabled ? "opacity: 0.5; cursor: not-allowed;" : ""}
              "
              ?disabled=${option.disabled || this.disabled}
              @click=${() => this.selectOption(option)}
              @keydown=${(e: KeyboardEvent) => this.handleKeyDown(e, index)}
              role="radio"
              aria-checked=${option.value === this.value}
              tabindex=${option.value === this.value ||
              (this.value === "" && index === 0)
                ? 0
                : -1}
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
    "adi-button-group": AdiButtonGroup;
  }
}
