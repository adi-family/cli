import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("toggle-input")
export class ToggleInput extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: Boolean }) checked = false;
  @property({ type: String }) label = "";
  @property({ type: Boolean }) disabled = false;

  static styles = css`
    :host {
      display: inline-flex;
    }

    .toggle-wrapper {
      display: flex;
      align-items: center;
      gap: 0.625rem;
      cursor: pointer;
      position: relative;
    }

    .toggle-wrapper.disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    /* Hidden native checkbox for accessibility */
    input {
      position: absolute;
      opacity: 0;
      width: 0;
      height: 0;
      pointer-events: none;
    }

    .toggle {
      position: relative;
      border-radius: 999px;
      background: rgba(255, 255, 255, 0.1);
      transition: all 0.2s ease;
      flex-shrink: 0;
    }

    .toggle.sm { width: 28px; height: 16px; }
    .toggle.md { width: 36px; height: 20px; }
    .toggle.lg { width: 44px; height: 24px; }

    /* Focus styles */
    input:focus-visible + .toggle {
      box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.25);
    }

    .toggle-wrapper:hover:not(.disabled) .toggle:not(.checked) {
      background: rgba(255, 255, 255, 0.15);
    }

    .toggle.checked {
      background: #8b5cf6;
    }

    .toggle.checked:hover:not(.disabled) {
      background: #7c3aed;
    }

    .knob {
      position: absolute;
      top: 2px;
      left: 2px;
      background: white;
      border-radius: 50%;
      transition: transform 0.2s ease;
      box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
    }

    .toggle.sm .knob { width: 12px; height: 12px; }
    .toggle.md .knob { width: 16px; height: 16px; }
    .toggle.lg .knob { width: 20px; height: 20px; }

    .toggle.checked.sm .knob { transform: translateX(12px); }
    .toggle.checked.md .knob { transform: translateX(16px); }
    .toggle.checked.lg .knob { transform: translateX(20px); }

    .label {
      color: #d1d5db;
      user-select: none;
      transition: color 0.15s;
    }

    .toggle-wrapper:hover:not(.disabled) .label {
      color: white;
    }

    .label.sm { font-size: 0.75rem; }
    .label.md { font-size: 0.875rem; }
    .label.lg { font-size: 1rem; }
  `;

  private toggle() {
    if (!this.disabled) {
      this.checked = !this.checked;
      this.dispatchEvent(new CustomEvent("change", { detail: this.checked }));
    }
  }

  private handleKeyDown(e: KeyboardEvent) {
    if (e.key === " " || e.key === "Enter") {
      e.preventDefault();
      this.toggle();
    }
  }

  render() {
    return html`
      <label 
        class="toggle-wrapper ${this.disabled ? "disabled" : ""}"
        @click=${(e: Event) => { e.preventDefault(); this.toggle(); }}
      >
        <input 
          type="checkbox" 
          role="switch"
          .checked=${this.checked}
          ?disabled=${this.disabled}
          @keydown=${this.handleKeyDown}
          aria-checked=${this.checked}
        />
        <div class="toggle ${this.size} ${this.checked ? "checked" : ""} ${this.disabled ? "disabled" : ""}">
          <span class="knob"></span>
        </div>
        ${this.label ? html`<span class="label ${this.size}">${this.label}</span>` : ""}
      </label>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "toggle-input": ToggleInput;
  }
}
