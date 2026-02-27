import { LitElement, html, css } from 'lit';
import { property, state } from 'lit/decorators.js';
import type { VideoConfig } from './types.js';

export class VideoComposition extends LitElement {
  static override styles = css`
    :host { display: block; position: relative; }
  `;

  @property({ type: Number }) width = 1920;
  @property({ type: Number }) height = 1080;
  @property({ type: Number }) fps = 30;
  @property({ type: Number, attribute: 'duration-in-frames' }) durationInFrames = 150;
  @property({ type: Number, attribute: 'controlled-frame' }) controlledFrame = -1;

  @state() private _frame = 0;
  @state() private _playing = false;

  private _rafId = 0;
  private _lastTime = 0;

  get frame(): number {
    return this.controlledFrame >= 0 ? this.controlledFrame : this._frame;
  }

  get playing(): boolean { return this._playing; }

  get config(): VideoConfig {
    return {
      width: this.width,
      height: this.height,
      fps: this.fps,
      durationInFrames: this.durationInFrames,
    };
  }

  play(): void {
    if (this.controlledFrame >= 0) return;
    this._lastTime = 0;
    this._playing = true;
    this._rafId = requestAnimationFrame(t => this._tick(t));
  }

  pause(): void {
    this._playing = false;
    cancelAnimationFrame(this._rafId);
  }

  seekTo(frame: number): void {
    this._frame = Math.max(0, Math.min(frame, this.durationInFrames - 1));
    this._dispatchFrameChange();
  }

  private _tick(timestamp: number): void {
    if (!this._playing) return;
    if (!this._lastTime) this._lastTime = timestamp;

    const elapsed = timestamp - this._lastTime;
    const frameDuration = 1000 / this.fps;

    if (elapsed >= frameDuration) {
      this._lastTime = timestamp - (elapsed % frameDuration);
      const next = this._frame + 1;
      this._frame = next >= this.durationInFrames ? 0 : next;
      this._dispatchFrameChange();
    }

    this._rafId = requestAnimationFrame(t => this._tick(t));
  }

  private _dispatchFrameChange(): void {
    this.dispatchEvent(new CustomEvent('frame-change', {
      detail: { frame: this.frame },
      bubbles: true,
      composed: true,
    }));
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    cancelAnimationFrame(this._rafId);
  }

  override updated(changed: Map<PropertyKey, unknown>): void {
    if (changed.has('controlledFrame') && this.controlledFrame >= 0) {
      this._dispatchFrameChange();
    }
  }

  override render() {
    return html`<slot></slot>`;
  }
}

customElements.define('video-composition', VideoComposition);
