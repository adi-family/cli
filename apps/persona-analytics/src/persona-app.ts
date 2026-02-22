import { LitElement, html } from 'lit';
import { customElement } from 'lit/decorators.js';

@customElement('persona-app')
export class PersonaApp extends LitElement {
  createRenderRoot() { return this; }

  render() {
    return html`
      <header class="glass" style="display:flex;align-items:center;justify-content:space-between;padding:calc(var(--l) * 0.5) calc(var(--l) * 1);border-bottom:1px solid var(--adi-border)">
        <div style="display:flex;align-items:center;gap:0.5rem">
          <svg width="20" height="20" viewBox="0 0 100 100" fill="none">
            <rect width="100" height="100" stroke="var(--adi-accent)" />
            <path d="M50 20L80 50L50 80L20 50L50 20Z" fill="var(--adi-accent)" />
          </svg>
          <span class="text-lg" style="font-weight:600;color:var(--adi-text)">Persona Analytics</span>
        </div>
      </header>

      <main class="container-wide" style="padding-top:calc(var(--l) * 3);padding-bottom:calc(var(--l) * 3)">
        <h1 class="text-3xl" style="font-weight:200;letter-spacing:-0.04em;color:var(--adi-text);line-height:1.2;margin-bottom:calc(var(--l) * 0.5)">Analytics</h1>
        <p class="text-lg" style="color:var(--adi-text-muted);margin-bottom:calc(var(--l) * 3)">Usage metrics and persona insights</p>

        <div class="card-grid" style="grid-template-columns:repeat(3,1fr);border-radius:var(--radius-lg);overflow:hidden;border:1px solid var(--adi-border)">
          <div class="card-hover p-v-1 p-h-15">
            <div class="text-xs" style="font-weight:500;letter-spacing:0.06em;text-transform:uppercase;color:var(--adi-text-muted);margin-bottom:calc(var(--l) * 0.25)">Active Personas</div>
            <div class="text-2xl" style="font-weight:200;letter-spacing:-0.04em;color:var(--adi-text)">—</div>
            <div class="text-sm" style="color:var(--adi-text-muted);margin-top:calc(var(--l) * 0.25)">awaiting data</div>
          </div>
          <div class="card-hover p-v-1 p-h-15">
            <div class="text-xs" style="font-weight:500;letter-spacing:0.06em;text-transform:uppercase;color:var(--adi-text-muted);margin-bottom:calc(var(--l) * 0.25)">Interactions</div>
            <div class="text-2xl" style="font-weight:200;letter-spacing:-0.04em;color:var(--adi-text)">—</div>
            <div class="text-sm" style="color:var(--adi-text-muted);margin-top:calc(var(--l) * 0.25)">awaiting data</div>
          </div>
          <div class="card-hover p-v-1 p-h-15">
            <div class="text-xs" style="font-weight:500;letter-spacing:0.06em;text-transform:uppercase;color:var(--adi-text-muted);margin-bottom:calc(var(--l) * 0.25)">Avg. Session</div>
            <div class="text-2xl" style="font-weight:200;letter-spacing:-0.04em;color:var(--adi-text)">—</div>
            <div class="text-sm" style="color:var(--adi-text-muted);margin-top:calc(var(--l) * 0.25)">awaiting data</div>
          </div>
        </div>
      </main>
    `;
  }
}
