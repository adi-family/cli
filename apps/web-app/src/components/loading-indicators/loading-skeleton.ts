import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("loading-skeleton")
export class LoadingSkeleton extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: String }) label = "";
  @property({ type: String }) variant: "card" | "text" | "avatar" = "card";

  static styles = css`
    :host {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 0.75rem;
    }

    .skeleton {
      background: #1a1525;
      border-radius: 8px;
      overflow: hidden;
      position: relative;
    }

    .skeleton::after {
      content: "";
      position: absolute;
      inset: 0;
      background: linear-gradient(
        90deg,
        transparent 0%,
        rgba(139, 92, 246, 0.1) 20%,
        rgba(139, 92, 246, 0.2) 50%,
        rgba(139, 92, 246, 0.1) 80%,
        transparent 100%
      );
      animation: shimmer 1.5s ease-in-out infinite;
    }

    /* Card variant */
    .card.sm { width: 100px; height: 70px; }
    .card.md { width: 150px; height: 100px; }
    .card.lg { width: 200px; height: 130px; }

    .card-content {
      padding: 12px;
      display: flex;
      flex-direction: column;
      gap: 8px;
      height: 100%;
      box-sizing: border-box;
    }

    .card-header {
      display: flex;
      gap: 10px;
      align-items: center;
    }

    .card-avatar {
      width: 24px;
      height: 24px;
      border-radius: 50%;
      background: #231d30;
      flex-shrink: 0;
    }

    .card-title {
      height: 12px;
      background: #231d30;
      border-radius: 4px;
      flex: 1;
    }

    .card-line {
      height: 8px;
      background: #231d30;
      border-radius: 4px;
    }

    .card-line.short { width: 60%; }
    .card-line.medium { width: 80%; }

    /* Text variant */
    .text {
      display: flex;
      flex-direction: column;
      gap: 8px;
    }

    .text.sm { width: 100px; }
    .text.md { width: 150px; }
    .text.lg { width: 200px; }

    .text-line {
      height: 10px;
      background: #231d30;
      border-radius: 4px;
    }

    .text-line:nth-child(1) { width: 100%; }
    .text-line:nth-child(2) { width: 90%; }
    .text-line:nth-child(3) { width: 70%; }

    /* Avatar variant */
    .avatar {
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .avatar.sm { width: 40px; height: 40px; }
    .avatar.md { width: 64px; height: 64px; }
    .avatar.lg { width: 88px; height: 88px; }

    .avatar-inner {
      width: 70%;
      height: 70%;
      border-radius: 50%;
      background: #231d30;
    }

    .label {
      font-size: 0.875rem;
      color: #9ca3af;
    }

    @keyframes shimmer {
      0% { transform: translateX(-100%); }
      100% { transform: translateX(100%); }
    }
  `;

  private renderCard() {
    return html`
      <div class="skeleton card ${this.size}">
        <div class="card-content">
          <div class="card-header">
            <div class="card-avatar"></div>
            <div class="card-title"></div>
          </div>
          <div class="card-line medium"></div>
          <div class="card-line short"></div>
        </div>
      </div>
    `;
  }

  private renderText() {
    return html`
      <div class="skeleton text ${this.size}">
        <div class="text-line"></div>
        <div class="text-line"></div>
        <div class="text-line"></div>
      </div>
    `;
  }

  private renderAvatar() {
    return html`
      <div class="skeleton avatar ${this.size}">
        <div class="avatar-inner"></div>
      </div>
    `;
  }

  render() {
    let content;
    switch (this.variant) {
      case "text":
        content = this.renderText();
        break;
      case "avatar":
        content = this.renderAvatar();
        break;
      default:
        content = this.renderCard();
    }

    return html`
      ${content}
      ${this.label ? html`<span class="label">${this.label}</span>` : ""}
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "loading-skeleton": LoadingSkeleton;
  }
}
