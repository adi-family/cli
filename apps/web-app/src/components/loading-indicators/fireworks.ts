import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

interface Rocket {
  x: number;
  y: number;
  vy: number;
  targetY: number;
  color: string;
  trail: { x: number; y: number; alpha: number }[];
}

interface Spark {
  x: number;
  y: number;
  vx: number;
  vy: number;
  life: number;
  color: string;
  size: number;
}

@customElement("fireworks-effect")
export class FireworksEffect extends LitElement {
  @property({ type: String }) size: "sm" | "md" | "lg" = "md";
  @property({ type: String }) label = "";

  private canvas?: HTMLCanvasElement;
  private ctx?: CanvasRenderingContext2D;
  private animationId?: number;
  private rockets: Rocket[] = [];
  private sparks: Spark[] = [];
  private lastLaunch = 0;
  private colors = ["#f472b6", "#a78bfa", "#67e8f9", "#4ade80", "#fbbf24", "#fb7185", "#c084fc"];

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

  private launchRocket() {
    const { width, height } = this.getCanvasSize();
    const x = width * 0.2 + Math.random() * width * 0.6;
    this.rockets.push({
      x,
      y: height,
      vy: -4 - Math.random() * 2,
      targetY: height * 0.2 + Math.random() * height * 0.3,
      color: this.colors[Math.floor(Math.random() * this.colors.length)],
      trail: [],
    });
  }

  private explode(rocket: Rocket) {
    const sparkCount = this.size === "sm" ? 20 : this.size === "md" ? 35 : 50;
    
    for (let i = 0; i < sparkCount; i++) {
      const angle = (Math.PI * 2 * i) / sparkCount + Math.random() * 0.3;
      const speed = 1 + Math.random() * 3;
      this.sparks.push({
        x: rocket.x,
        y: rocket.y,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed,
        life: 1,
        color: rocket.color,
        size: 2 + Math.random() * 2,
      });
    }

    // Add some white sparkles in center
    for (let i = 0; i < 8; i++) {
      const angle = Math.random() * Math.PI * 2;
      const speed = 0.5 + Math.random() * 1.5;
      this.sparks.push({
        x: rocket.x,
        y: rocket.y,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed,
        life: 1,
        color: "#ffffff",
        size: 1.5,
      });
    }
  }

  private runAnimation = () => {
    if (!this.ctx || !this.canvas) return;

    const { width, height } = this.getCanvasSize();
    this.ctx.clearRect(0, 0, width, height);

    const now = Date.now();
    if (now - this.lastLaunch > 1200) {
      this.launchRocket();
      this.lastLaunch = now;
    }

    // Update rockets
    this.rockets = this.rockets.filter((r) => {
      r.y += r.vy;
      r.trail.push({ x: r.x, y: r.y, alpha: 1 });
      if (r.trail.length > 10) r.trail.shift();

      // Draw trail
      for (let i = 0; i < r.trail.length; i++) {
        const t = r.trail[i];
        t.alpha *= 0.85;
        this.ctx!.beginPath();
        this.ctx!.arc(t.x, t.y, 2, 0, Math.PI * 2);
        this.ctx!.fillStyle = `rgba(255, 255, 255, ${t.alpha * 0.5})`;
        this.ctx!.fill();
      }

      // Draw rocket head
      this.ctx!.beginPath();
      this.ctx!.arc(r.x, r.y, 3, 0, Math.PI * 2);
      this.ctx!.fillStyle = r.color;
      this.ctx!.fill();

      if (r.y <= r.targetY) {
        this.explode(r);
        return false;
      }
      return true;
    });

    // Update sparks
    this.sparks = this.sparks.filter((s) => {
      s.x += s.vx;
      s.y += s.vy;
      s.vy += 0.05; // gravity
      s.vx *= 0.98;
      s.life -= 0.015;

      if (s.life <= 0) return false;

      // Draw spark
      this.ctx!.beginPath();
      this.ctx!.arc(s.x, s.y, s.size * s.life, 0, Math.PI * 2);
      this.ctx!.fillStyle = s.color;
      this.ctx!.globalAlpha = s.life;
      this.ctx!.fill();

      // Glow
      this.ctx!.beginPath();
      this.ctx!.arc(s.x, s.y, s.size * s.life * 2, 0, Math.PI * 2);
      this.ctx!.fillStyle = s.color;
      this.ctx!.globalAlpha = s.life * 0.3;
      this.ctx!.fill();

      this.ctx!.globalAlpha = 1;
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
      this.launchRocket();
      this.lastLaunch = Date.now();
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
    "fireworks-effect": FireworksEffect;
  }
}
