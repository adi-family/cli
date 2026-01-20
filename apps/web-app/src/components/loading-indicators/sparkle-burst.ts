import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

interface Sparkle {
  x: number;
  y: number;
  size: number;
  maxSize: number;
  rotation: number;
  rotationSpeed: number;
  life: number;
  delay: number;
  color: string;
}

@customElement("sparkle-burst")
export class SparkleBurst extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: String }) label = "";

  private canvas?: HTMLCanvasElement;
  private ctx?: CanvasRenderingContext2D;
  private animationId?: number;
  private sparkles: Sparkle[] = [];
  private lastBurst = 0;
  private colors = ["#fbbf24", "#fcd34d", "#fef08a", "#ffffff", "#a78bfa"];

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

    canvas.sm { width: 60px; height: 60px; }
    canvas.md { width: 100px; height: 100px; }
    canvas.lg { width: 140px; height: 140px; }

    .label {
      font-size: 0.875rem;
      color: #9ca3af;
    }
  `;

  private getCanvasSize() {
    return this.size === "sm" ? 120 : this.size === "md" ? 200 : 280;
  }

  private drawStar(x: number, y: number, size: number, rotation: number, color: string, alpha: number) {
    if (!this.ctx) return;
    
    const spikes = 4;
    const outerRadius = size;
    const innerRadius = size * 0.4;

    this.ctx.save();
    this.ctx.translate(x, y);
    this.ctx.rotate(rotation);
    this.ctx.globalAlpha = alpha;

    this.ctx.beginPath();
    for (let i = 0; i < spikes * 2; i++) {
      const radius = i % 2 === 0 ? outerRadius : innerRadius;
      const angle = (i * Math.PI) / spikes;
      if (i === 0) {
        this.ctx.moveTo(Math.cos(angle) * radius, Math.sin(angle) * radius);
      } else {
        this.ctx.lineTo(Math.cos(angle) * radius, Math.sin(angle) * radius);
      }
    }
    this.ctx.closePath();
    this.ctx.fillStyle = color;
    this.ctx.fill();

    // Glow
    this.ctx.shadowColor = color;
    this.ctx.shadowBlur = size * 0.8;
    this.ctx.fill();

    this.ctx.restore();
  }

  private burst() {
    const canvasSize = this.getCanvasSize();
    const count = this.size === "sm" ? 8 : this.size === "md" ? 12 : 16;

    for (let i = 0; i < count; i++) {
      const angle = (Math.PI * 2 * i) / count;
      const distance = canvasSize * 0.2 + Math.random() * canvasSize * 0.2;
      this.sparkles.push({
        x: canvasSize / 2 + Math.cos(angle) * distance,
        y: canvasSize / 2 + Math.sin(angle) * distance,
        size: 0,
        maxSize: 8 + Math.random() * 12,
        rotation: Math.random() * Math.PI,
        rotationSpeed: (Math.random() - 0.5) * 0.1,
        life: 1,
        delay: i * 0.03,
        color: this.colors[Math.floor(Math.random() * this.colors.length)],
      });
    }

    // Add center sparkle
    this.sparkles.push({
      x: canvasSize / 2,
      y: canvasSize / 2,
      size: 0,
      maxSize: 15 + Math.random() * 10,
      rotation: 0,
      rotationSpeed: 0.05,
      life: 1,
      delay: 0,
      color: "#ffffff",
    });
  }

  private runAnimation = () => {
    if (!this.ctx || !this.canvas) return;

    const canvasSize = this.getCanvasSize();
    this.ctx.clearRect(0, 0, canvasSize, canvasSize);

    const now = Date.now();
    if (now - this.lastBurst > 1800) {
      this.burst();
      this.lastBurst = now;
    }

    this.sparkles = this.sparkles.filter((s) => {
      if (s.delay > 0) {
        s.delay -= 0.016;
        return true;
      }

      s.rotation += s.rotationSpeed;
      s.life -= 0.02;

      // Scale up then down
      const progress = 1 - s.life;
      if (progress < 0.3) {
        s.size = s.maxSize * (progress / 0.3);
      } else {
        s.size = s.maxSize * s.life;
      }

      if (s.life <= 0) return false;

      this.drawStar(s.x, s.y, s.size, s.rotation, s.color, s.life);
      return true;
    });

    this.animationId = requestAnimationFrame(this.runAnimation);
  };

  firstUpdated() {
    this.canvas = this.shadowRoot?.querySelector("canvas") as HTMLCanvasElement;
    if (this.canvas) {
      const size = this.getCanvasSize();
      this.canvas.width = size;
      this.canvas.height = size;
      this.ctx = this.canvas.getContext("2d")!;
      this.burst();
      this.lastBurst = Date.now();
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
    "sparkle-burst": SparkleBurst;
  }
}
