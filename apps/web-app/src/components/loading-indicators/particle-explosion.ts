import { LitElement, html, css } from "lit";
import { customElement, property, state } from "lit/decorators.js";

interface Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  life: number;
  maxLife: number;
  size: number;
  hue: number;
}

@customElement("particle-explosion")
export class ParticleExplosion extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: String }) label = "";

  @state() private particles: Particle[] = [];
  private canvas?: HTMLCanvasElement;
  private ctx?: CanvasRenderingContext2D;
  private animationId?: number;
  private lastExplosion = 0;

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

  private explode() {
    const canvasSize = this.getCanvasSize();
    const cx = canvasSize / 2;
    const cy = canvasSize / 2;
    const count = this.size === "sm" ? 15 : this.size === "md" ? 25 : 35;

    for (let i = 0; i < count; i++) {
      const angle = (Math.PI * 2 * i) / count + Math.random() * 0.5;
      const speed = 2 + Math.random() * 3;
      this.particles.push({
        x: cx,
        y: cy,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed,
        life: 1,
        maxLife: 60 + Math.random() * 40,
        size: 2 + Math.random() * 3,
        hue: 250 + Math.random() * 30, // violet range
      });
    }
  }

  private runAnimation = () => {
    if (!this.ctx || !this.canvas) return;

    const canvasSize = this.getCanvasSize();
    this.ctx.fillStyle = "rgba(13, 10, 20, 0.15)";
    this.ctx.fillRect(0, 0, canvasSize, canvasSize);

    // Trigger explosion periodically
    const now = Date.now();
    if (now - this.lastExplosion > 1500) {
      this.explode();
      this.lastExplosion = now;
    }

    // Update and draw particles
    this.particles = this.particles.filter((p) => {
      p.x += p.vx;
      p.y += p.vy;
      p.vx *= 0.98;
      p.vy *= 0.98;
      p.life -= 1 / p.maxLife;

      if (p.life <= 0) return false;

      const alpha = p.life;
      this.ctx!.beginPath();
      this.ctx!.arc(p.x, p.y, p.size * p.life, 0, Math.PI * 2);
      this.ctx!.fillStyle = `hsla(${p.hue}, 70%, 60%, ${alpha})`;
      this.ctx!.fill();

      // Glow effect
      this.ctx!.beginPath();
      this.ctx!.arc(p.x, p.y, p.size * p.life * 2, 0, Math.PI * 2);
      this.ctx!.fillStyle = `hsla(${p.hue}, 70%, 60%, ${alpha * 0.3})`;
      this.ctx!.fill();

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
      this.ctx.fillStyle = "#0d0a14";
      this.ctx.fillRect(0, 0, size, size);
      this.explode();
      this.lastExplosion = Date.now();
      this.runAnimation();
    }
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    if (this.animationId) {
      cancelAnimationFrame(this.animationId);
    }
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
    "particle-explosion": ParticleExplosion;
  }
}
