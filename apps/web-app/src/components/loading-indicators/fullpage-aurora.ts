import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";

/**
 * Full-page aurora/northern lights overlay effect.
 * 
 * Usage:
 *   <fullpage-aurora id="aurora"></fullpage-aurora>
 * 
 * Trigger via:
 *   - Method: document.getElementById('aurora').trigger()
 *   - Event: document.dispatchEvent(new CustomEvent('trigger-aurora'))
 *   - Global: window.triggerAurora()
 */
@customElement("fullpage-aurora")
export class FullpageAurora extends LitElement {
  @property({ type: Number }) duration: number = 4000;

  private canvas?: HTMLCanvasElement;
  private ctx?: CanvasRenderingContext2D;
  private animationId?: number;
  private isActive = false;
  private startTime = 0;
  private time = 0;

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

  private runAnimation = () => {
    if (!this.ctx || !this.canvas || !this.isActive) return;

    const width = this.canvas.width / window.devicePixelRatio;
    const height = this.canvas.height / window.devicePixelRatio;
    const elapsed = Date.now() - this.startTime;
    const progress = Math.min(1, elapsed / this.duration);
    
    // Fade in/out curve
    let alpha: number;
    if (progress < 0.2) {
      alpha = progress / 0.2;
    } else if (progress > 0.7) {
      alpha = (1 - progress) / 0.3;
    } else {
      alpha = 1;
    }

    this.ctx.clearRect(0, 0, width, height);
    this.time += 0.02;

    // Draw flowing aurora bands
    this.ctx.globalCompositeOperation = "screen";
    
    for (let i = 0; i < 4; i++) {
      this.ctx.beginPath();
      this.ctx.moveTo(0, height * 0.3);
      
      for (let x = 0; x <= width; x += 8) {
        const y = height * 0.25 + 
          Math.sin(x * 0.008 + this.time * 2 + i * 0.8) * 40 +
          Math.sin(x * 0.015 + this.time * 1.5 + i * 1.5) * 30 +
          Math.sin(x * 0.003 + this.time * 0.8 + i * 2) * 60 +
          i * 30;
        this.ctx.lineTo(x, y);
      }
      
      this.ctx.lineTo(width, height * 0.8);
      this.ctx.lineTo(0, height * 0.8);
      this.ctx.closePath();
      
      const gradient = this.ctx.createLinearGradient(0, 0, 0, height * 0.8);
      const hue = 160 + i * 35 + Math.sin(this.time + i) * 20;
      gradient.addColorStop(0, `hsla(${hue}, 80%, 60%, 0)`);
      gradient.addColorStop(0.2, `hsla(${hue}, 80%, 55%, ${0.15 * alpha})`);
      gradient.addColorStop(0.4, `hsla(${hue + 30}, 75%, 50%, ${0.25 * alpha})`);
      gradient.addColorStop(0.6, `hsla(${hue + 60}, 80%, 55%, ${0.15 * alpha})`);
      gradient.addColorStop(1, `hsla(${hue + 60}, 80%, 60%, 0)`);
      
      this.ctx.fillStyle = gradient;
      this.ctx.fill();
    }

    // Add shimmer particles
    this.ctx.globalCompositeOperation = "source-over";
    for (let i = 0; i < 30; i++) {
      const px = (Math.sin(this.time * 0.5 + i * 0.7) * 0.5 + 0.5) * width;
      const py = (Math.sin(this.time * 0.3 + i * 0.5) * 0.25 + 0.25) * height;
      const size = 1.5 + Math.sin(this.time * 3 + i) * 0.8;
      const sparkleAlpha = (0.4 + Math.sin(this.time * 4 + i * 2) * 0.3) * alpha;
      
      this.ctx.beginPath();
      this.ctx.arc(px, py, size, 0, Math.PI * 2);
      this.ctx.fillStyle = `rgba(255, 255, 255, ${sparkleAlpha})`;
      this.ctx.fill();
    }

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

  /** Trigger the aurora effect */
  trigger() {
    this.isActive = true;
    this.setAttribute("active", "");
    this.startTime = Date.now();
    this.time = 0;
    
    if (!this.canvas) {
      this.canvas = this.shadowRoot?.querySelector("canvas") as HTMLCanvasElement;
      this.ctx = this.canvas?.getContext("2d") ?? undefined;
    }
    
    this.resizeCanvas();
    
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
    document.addEventListener("trigger-aurora", this.handleGlobalTrigger);
    (window as any).triggerAurora = () => this.trigger();
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    document.removeEventListener("trigger-aurora", this.handleGlobalTrigger);
    delete (window as any).triggerAurora;
    if (this.animationId) cancelAnimationFrame(this.animationId);
  }

  render() {
    return html`<canvas></canvas>`;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "fullpage-aurora": FullpageAurora;
  }
  interface WindowEventMap {
    "trigger-aurora": CustomEvent;
  }
}
