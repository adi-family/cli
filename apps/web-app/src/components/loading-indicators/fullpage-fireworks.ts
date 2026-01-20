import { LitElement, html, css } from "lit";
import { customElement, property } from "lit/decorators.js";
import { triggerSound } from "../sounds";

interface Rocket {
  x: number;
  y: number;
  vy: number;
  targetY: number;
  color: string;
  prevY: number;
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

/**
 * Full-page fireworks overlay effect.
 * 
 * Usage:
 *   <fullpage-fireworks id="fireworks"></fullpage-fireworks>
 * 
 * Trigger via:
 *   - Method: document.getElementById('fireworks').trigger()
 *   - Event: document.dispatchEvent(new CustomEvent('trigger-fireworks'))
 *   - Global: window.triggerFireworks()
 */
@customElement("fullpage-fireworks")
export class FullpageFireworks extends LitElement {
  @property({ type: Number }) rockets: number = 5;

  private canvas?: HTMLCanvasElement;
  private ctx?: CanvasRenderingContext2D;
  private animationId?: number;
  private rocketList: Rocket[] = [];
  private sparks: Spark[] = [];
  private isActive = false;
  private launchCount = 0;
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
      background: rgba(0, 0, 0, 0.25);
      animation: fadeIn 0.3s ease-out;
    }

    :host([fading]) {
      animation: fadeOut 0.3s ease-out forwards;
    }

    @keyframes fadeIn {
      from { opacity: 0; }
      to { opacity: 1; }
    }

    @keyframes fadeOut {
      from { opacity: 1; }
      to { opacity: 0; }
    }

