import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";
import { triggerSound } from "../sounds";

interface Star {
  x: number;
  y: number;
  z: number;
  size: number;
  color: string;
}

/**
 * Full-page starfield/warp speed overlay effect.
 * 
 * Usage:
 *   <fullpage-starfield id="starfield"></fullpage-starfield>
 * 
 * Trigger via:
 *   - Method: document.getElementById('starfield').trigger()
 *   - Event: document.dispatchEvent(new CustomEvent('trigger-starfield'))
 *   - Global: window.triggerStarfield()
 */
@customElement("fullpage-starfield")
export class FullpageStarfield extends LitElement {
  @property({ type: Number }) duration: number = 3000;
  @property({ type: Number }) speed: number = 5;

  private canvas?: HTMLCanvasElement;
  private ctx?: CanvasRenderingContext2D;
  private animationId?: number;
  private stars: Star[] = [];
  private isActive = false;
  private startTime = 0;
  private centerX = 0;
  private centerY = 0;
  private colors = ["#ffffff", "#a78bfa", "#67e8f9", "#f472b6", "#fbbf24"];

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

  private initStars(width: number, height: number) {
    this.stars = [];
    this.centerX = width / 2;
    this.centerY = height / 2;
    const count = 300;
    
    for (let i = 0; i < count; i++) {
      this.stars.push({
        x: Math.random() * width - width / 2,
        y: Math.random() * height - height / 2,
        z: Math.random() * 1000,
        size: 0.5 + Math.random() * 2,
        color: this.colors[Math.floor(Math.random() * this.colors.length)],
      });
    }
  }

  private runAnimation = () => {
    if (!this.ctx || !this.canvas || !this.isActive) return;

    const width = this.canvas.width / window.devicePixelRatio;
    const height = this.canvas.height / window.devicePixelRatio;
    const elapsed = Date.now() - this.startTime;
    const progress = Math.min(1, elapsed / this.duration);
    
    // Speed curve - accelerate then decelerate
    let currentSpeed: number;
    let alpha: number;
    if (progress < 0.15) {
      currentSpeed = this.speed * (progress / 0.15);
      alpha = progress / 0.15;
    } else if (progress > 0.75) {
      currentSpeed = this.speed * ((1 - progress) / 0.25);
      alpha = (1 - progress) / 0.25;
    } else {
      currentSpeed = this.speed;
      alpha = 1;
    }

    // Clear canvas (transparent)
    this.ctx.clearRect(0, 0, width, height);

    for (const star of this.stars) {
      star.z -= currentSpeed * 5;

      if (star.z <= 0) {
        star.x = Math.random() * width - width / 2;
        star.y = Math.random() * height - height / 2;
        star.z = 1000;
      }

      const perspective = 400 / (star.z + 1);
      const screenX = this.centerX + star.x * perspective;
      const screenY = this.centerY + star.y * perspective;
      const size = star.size * perspective * alpha;

      if (screenX < -20 || screenX > width + 20 || screenY < -20 || screenY > height + 20) continue;

      const starAlpha = Math.min(1, (1000 - star.z) / 400) * alpha;

      // Draw streak
      if (star.z < 600 && currentSpeed > 5) {
        const streakLength = Math.min(50, (600 - star.z) / 600 * 40 * (currentSpeed / this.speed));
        const prevPerspective = 400 / (star.z + currentSpeed * 5 + 1);
        const prevX = this.centerX + star.x * prevPerspective;
        const prevY = this.centerY + star.y * prevPerspective;
        
        const gradient = this.ctx.createLinearGradient(prevX, prevY, screenX, screenY);
        gradient.addColorStop(0, `${star.color}00`);
        gradient.addColorStop(1, star.color);
        
        this.ctx.beginPath();
        this.ctx.moveTo(prevX, prevY);
        this.ctx.lineTo(screenX, screenY);
        this.ctx.strokeStyle = gradient;
        this.ctx.globalAlpha = starAlpha * 0.7;
        this.ctx.lineWidth = size * 0.8;
        this.ctx.stroke();
      }

      // Draw star
      this.ctx.globalAlpha = starAlpha;
      this.ctx.beginPath();
      this.ctx.arc(screenX, screenY, size, 0, Math.PI * 2);
      this.ctx.fillStyle = star.color;
      this.ctx.fill();

      // Glow
      this.ctx.beginPath();
      this.ctx.arc(screenX, screenY, size * 2.5, 0, Math.PI * 2);
      this.ctx.fillStyle = star.color;
      this.ctx.globalAlpha = starAlpha * 0.15;
      this.ctx.fill();
    }

    this.ctx.globalAlpha = 1;

    if (progress < 1) {
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

  /** Trigger the starfield effect */
  trigger() {
    this.isActive = true;
    this.setAttribute("active", "");
    this.startTime = Date.now();
    
    if (!this.canvas) {
      this.canvas = this.shadowRoot?.querySelector("canvas") as HTMLCanvasElement;
      this.ctx = this.canvas?.getContext("2d") ?? undefined;
    }
    
    this.resizeCanvas();
    const width = this.canvas!.width / window.devicePixelRatio;
    const height = this.canvas!.height / window.devicePixelRatio;
    this.initStars(width, height);
    
    // Play whoosh sound for warp speed
    triggerSound("whoosh");
    
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
    if (this.ctx && this.canvas) {
      const width = this.canvas.width / window.devicePixelRatio;
      const height = this.canvas.height / window.devicePixelRatio;
      this.ctx.clearRect(0, 0, width, height);
    }
    this.dispatchEvent(new CustomEvent("effect-ended", { bubbles: true, composed: true }));
  }

  private handleGlobalTrigger = () => this.trigger();

  connectedCallback() {
    super.connectedCallback();
    document.addEventListener("trigger-starfield", this.handleGlobalTrigger);
    (window as any).triggerStarfield = () => this.trigger();
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    document.removeEventListener("trigger-starfield", this.handleGlobalTrigger);
    delete (window as any).triggerStarfield;
    if (this.animationId) cancelAnimationFrame(this.animationId);
  }

  render() {
    return html`<canvas></canvas>`;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "fullpage-starfield": FullpageStarfield;
  }
  interface WindowEventMap {
    "trigger-starfield": CustomEvent;
  }
}
