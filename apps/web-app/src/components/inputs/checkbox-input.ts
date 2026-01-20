import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("checkbox-input")
export class CheckboxInput extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: Boolean }) checked = false;
  @property({ type: String }) label = "";
  @property({ type: Boolean }) disabled = false;
  @property({ type: Boolean }) indeterminate = false;

  static styles = css`
    :host {
      display: inline-flex;
    }

    .checkbox-wrapper {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      cursor: pointer;
      position: relative;
    }

    .checkbox-wrapper.disabled {
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

    .checkbox {
      position: relative;
      border: 1px solid rgba(255, 255, 255, 0.2);
      border-radius: 0.25rem;
      background: rgba(255, 255, 255, 0.03);
      transition: all 0.15s ease;
      display: flex;
      align-items: center;
      justify-content: center;
      flex-shrink: 0;
    }

    .checkbox.sm { width: 14px; height: 14px; }
    .checkbox.md { width: 18px; height: 18px; }
    .checkbox.lg { width: 22px; height: 22px; }

    /* Focus styles */
    input:focus-visible + .checkbox {
      border-color: #8b5cf6;
      box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.25);
    }

    .checkbox-wrapper:hover:not(.disabled) .checkbox {
      border-color: #8b5cf6;
    }

    .checkbox.checked,
    .checkbox.indeterminate {
      background: #8b5cf6;
      border-color: #8b5cf6;
    }

    .checkbox.checked:hover:not(.disabled),
    .checkbox.indeterminate:hover:not(.disabled) {
      background: #7c3aed;
      border-color: #7c3aed;
    }

    .mark {
      color: white;
      opacity: 0;
      transform: scale(0);
      transition: all 0.15s ease;
      display: flex;
      align-items: center;
      justify-content: center;
      width: 100%;
      height: 100%;
    }

    .checkbox.checked .mark,
    .checkbox.indeterminate .mark {
      opacity: 1;
      transform: scale(1);
    }

    .mark svg {
      width: 70%;
      height: 70%;
    }

    .label {
      color: #d1d5db;
      user-select: none;
      transition: color 0.15s;
    }

    .checkbox-wrapper:hover:not(.disabled) .label {
      color: white;
    }

    .label.sm { font-size: 0.75rem; }
    .label.md { font-size: 0.875rem; }
    .label.lg { font-size: 1rem; }
  `;

  private toggle() {
    if (!this.disabled) {
      this.checked = !this.checked;
      this.indeterminate = false;
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
    const stateClass = this.indeterminate ? "indeterminate" : this.checked ? "checked" : "";
    
    return html`
      <label 
        class="checkbox-wrapper ${this.disabled ? "disabled" : ""}"
        @click=${(e: Event) => { e.preventDefault(); this.toggle(); }}
      >
        <input 
          type="checkbox" 
          .checked=${this.checked}
          .indeterminate=${this.indeterminate}
          ?disabled=${this.disabled}
          @keydown=${this.handleKeyDown}
          aria-checked=${this.indeterminate ? "mixed" : this.checked}
        />
        <div class="checkbox ${this.size} ${stateClass} ${this.disabled ? "disabled" : ""}">
          <span class="mark">
            ${this.indeterminate
              ? html`<svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 12h14"/>
                </svg>`
              : html`<svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"/>
                </svg>`
            }
          </span>
        </div>
        ${this.label ? html`<span class="label ${this.size}">${this.label}</span>` : ""}
      </label>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "checkbox-input": CheckboxInput;
  }
}
