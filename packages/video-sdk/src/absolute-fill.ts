import { LitElement, html, css } from 'lit';

export class VideoAbsoluteFill extends LitElement {
  static override styles = css`
    :host {
      display: block;
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
    }
  `;

  override render() {
    return html`<slot></slot>`;
  }
}

customElements.define('video-absolute-fill', VideoAbsoluteFill);
