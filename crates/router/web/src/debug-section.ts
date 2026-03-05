import { LitElement, html, nothing } from 'lit';
import { customElement, state } from 'lit/decorators.js';

export interface RouterDebugRoute {
  path: string;
}

@customElement('adi-router-debug')
export class AdiRouterDebugElement extends LitElement {
  @state() routes: RouterDebugRoute[] = [];
  @state() currentPath = '';

  override createRenderRoot() {
    return this;
  }

  override render() {
    return html`
      <div style="display:flex;flex-direction:column;gap:1rem">
        <section>
          <div class="text-xs uppercase" style="color:var(--adi-text-muted);font-weight:600;margin-bottom:0.5rem">
            Current Path
          </div>
          <code class="text-sm">${this.currentPath || '/'}</code>
        </section>

        <section>
          <div class="text-xs uppercase" style="color:var(--adi-text-muted);font-weight:600;margin-bottom:0.5rem">
            Registered Routes (${this.routes.length})
          </div>
          ${this.routes.length === 0
            ? html`<p class="text-sm" style="color:var(--adi-text-muted)">No routes registered.</p>`
            : html`
              <table class="dr-table text-sm">
                <thead>
                  <tr>
                    <th class="dr-th">Path</th>
                    <th class="dr-th">Active</th>
                  </tr>
                </thead>
                <tbody>
                  ${this.routes.map(
                    (r) => html`
                      <tr class="dr-row">
                        <td class="dr-td"><code>${r.path}</code></td>
                        <td class="dr-td">${this.isActive(r.path) ? html`<span style="color:var(--adi-accent)">●</span>` : nothing}</td>
                      </tr>
                    `,
                  )}
                </tbody>
              </table>
            `}
        </section>
      </div>
    `;
  }

  private isActive(routePath: string): boolean {
    return this.currentPath === routePath || this.currentPath.startsWith(routePath + '/');
  }
}
