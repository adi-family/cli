import { LitElement, html, css } from 'lit';
import { property, state } from 'lit/decorators.js';

export class VideoSequence extends LitElement {
  static override styles = css`
    :host { display: contents; }
    :host([hidden]) { display: none; }
  `;

  @property({ type: Number }) from = 0;
  @property({ type: Number, attribute: 'duration-in-frames' }) durationInFrames = 0;
  @property({ type: Number, attribute: 'parent-offset' }) parentOffset = 0;

  @state() private _visible = false;

  get absoluteFrom(): number {
    return this.parentOffset + this.from;
  }

  get localFrame(): number {
    return this._currentFrame - this.absoluteFrom;
  }

  private _currentFrame = 0;

  override connectedCallback(): void {
    super.connectedCallback();
    this.addEventListener('frame-change', this._onFrameChange as EventListener);
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.removeEventListener('frame-change', this._onFrameChange as EventListener);
  }

  private _onFrameChange = (e: CustomEvent<{ frame: number }>): void => {
    this._currentFrame = e.detail.frame;
    const visible = this._currentFrame >= this.absoluteFrom &&
      this._currentFrame < this.absoluteFrom + this.durationInFrames;
    if (visible !== this._visible) {
      this._visible = visible;
    }
  };

  override render() {
    if (!this._visible) return html``;
    return html`<slot></slot>`;
  }
}

customElements.define('video-sequence', VideoSequence);
