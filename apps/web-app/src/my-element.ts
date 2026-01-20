import { LitElement, html } from 'lit'
import { customElement, property } from 'lit/decorators.js'
import litLogo from './assets/lit.svg'
import viteLogo from '/vite.svg'

/**
 * An example element.
 *
 * @slot - This element has a slot
 * @csspart button - The button
 */
@customElement('my-element')
export class MyElement extends LitElement {
  /**
   * Copy for the read the docs hint.
   */
  @property()
  docsHint = 'Click on the Vite and Lit logos to learn more'

  /**
   * The number of times the button has been clicked.
   */
  @property({ type: Number })
  count = 0

  createRenderRoot() {
    return this
  }

  render() {
    return html`
      <div class="my-element">
        <div>
          <a href="https://vite.dev" target="_blank">
            <img src=${viteLogo} class="my-element__logo" alt="Vite logo" />
          </a>
          <a href="https://lit.dev" target="_blank">
            <img src=${litLogo} class="my-element__logo my-element__logo--lit" alt="Lit logo" />
          </a>
        </div>
        <slot></slot>
        <div class="my-element__card">
          <button @click=${this._onClick} class="my-element__btn" part="button">
            count is ${this.count}
          </button>
        </div>
        <p class="my-element__hint">${this.docsHint}</p>
      </div>
    `
  }

  private _onClick() {
    this.count++
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'my-element': MyElement
  }
}
