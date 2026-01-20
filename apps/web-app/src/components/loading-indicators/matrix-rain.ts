import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("matrix-rain")
export class MatrixRain extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: String }) label = "";

  private canvas?: HTMLCanvasElement;
  private ctx?: CanvasRenderingContext2D;
  private animationId?: number;
  private columns: number[] = [];
  private chars = "アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲン0123456789";

  static styles = css`
    :host {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 0.75rem;
    }

    canvas {
      border-radius: 8px;
      background: #0d0a14;
    }

    canvas.sm { width: 60px; height: 80px; }
    canvas.md { width: 100px; height: 120px; }
    canvas.lg { width: 140px; height: 160px; }

    .label {
      font-size: 0.875rem;
      color: #9ca3af;
    }
  `;

  private getCanvasSize() {
    const sizes = {
      sm: { width: 120, height: 160 },
      md: { width: 200, height: 240 },
      lg: { width: 280, height: 320 },
    };
    return sizes[this.size];
  }

  private runAnimation = () => {
    if (!this.ctx || !this.canvas) return;

    const { width, height } = this.getCanvasSize();
    const fontSize = this.size === "sm" ? 10 : this.size === "md" ? 12 : 14;

    // Fade effect
    this.ctx.fillStyle = "rgba(13, 10, 20, 0.05)";
    this.ctx.fillRect(0, 0, width, height);

    // Set font
    this.ctx.font = `${fontSize}px monospace`;

    for (let i = 0; i < this.columns.length; i++) {
      // Random character
      const char = this.chars[Math.floor(Math.random() * this.chars.length)];
      const x = i * fontSize;
      const y = this.columns[i] * fontSize;

      // Gradient from bright to dim
      const brightness = Math.random();
      if (brightness > 0.98) {
        this.ctx.fillStyle = "#fff";
      } else if (brightness > 0.9) {
        this.ctx.fillStyle = "#c4b5fd";
      } else {
        this.ctx.fillStyle = "#8b5cf6";
      }

      this.ctx.fillText(char, x, y);

      // Reset or increment
      if (y > height && Math.random() > 0.98) {
        this.columns[i] = 0;
      }
      this.columns[i]++;
    }

    this.animationId = requestAnimationFrame(this.runAnimation);
  };

  firstUpdated() {
    this.canvas = this.shadowRoot?.querySelector("canvas") as HTMLCanvasElement;
    if (this.canvas) {
      const { width, height } = this.getCanvasSize();
      this.canvas.width = width;
      this.canvas.height = height;
      this.ctx = this.canvas.getContext("2d")!;

      // Initialize columns
      const fontSize = this.size === "sm" ? 10 : this.size === "md" ? 12 : 14;
      const columnCount = Math.floor(width / fontSize);
      this.columns = Array.from({ length: columnCount }, () => 
        Math.floor(Math.random() * (height / fontSize))
      );

      this.runAnimation();
    }
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    if (this.animationId) cancelAnimationFrame(this.animationId);
  }

  render() {
    return html`
      <canvas class="${this.size}"></canvas>
      ${this.label ? html`<span class="label">${this.label}</span>` : ""}
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "matrix-rain": MatrixRain;
  }
}
