import { LitElement, html } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("adi-toggle-input")
export class AdiToggleInput extends LitElement {
  @property({ type: Boolean }) checked = false;
  @property({ type: String }) label = "";
  @property({ type: Boolean }) disabled = false;

  createRenderRoot() { return this; }

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
        style="
          display: inline-flex;
          align-items: center;
          gap: calc(1rem * 0.625);
          cursor: pointer;
          position: relative;
          ${this.disabled ? "opacity: 0.5; cursor: not-allowed;" : ""}
        "
        @click=${(e: Event) => { e.preventDefault(); this.toggle(); }}
      >
        <input
          type="checkbox"
          role="switch"
          style="position:absolute;opacity:0;width:0;height:0;pointer-events:none;"
          .checked=${this.checked}
          ?disabled=${this.disabled}
          @keydown=${this.handleKeyDown}
          aria-checked=${this.checked}
        />
        <div style="
          position: relative;
          width: calc(1rem * 2.25);
          height: calc(1rem * 1.25);
          border-radius: 9999px;
          background: ${this.checked ? "var(--adi-accent)" : "color-mix(in srgb, var(--adi-text) 10%, transparent)"};
          transition: background-color 200ms;
          flex-shrink: 0;
        ">
          <span style="
            position: absolute;
            top: calc(1rem * 0.125);
            left: calc(1rem * 0.125);
            width: 1rem;
            height: 1rem;
            background: white;
            border-radius: 50%;
            transition: transform 200ms;
            box-shadow: 0 1px 3px rgba(0,0,0,0.3);
            ${this.checked ? `transform: translateX(1rem);` : ""}
          "></span>
        </div>
        ${this.label ? html`<span style="color:var(--adi-text-muted);user-select:none;font-size:calc(1rem * 0.875);transition:color 150ms;">${this.label}</span>` : ""}
      </label>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "adi-toggle-input": AdiToggleInput;
  }
}
