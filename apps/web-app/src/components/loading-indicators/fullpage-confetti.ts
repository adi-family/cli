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
  shape: "rect" | "circle" | "triangle";
}

/**
 * Full-page confetti overlay effect.
 * 
 * Usage:
 *   <fullpage-confetti id="confetti"></fullpage-confetti>
 * 
 * Trigger via:
 *   - Method: document.getElementById('confetti').trigger()
 *   - Event: document.dispatchEvent(new CustomEvent('trigger-confetti'))
 *   - Global: window.triggerConfetti()
 */
@customElement("fullpage-confetti")
export class FullpageConfetti extends LitElement {
  @property({ type: Number }) intensity: number = 80;
  @property({ type: Number }) duration: number = 3000;

  private canvas?: HTMLCanvasElement;
  private ctx?: CanvasRenderingContext2D;
  private animationId?: number;
  private confetti: ConfettiPiece[] = [];
  private isActive = false;
  private colors = ["#f472b6", "#a78bfa", "#67e8f9", "#4ade80", "#fbbf24", "#fb7185", "#c084fc", "#38bdf8"];

  static styles = css`
    :host {
      position: fixed;
      top: 0;
      left: 0;
      width: 100vw;
      height: 100vh;
      pointer-events: none;
      z-index: 9999;
      display: none;
    }

    :host([active]) {
      display: block;
    }

    canvas {
      width: 100%;
      height: 100%;
    }
  `;

  private burst() {
    if (!this.canvas) return;
    
    const width = this.canvas.width / window.devicePixelRatio;
    const height = this.canvas.height / window.devicePixelRatio;
    const shapes: ("rect" | "circle" | "triangle")[] = ["rect", "circle", "triangle"];

    // Burst from left and right sides of the screen
    const burstPoints = [
      // Left side - shooting right and up
      { x: -20, y: height * 0.3, dirX: 1 },
      { x: -20, y: height * 0.5, dirX: 1 },
      { x: -20, y: height * 0.7, dirX: 1 },
      // Right side - shooting left and up
      { x: width + 20, y: height * 0.3, dirX: -1 },
      { x: width + 20, y: height * 0.5, dirX: -1 },
      { x: width + 20, y: height * 0.7, dirX: -1 },
    ];

    for (const point of burstPoints) {
      for (let i = 0; i < this.intensity / 6; i++) {
        const angle = point.dirX > 0 
          ? -Math.PI * 0.3 + Math.random() * Math.PI * 0.6  // Left side: shoot right-ish
          : Math.PI * 0.7 + Math.random() * Math.PI * 0.6;  // Right side: shoot left-ish
        const speed = 10 + Math.random() * 15;
        this.confetti.push({
          x: point.x + (Math.random() - 0.5) * 40,
          y: point.y + (Math.random() - 0.5) * 100,
          vx: Math.cos(angle) * speed,
          vy: Math.sin(angle) * speed - 3,
          rotation: Math.random() * Math.PI * 2,
          rotationSpeed: (Math.random() - 0.5) * 0.4,
          width: 8 + Math.random() * 12,
          height: 12 + Math.random() * 16,
          color: this.colors[Math.floor(Math.random() * this.colors.length)],
          life: 1,
          shape: shapes[Math.floor(Math.random() * shapes.length)],
        });
      }
    }
  }

  private drawShape(c: ConfettiPiece) {
    if (!this.ctx) return;

    this.ctx.save();
    this.ctx.translate(c.x, c.y);
    this.ctx.rotate(c.rotation);
    this.ctx.globalAlpha = c.life;
    this.ctx.fillStyle = c.color;

    switch (c.shape) {
      case "rect":
        this.ctx.fillRect(-c.width / 2, -c.height / 2, c.width, c.height);
        break;
      case "circle":
        this.ctx.beginPath();
        this.ctx.arc(0, 0, c.width / 2, 0, Math.PI * 2);
        this.ctx.fill();
        break;
      case "triangle":
        this.ctx.beginPath();
        this.ctx.moveTo(0, -c.height / 2);
        this.ctx.lineTo(c.width / 2, c.height / 2);
        this.ctx.lineTo(-c.width / 2, c.height / 2);
        this.ctx.closePath();
        this.ctx.fill();
        break;
    }

    this.ctx.restore();
  }

  private runAnimation = () => {
    if (!this.ctx || !this.canvas || !this.isActive) return;

    const width = this.canvas.width / window.devicePixelRatio;
    const height = this.canvas.height / window.devicePixelRatio;
    this.ctx.clearRect(0, 0, width, height);

    this.confetti = this.confetti.filter((c) => {
      c.x += c.vx;
      c.y += c.vy;
      c.vy += 0.3;
      c.vx *= 0.99;
      c.rotation += c.rotationSpeed;
      c.life -= 0.004;

      if (c.life <= 0 || c.y > height + 50) return false;

      this.drawShape(c);
      return true;
    });

    if (this.confetti.length > 0) {
      this.animationId = requestAnimationFrame(this.runAnimation);
    } else {
      this.stop();
    }
  };

  private resizeCanvas() {
    if (this.canvas) {
      this.canvas.width = window.innerWidth * window.devicePixelRatio;
      this.canvas.height = window.innerHeight * window.devicePixelRatio;
      this.ctx?.scale(window.devicePixelRatio, window.devicePixelRatio);
    }
  }

  /** Trigger the confetti effect */
  trigger() {
    this.isActive = true;
    this.setAttribute("active", "");
    this.confetti = [];
    
    if (!this.canvas) {
      this.canvas = this.shadowRoot?.querySelector("canvas") as HTMLCanvasElement;
      this.ctx = this.canvas?.getContext("2d") ?? undefined;
    }
    
    this.resizeCanvas();
    this.burst();
    
    // Add more bursts over time
    setTimeout(() => this.burst(), 150);
    setTimeout(() => this.burst(), 300);
    
    if (this.animationId) cancelAnimationFrame(this.animationId);
    this.runAnimation();

    this.dispatchEvent(new CustomEvent("effect-started", { bubbles: true, composed: true }));
  }

  private stop() {
    this.isActive = false;
    this.removeAttribute("active");
    if (this.animationId) {
      cancelAnimationFrame(this.animationId);
      this.animationId = undefined;
    }
    this.dispatchEvent(new CustomEvent("effect-ended", { bubbles: true, composed: true }));
  }

  private handleGlobalTrigger = () => this.trigger();

  connectedCallback() {
    super.connectedCallback();
    document.addEventListener("trigger-confetti", this.handleGlobalTrigger);
    (window as any).triggerConfetti = () => this.trigger();
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    document.removeEventListener("trigger-confetti", this.handleGlobalTrigger);
    delete (window as any).triggerConfetti;
    if (this.animationId) cancelAnimationFrame(this.animationId);
  }

  render() {
    return html`<canvas></canvas>`;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "fullpage-confetti": FullpageConfetti;
  }
  interface WindowEventMap {
    "trigger-confetti": CustomEvent;
  }
}
