import { LitElement, html, css } from 'lit';
import { property, state, query } from 'lit/decorators.js';
import type { VideoConfig } from './types.js';
import './composition.js';
import type { VideoComposition } from './composition.js';

export class VideoPlayer extends LitElement {
  static override styles = css`
    :host { display: flex; flex-direction: column; gap: 8px; }
    .viewport {
      overflow: hidden;
      position: relative;
      background: #000;
      border-radius: 8px;
    }
    .inner {
      position: relative;
      transform-origin: top left;
    }
    .controls {
      display: flex; gap: 4px; padding: 4px 0;
    }
    .controls button {
      background: none;
      border: 1px solid rgba(255,255,255,0.3);
      color: #fff;
      padding: 4px 12px;
      border-radius: 4px;
      cursor: pointer;
      font-size: 14px;
    }
    .controls button:hover { border-color: rgba(255,255,255,0.6); }
    .timeline {
      display: flex; align-items: center; gap: 8px; padding: 4px 0;
    }
    .timeline .frame-label {
      font-family: monospace; font-size: 12px; color: #aaa; min-width: 60px;
    }
    .track {
      flex: 1; height: 6px; background: rgba(255,255,255,0.2);
      border-radius: 3px; cursor: pointer; position: relative;
    }
    .track-fill {
      height: 100%; background: #3b82f6; border-radius: 3px;
    }
  `;

  @property({ type: Number }) width = 1920;
  @property({ type: Number }) height = 1080;
  @property({ type: Number }) fps = 30;
  @property({ type: Number, attribute: 'duration-in-frames' }) durationInFrames = 150;
  @property({ type: Number }) scale = 0.5;
  @property({ type: Number, attribute: 'controlled-frame' }) controlledFrame = -1;

  @state() private _frame = 0;
  @state() private _playing = false;

  @query('video-composition') private _composition!: VideoComposition;

  get config(): VideoConfig {
    return { width: this.width, height: this.height, fps: this.fps, durationInFrames: this.durationInFrames };
  }

  get compositionEl(): VideoComposition | null {
    return this._composition ?? null;
  }

  private _onFrameChange = (e: Event): void => {
    const detail = (e as CustomEvent<{ frame: number }>).detail;
    this._frame = detail.frame;
  };

  private _play(): void {
    this._composition?.play();
    this._playing = true;
  }

  private _pause(): void {
    this._composition?.pause();
    this._playing = false;
  }

  private _seekTo(frame: number): void {
    this._composition?.seekTo(frame);
    this._frame = frame;
  }

  private _onTrackClick(e: MouseEvent): void {
    const track = e.currentTarget as HTMLElement;
    const rect = track.getBoundingClientRect();
    const ratio = (e.clientX - rect.left) / rect.width;
    this._seekTo(Math.round(ratio * (this.durationInFrames - 1)));
  }

  override render() {
    const progress = this.durationInFrames > 1
      ? this._frame / (this.durationInFrames - 1) : 0;

    return html`
      <video-composition
        .width=${this.width}
        .height=${this.height}
        .fps=${this.fps}
        .durationInFrames=${this.durationInFrames}
        .controlledFrame=${this.controlledFrame}
        @frame-change=${this._onFrameChange}
      >
        <div class="viewport" style="width:${this.width * this.scale}px;height:${this.height * this.scale}px">
          <div class="inner" style="width:${this.width}px;height:${this.height}px;transform:scale(${this.scale})">
            <slot></slot>
          </div>
        </div>
      </video-composition>

      <div class="controls">
        <button @click=${() => this._seekTo(0)}>|&lt;</button>
        ${this._playing
          ? html`<button @click=${this._pause}>Pause</button>`
          : html`<button @click=${this._play}>Play</button>`
        }
      </div>

      <div class="timeline">
        <span class="frame-label">${this._frame}/${this.durationInFrames - 1}</span>
        <div class="track" @click=${this._onTrackClick}>
          <div class="track-fill" style="width:${progress * 100}%"></div>
        </div>
      </div>
    `;
  }
}

customElements.define('video-player', VideoPlayer);
