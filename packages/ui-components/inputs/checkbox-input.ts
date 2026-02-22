import { LitElement, html } from "lit";
import { customElement, property } from "lit/decorators.js";

/// Checkbox with indeterminate support. Sizing via ADID AX system (--l, --t, --r).
@customElement("adi-checkbox-input")
export class AdiCheckboxInput extends LitElement {
  @property({ type: Boolean }) checked = false;
  @property({ type: String }) label = "";
  @property({ type: Boolean }) disabled = false;
  @property({ type: Boolean }) indeterminate = false;

  createRenderRoot() { return this; }

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
    const isActive = this.indeterminate || this.checked;
    // Box: 1.125 * --l square
    const boxSize = "calc(var(--l) * 1.125)";

    return html`
      <label
        style="
          display: inline-flex;
          align-items: center;
          gap: calc(var(--l) * 0.5);
          cursor: pointer;
          position: relative;
          ${this.disabled ? "opacity: 0.5; cursor: not-allowed;" : ""}
        "
        @click=${(e: Event) => { e.preventDefault(); this.toggle(); }}
      >
        <input
          type="checkbox"
          style="position:absolute;opacity:0;width:0;height:0;pointer-events:none;"
          .checked=${this.checked}
          .indeterminate=${this.indeterminate}
          ?disabled=${this.disabled}
          @keydown=${this.handleKeyDown}
          aria-checked=${this.indeterminate ? "mixed" : this.checked}
        />
        <div style="
          position: relative;
          width: ${boxSize};
          height: ${boxSize};
          border: 1px solid ${isActive ? "var(--adi-accent)" : "color-mix(in srgb, var(--adi-text) 20%, transparent)"};
          border-radius: max(2px, calc(var(--r) * 0.25));
          background: ${isActive ? "var(--adi-accent)" : "color-mix(in srgb, var(--adi-text) 3%, transparent)"};
          display: flex;
          align-items: center;
          justify-content: center;
          flex-shrink: 0;
          transition: background-color 150ms, border-color 150ms;
        ">
          <span style="
            color: white;
            display: flex;
            align-items: center;
            justify-content: center;
            width: 100%;
            height: 100%;
            transition: opacity 150ms, transform 150ms;
            opacity: ${isActive ? "1" : "0"};
            transform: ${isActive ? "scale(1)" : "scale(0)"};
          ">
            ${this.indeterminate
              ? html`<svg width="70%" height="70%" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 12h14"/>
                </svg>`
              : html`<svg width="70%" height="70%" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"/>
                </svg>`
            }
          </span>
        </div>
        ${this.label ? html`<span style="color:var(--adi-text-muted);user-select:none;font-size:calc(var(--t) * 0.875);transition:color 150ms;">${this.label}</span>` : ""}
      </label>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "adi-checkbox-input": AdiCheckboxInput;
  }
}
