import { LitElement, html } from "lit";
import { customElement, property } from "lit/decorators.js";

/// Placeholder for pages or sections not yet built.
@customElement("adi-under-construction")
export class AdiUnderConstruction extends LitElement {
  @property({ type: String }) heading = "Under Construction";
  @property({ type: String }) description = "This section is being built. Check back soon.";
  @property({ type: String }) badge = "In Progress";

  createRenderRoot() { return this; }

  render() {
    return html`
      <div style="
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        text-align: center;
        padding: calc(var(--l) * 4) calc(var(--l) * 2);
        min-height: calc(var(--l) * 16);
        gap: calc(var(--l) * 1.5);
      ">
        <div style="
          display: flex;
          align-items: center;
          justify-content: center;
          width: calc(var(--l) * 5);
          height: calc(var(--l) * 5);
          border-radius: 50%;
          border: 1px solid var(--adi-border);
          background: var(--adi-surface);
        ">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="var(--adi-accent)"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
            style="width:calc(var(--l) * 2.5);height:calc(var(--l) * 2.5);"
          >
            <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" />
          </svg>
        </div>

        <div style="display:flex;flex-direction:column;gap:calc(var(--l) * 0.5);max-width:calc(var(--l) * 25);">
          <h2 style="
            font-size: calc(var(--t) * 1.953);
            font-weight: 600;
            color: var(--adi-text);
            margin: 0;
            line-height: 1.2;
          ">${this.heading}</h2>

          <p style="
            font-size: calc(var(--t) * 0.875);
            color: var(--adi-text-muted);
            margin: 0;
            line-height: 1.6;
          ">${this.description}</p>
        </div>

        <div style="
          display: inline-flex;
          align-items: center;
          gap: calc(var(--l) * 0.5);
          padding: calc(var(--l) * 0.375) var(--l);
          border-radius: calc(var(--r) * 2);
          border: 1px solid var(--adi-accent);
          background: color-mix(in srgb, var(--adi-accent) 6%, transparent);
        ">
          <span style="
            width: calc(var(--l) * 0.375);
            height: calc(var(--l) * 0.375);
            border-radius: 50%;
            background: var(--adi-accent);
          "></span>
          <span style="
            font-size: calc(var(--t) * 0.75);
            font-family: monospace;
            text-transform: uppercase;
            letter-spacing: 0.1em;
            color: var(--adi-accent);
          ">${this.badge}</span>
        </div>
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "adi-under-construction": AdiUnderConstruction;
  }
}
