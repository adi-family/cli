import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

interface ConfettiPiece {
  x: number;
  y: number;
  vx: number;
  vy: number;
  rotation: number;
  rotationSpeed: number;
  width: number;
  height: number;
  color: string;
  life: number;
}

@customElement("confetti-burst")
export class ConfettiBurst extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: String }) label = "";

  private canvas?: HTMLCanvasElement;
  private ctx?: CanvasRenderingContext2D;
  private animationId?: number;
  private confetti: ConfettiPiece[] = [];
  private lastBurst = 0;
  private colors = ["#f472b6", "#a78bfa", "#67e8f9", "#4ade80", "#fbbf24", "#fb7185"];

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

  private burst() {
    const { width, height } = this.getCanvasSize();
    const cx = width / 2;
    const cy = height * 0.4;
    const count = this.size === "sm" ? 20 : this.size === "md" ? 35 : 50;

    for (let i = 0; i < count; i++) {
      const angle = Math.random() * Math.PI * 2;
      const speed = 2 + Math.random() * 4;
      this.confetti.push({
        x: cx,
        y: cy,
        vx: Math.cos(angle) * speed * (0.5 + Math.random()),
        vy: Math.sin(angle) * speed - 3,
        rotation: Math.random() * Math.PI * 2,
        rotationSpeed: (Math.random() - 0.5) * 0.3,
        width: 4 + Math.random() * 6,
        height: 8 + Math.random() * 8,
        color: this.colors[Math.floor(Math.random() * this.colors.length)],
        life: 1,
      });
    }
  }

  private runAnimation = () => {
    if (!this.ctx || !this.canvas) return;

    const { width, height } = this.getCanvasSize();
    this.ctx.clearRect(0, 0, width, height);

    const now = Date.now();
    if (now - this.lastBurst > 2000) {
      this.burst();
      this.lastBurst = now;
    }

    this.confetti = this.confetti.filter((c) => {
      c.x += c.vx;
      c.y += c.vy;
      c.vy += 0.15; // gravity
      c.vx *= 0.99;
      c.rotation += c.rotationSpeed;
      c.life -= 0.008;

      if (c.life <= 0 || c.y > height + 20) return false;

      this.ctx!.save();
      this.ctx!.translate(c.x, c.y);
      this.ctx!.rotate(c.rotation);
      this.ctx!.globalAlpha = c.life;
      this.ctx!.fillStyle = c.color;
      this.ctx!.fillRect(-c.width / 2, -c.height / 2, c.width, c.height);
      this.ctx!.restore();

      return true;
    });

    this.animationId = requestAnimationFrame(this.runAnimation);
  };

  firstUpdated() {
    this.canvas = this.shadowRoot?.querySelector("canvas") as HTMLCanvasElement;
    if (this.canvas) {
      const { width, height } = this.getCanvasSize();
      this.canvas.width = width;
      this.canvas.height = height;
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
    "confetti-burst": ConfettiBurst;
  }
}
