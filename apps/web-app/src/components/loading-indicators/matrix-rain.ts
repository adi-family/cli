import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("matrix-rain")
export class MatrixRain extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: String }) label = "";

  private canvas?: HTMLCanvasElement;
  private ctx?: CanvasRenderingContext2D;
  private animationId?: number;
  private columns: { y: number; chars: { char: string; opacity: number }[] }[] = [];
  private chars = "アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲン0123456789";
  private trailLength = 25;
  private frameCount = 0;
  private frameSkip = 3; // Update every 3rd frame (slower)

  static styles = css`
    :host {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 0.75rem;
    }

    canvas {
      border-radius: 8px;
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
    const shouldUpdate = this.frameCount % this.frameSkip === 0;

    // Clear canvas (transparent)
    this.ctx.clearRect(0, 0, width, height);

    // Set font
    this.ctx.font = `${fontSize}px monospace`;

    for (let i = 0; i < this.columns.length; i++) {
      const col = this.columns[i];
      const x = i * fontSize;

      // Draw trail characters with fading opacity
      for (let j = 0; j < col.chars.length; j++) {
        const trailChar = col.chars[j];
        const trailY = (col.y - col.chars.length + j + 1) * fontSize;
        
        if (trailY > 0 && trailY < height) {
          // Gray color with fading opacity for trail
          const alpha = trailChar.opacity * 0.6;
          this.ctx.fillStyle = `rgba(107, 114, 128, ${alpha})`;
          this.ctx.fillText(trailChar.char, x, trailY);
        }
      }

      // Draw head character (bright colored)
      const headY = col.y * fontSize;
      if (headY > 0 && headY < height) {
        const brightness = Math.random();
        if (brightness > 0.7) {
          this.ctx.fillStyle = "#fff";
        } else if (brightness > 0.3) {
          this.ctx.fillStyle = "#c4b5fd";
        } else {
          this.ctx.fillStyle = "#8b5cf6";
        }
        const headChar = this.chars[Math.floor(Math.random() * this.chars.length)];
        this.ctx.fillText(headChar, x, headY);
        
        // Only update on specific frames (slower movement)
        if (shouldUpdate) {
          // Add to trail
          col.chars.push({ char: headChar, opacity: 1 });
          if (col.chars.length > this.trailLength) {
            col.chars.shift();
          }
          // Fade trail
          for (const tc of col.chars) {
            tc.opacity *= 0.9;
          }
        }
      }

      // Move down only on update frames
      if (shouldUpdate) {
        col.y++;

        // Reset if past bottom
        if (col.y * fontSize > height + this.trailLength * fontSize && Math.random() > 0.98) {
          col.y = 0;
          col.chars = [];
        }
      }
    }

    this.frameCount++;
    this.animationId = requestAnimationFrame(this.runAnimation);
  };

  firstUpdated() {
    this.canvas = this.shadowRoot?.querySelector("canvas") as HTMLCanvasElement;
    if (this.canvas) {
      const { width, height } = this.getCanvasSize();
      this.canvas.width = width;
      this.canvas.height = height;
      this.ctx = this.canvas.getContext("2d")!;

      // Initialize columns with trail tracking
      const fontSize = this.size === "sm" ? 10 : this.size === "md" ? 12 : 14;
      const columnCount = Math.floor(width / fontSize);
      this.columns = Array.from({ length: columnCount }, () => ({
        y: Math.floor(Math.random() * (height / fontSize)),
        chars: [],
      }));

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