    canvas {
      width: 100%;
      height: 100%;
    }
  `;

  private launchRocket() {
    if (!this.canvas) return;
    
    const width = this.canvas.width / window.devicePixelRatio;
    const height = this.canvas.height / window.devicePixelRatio;
    const x = width * 0.15 + Math.random() * width * 0.7;
    const y = height + 10;
    
    this.rocketList.push({
      x,
      y,
      prevY: y,
      vy: -10 - Math.random() * 3,
      targetY: height * 0.15 + Math.random() * height * 0.35,
      color: this.colors[Math.floor(Math.random() * this.colors.length)],
    });
  }

  private explode(rocket: Rocket) {
    // Play firework explosion sound
    triggerSound("firework", 0.3);
    
    const sparkCount = 35;
    const isRing = Math.random() < 0.5;
    
    for (let i = 0; i < sparkCount; i++) {
      const angle = isRing ? (Math.PI * 2 * i) / sparkCount : Math.random() * Math.PI * 2;
      const speed = isRing ? 3 + Math.random() * 1.5 : 1.5 + Math.random() * 4;
      this.sparks.push({
        x: rocket.x,
        y: rocket.y,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed,
        life: 1,
        color: rocket.color,
        size: 2.5,
      });
    }

    // White center sparkles
    for (let i = 0; i < 8; i++) {
      const angle = Math.random() * Math.PI * 2;
      const speed = 0.8 + Math.random() * 2;
      this.sparks.push({
        x: rocket.x,
        y: rocket.y,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed,
        life: 1,
        color: "#ffffff",
        size: 2,
      });
    }
  }

  private runAnimation = () => {
    if (!this.ctx || !this.canvas || !this.isActive) return;

    const width = this.canvas.width / window.devicePixelRatio;
    const height = this.canvas.height / window.devicePixelRatio;
    
    this.ctx.clearRect(0, 0, width, height);

    // Batch rockets by color - draw trails as lines
    this.rocketList = this.rocketList.filter((r) => {
      r.prevY = r.y;
      r.y += r.vy;
      r.vy += 0.05; // Gravity slows rocket as it rises

      // Draw trail as gradient line
      const gradient = this.ctx!.createLinearGradient(r.x, r.prevY, r.x, r.y);
      gradient.addColorStop(0, "rgba(255, 255, 255, 0)");
      gradient.addColorStop(1, "rgba(255, 255, 255, 0.8)");
      this.ctx!.strokeStyle = gradient;
      this.ctx!.lineWidth = 2;
      this.ctx!.beginPath();
      this.ctx!.moveTo(r.x, r.prevY);
      this.ctx!.lineTo(r.x, r.y);
      this.ctx!.stroke();

      // Draw rocket head
      this.ctx!.fillStyle = r.color;
      this.ctx!.beginPath();
      this.ctx!.arc(r.x, r.y, 3, 0, Math.PI * 2);
      this.ctx!.fill();

      if (r.y <= r.targetY) {
        this.explode(r);
        return false;
      }
      return true;
    });

    // Batch sparks - group by color, minimal state changes
    const sparksByColor = new Map<string, Spark[]>();
    
    this.sparks = this.sparks.filter((s) => {
      s.x += s.vx;
      s.y += s.vy;
      s.vy += 0.1;
      s.vx *= 0.98;
      s.life -= 0.02;

      if (s.life <= 0) return false;

      const colorKey = s.color + Math.floor(s.life * 5); // Group by color + alpha bucket
      if (!sparksByColor.has(colorKey)) sparksByColor.set(colorKey, []);
      sparksByColor.get(colorKey)!.push(s);
      
      return true;
    });

    // Draw sparks batched by color
    for (const [colorKey, sparks] of sparksByColor) {
      const alpha = (parseInt(colorKey.slice(-1)) + 1) / 5;
      const color = colorKey.slice(0, -1);
      
      this.ctx.globalAlpha = alpha;
      this.ctx.fillStyle = color;
      this.ctx.beginPath();
      
      for (const s of sparks) {
        const radius = s.size * s.life;
        this.ctx.moveTo(s.x + radius, s.y);
        this.ctx.arc(s.x, s.y, radius, 0, Math.PI * 2);
      }
      
      this.ctx.fill();
    }
    
    this.ctx.globalAlpha = 1;

    if (this.rocketList.length > 0 || this.sparks.length > 0) {
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

  /** Trigger the fireworks effect */
  trigger() {
    this.isActive = true;
    this.setAttribute("active", "");
    this.rocketList = [];
    this.sparks = [];
    this.launchCount = 0;
    
    if (!this.canvas) {
      this.canvas = this.shadowRoot?.querySelector("canvas") as HTMLCanvasElement;
      this.ctx = this.canvas?.getContext("2d") ?? undefined;
    }
    
    this.resizeCanvas();
    
    // Launch rockets with staggered timing
    const launchNext = () => {
      if (this.launchCount < this.rockets) {
        this.launchRocket();
        this.launchCount++;
        setTimeout(launchNext, 200 + Math.random() * 300);
      }
    };
    launchNext();
    
    if (this.animationId) cancelAnimationFrame(this.animationId);
    this.runAnimation();

    this.dispatchEvent(new CustomEvent("effect-started", { bubbles: true, composed: true }));
  }

  private stop() {
    this.isActive = false;
    this.setAttribute("fading", "");
    
    if (this.animationId) {
      cancelAnimationFrame(this.animationId);
      this.animationId = undefined;
    }
    
    // Wait for fade out animation
    setTimeout(() => {
      this.removeAttribute("active");
      this.removeAttribute("fading");
      if (this.ctx && this.canvas) {
        const width = this.canvas.width / window.devicePixelRatio;
        const height = this.canvas.height / window.devicePixelRatio;
        this.ctx.clearRect(0, 0, width, height);
      }
      this.dispatchEvent(new CustomEvent("effect-ended", { bubbles: true, composed: true }));
    }, 300);
  }

  private handleGlobalTrigger = () => this.trigger();

  connectedCallback() {
    super.connectedCallback();
    document.addEventListener("trigger-fireworks", this.handleGlobalTrigger);
    (window as any).triggerFireworks = () => this.trigger();
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    document.removeEventListener("trigger-fireworks", this.handleGlobalTrigger);
    delete (window as any).triggerFireworks;
    if (this.animationId) cancelAnimationFrame(this.animationId);
  }

  render() {
    return html`<canvas></canvas>`;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "fullpage-fireworks": FullpageFireworks;
  }
  interface WindowEventMap {
    "trigger-fireworks": CustomEvent;
  }
}
