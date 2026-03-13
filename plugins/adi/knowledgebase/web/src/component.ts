import { LitElement, html } from 'lit';
import { state } from 'lit/decorators.js';

export class AdiKnowledgebaseElement extends LitElement {
  @state() private loading = true;

  override connectedCallback(): void {
    super.connectedCallback();
    this.loading = false;
  }

  override render() {
    if (this.loading) {
      return html`<div>Loading knowledgebase...</div>`;
    }
    return html`<div>Knowledgebase</div>`;
  }
}
