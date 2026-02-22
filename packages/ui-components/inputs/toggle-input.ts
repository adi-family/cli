import { LitElement, html } from "lit";
import { customElement, property } from "lit/decorators.js";

/// Switch toggle. Sizing via ADID AX system (--l, --t).
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
    // Track: 2.25 * --l wide, 1.25 * --l tall. Knob: 1 * --l diameter.
    return html`
      <label
        style="
          display: inline-flex;
          align-items: center;
          gap: calc(var(--l) * 0.625);
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
          width: calc(var(--l) * 2.25);
          height: calc(var(--l) * 1.25);
          border-radius: 9999px;
          background: ${this.checked ? "var(--adi-accent)" : "color-mix(in srgb, var(--adi-text) 10%, transparent)"};
          transition: background-color 200ms;
          flex-shrink: 0;
        ">
          <span style="
            position: absolute;
            top: calc(var(--l) * 0.125);
            left: calc(var(--l) * 0.125);
            width: var(--l);
            height: var(--l);
            background: white;
            border-radius: 50%;
            transition: transform 200ms;
            box-shadow: 0 1px 3px rgba(0,0,0,0.3);
            ${this.checked ? `transform: translateX(var(--l));` : ""}
          "></span>
        </div>
        ${this.label ? html`<span style="color:var(--adi-text-muted);user-select:none;font-size:calc(var(--t) * 0.875);transition:color 150ms;">${this.label}</span>` : ""}
      </label>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "adi-toggle-input": AdiToggleInput;
  }
}
