import { LitElement, html } from "lit";
import { customElement, property } from "lit/decorators.js";

/// Shimmer placeholder skeleton. Sizing via ADID AX system (--l, --t, --r).
@customElement("adi-loading-skeleton")
export class AdiLoadingSkeleton extends LitElement {
  @property({ type: String }) label = "";
  @property({ type: String }) variant: "card" | "text" | "avatar" = "card";

  createRenderRoot() { return this; }

  private renderCard() {
    // Card: 9.375 x 6.25 --l units
    return html`
      <div style="
        width: calc(var(--l) * 9.375);
        height: calc(var(--l) * 6.25);
        background: var(--adi-surface);
        border-radius: var(--r);
        overflow: hidden;
        position: relative;
      " class="skeleton-shimmer">
        <div style="padding:calc(var(--l) * 0.75);display:flex;flex-direction:column;gap:calc(var(--l) * 0.5);height:100%;box-sizing:border-box;">
          <div style="display:flex;gap:calc(var(--l) * 0.625);align-items:center;">
            <div style="width:calc(var(--l) * 1.5);height:calc(var(--l) * 1.5);border-radius:50%;background:var(--adi-surface-alt);flex-shrink:0;"></div>
            <div style="height:calc(var(--t) * 0.75);background:var(--adi-surface-alt);border-radius:calc(var(--r) * 0.5);flex:1;"></div>
          </div>
          <div style="height:calc(var(--t) * 0.5);background:var(--adi-surface-alt);border-radius:calc(var(--r) * 0.5);width:80%;"></div>
          <div style="height:calc(var(--t) * 0.5);background:var(--adi-surface-alt);border-radius:calc(var(--r) * 0.5);width:60%;"></div>
        </div>
      </div>
    `;
  }

  private renderText() {
    // Text: 9.375 --l wide, three lines
    return html`
      <div style="
        width: calc(var(--l) * 9.375);
        display: flex;
        flex-direction: column;
        gap: calc(var(--l) * 0.5);
        position: relative;
        overflow: hidden;
        border-radius: var(--r);
      " class="skeleton-shimmer">
        <div style="height:calc(var(--t) * 0.625);background:var(--adi-surface-alt);border-radius:calc(var(--r) * 0.5);width:100%;"></div>
        <div style="height:calc(var(--t) * 0.625);background:var(--adi-surface-alt);border-radius:calc(var(--r) * 0.5);width:90%;"></div>
        <div style="height:calc(var(--t) * 0.625);background:var(--adi-surface-alt);border-radius:calc(var(--r) * 0.5);width:70%;"></div>
      </div>
    `;
  }

  private renderAvatar() {
    // Avatar: 4 * --l diameter
    return html`
      <div style="
        width: calc(var(--l) * 4);
        height: calc(var(--l) * 4);
        border-radius: 50%;
        background: var(--adi-surface);
        display: flex;
        align-items: center;
        justify-content: center;
        position: relative;
        overflow: hidden;
      " class="skeleton-shimmer">
        <div style="width:70%;height:70%;border-radius:50%;background:var(--adi-surface-alt);"></div>
      </div>
    `;
  }

  render() {
    let content;
    switch (this.variant) {
      case "text": content = this.renderText(); break;
      case "avatar": content = this.renderAvatar(); break;
      default: content = this.renderCard();
    }

    return html`
      <div style="display:flex;flex-direction:column;align-items:center;gap:calc(var(--l) * 0.75);">
        ${content}
        ${this.label ? html`<span style="font-size:calc(var(--t) * 0.875);color:var(--adi-text-muted);">${this.label}</span>` : ""}
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "adi-loading-skeleton": AdiLoadingSkeleton;
  }
}
