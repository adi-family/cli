import { LitElement, html, css } from "lit";
import { customElement, state } from "lit/decorators.js";
import "./loading-indicators";

type TabId = "loading" | "buttons" | "inputs" | "feedback";

interface ComponentInfo {
  name: string;
  tag: string;
  description: string;
  props: { name: string; type: string; default?: string; description: string }[];
}

@customElement("components-page")
export class ComponentsPage extends LitElement {
  @state() private activeTab: TabId = "loading";
  @state() private selectedComponent: string | null = null;

  private loadingComponents: ComponentInfo[] = [
    {
      name: "Loading Skeleton",
      tag: "loading-skeleton",
      description: "Shimmer placeholder effect for content loading states",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the skeleton" },
        { name: "variant", type: '"card" | "text" | "avatar"', default: '"card"', description: "Shape variant" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
    {
      name: "Ripple Effect",
      tag: "ripple-effect",
      description: "Expanding water ripple animation",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the ripple" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
    {
      name: "Morphing Blob",
      tag: "morphing-blob",
      description: "Organic SVG shape that continuously morphs",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the blob" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
    {
      name: "Matrix Rain",
      tag: "matrix-rain",
      description: "Falling characters effect inspired by The Matrix",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the canvas" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
    {
      name: "Particle Explosion",
      tag: "particle-explosion",
      description: "Canvas-based particles that burst and regenerate",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the canvas" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
    {
      name: "Gradient Sweep",
      tag: "gradient-sweep",
      description: "Circular progress with animated sweeping gradient",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the circle" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
    {
      name: "Wave Bar",
      tag: "wave-bar",
      description: "Audio visualizer-style bouncing bars",
      props: [
        { name: "size", type: '"sm" | "md" | "lg"', default: '"md"', description: "Size of the bars" },
        { name: "bars", type: "number", default: "5", description: "Number of bars" },
        { name: "label", type: "string", description: "Optional label below" },
      ],
    },
  ];

  static styles = css`
    :host {
      display: block;
      min-height: calc(100vh - 4rem);
      background: #0d0a14;
      color: #d1d5db;
      font-family: 'Inter', system-ui, sans-serif;
      overflow: auto;
    }

    .page {
      max-width: 1400px;
      margin: 0 auto;
      padding: 2rem;
      padding-bottom: 4rem;
    }

    .header {
      margin-bottom: 2rem;
    }

    .header h1 {
      font-size: 2rem;
      font-weight: 700;
      color: white;
      margin: 0 0 0.5rem;
    }

    .header p {
      color: #9ca3af;
      margin: 0;
    }

    .tabs {
      display: flex;
      gap: 0.5rem;
      margin-bottom: 2rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
      padding-bottom: 1rem;
    }

    .tab {
      padding: 0.625rem 1rem;
      border: none;
      background: transparent;
      color: #9ca3af;
      font-size: 0.875rem;
      font-weight: 500;
      cursor: pointer;
      border-radius: 0.5rem;
      transition: all 0.2s;
      font-family: inherit;
    }

    .tab:hover {
      color: white;
      background: rgba(139, 92, 246, 0.1);
    }

    .tab.active {
      color: white;
      background: rgba(139, 92, 246, 0.2);
    }

    .tab.disabled {
      opacity: 0.4;
      cursor: not-allowed;
    }

    .content {
      display: grid;
      grid-template-columns: 260px 1fr;
      gap: 2rem;
    }

    @media (max-width: 900px) {
      .content {
        grid-template-columns: 1fr;
      }
    }

    .sidebar {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }

    .sidebar-item {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      padding: 0.75rem 1rem;
      border: none;
      background: transparent;
      color: #9ca3af;
      font-size: 0.875rem;
      text-align: left;
      cursor: pointer;
      border-radius: 0.5rem;
      transition: all 0.2s;
      font-family: inherit;
    }

    .sidebar-item:hover {
      color: white;
      background: rgba(255, 255, 255, 0.05);
    }

    .sidebar-item.active {
      color: white;
      background: rgba(139, 92, 246, 0.15);
      border-left: 2px solid #8b5cf6;
    }

    .sidebar-item-icon {
      width: 32px;
      height: 32px;
      display: flex;
      align-items: center;
      justify-content: center;
      background: rgba(139, 92, 246, 0.1);
      border-radius: 0.375rem;
      flex-shrink: 0;
      overflow: hidden;
    }

    .main {
      background: #13101c;
      border-radius: 1rem;
      border: 1px solid rgba(255, 255, 255, 0.1);
      overflow: hidden;
    }

    .main-header {
      padding: 1.5rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }

    .main-header h2 {
      font-size: 1.25rem;
      font-weight: 600;
      color: white;
      margin: 0 0 0.25rem;
    }

    .main-header p {
      color: #6b7280;
      font-size: 0.875rem;
      margin: 0;
    }

    .preview {
      padding: 2rem;
      background: rgba(0, 0, 0, 0.2);
      display: flex;
      justify-content: center;
      align-items: center;
      min-height: 200px;
    }

    .preview-sizes {
      display: flex;
      gap: 3rem;
      align-items: flex-end;
    }

    .preview-item {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 0.75rem;
    }

    .preview-item span {
      font-size: 0.75rem;
      color: #6b7280;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }

    .props {
      padding: 1.5rem;
    }

    .props h3 {
      font-size: 0.875rem;
      font-weight: 600;
      color: white;
      margin: 0 0 1rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }

    .props-table {
      width: 100%;
      border-collapse: collapse;
      font-size: 0.875rem;
    }

    .props-table th {
      text-align: left;
      padding: 0.75rem;
      color: #9ca3af;
      font-weight: 500;
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }

    .props-table td {
      padding: 0.75rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    }

    .props-table tr:last-child td {
      border-bottom: none;
    }

    .prop-name {
      color: #a78bfa;
      font-family: 'JetBrains Mono', monospace;
    }

    .prop-type {
      color: #6b7280;
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.75rem;
    }

    .prop-default {
      color: #4ade80;
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.75rem;
    }

    .code {
      padding: 1.5rem;
      border-top: 1px solid rgba(255, 255, 255, 0.1);
    }

    .code h3 {
      font-size: 0.875rem;
      font-weight: 600;
      color: white;
      margin: 0 0 1rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }

    .code-block {
      background: #0d0a14;
      border-radius: 0.5rem;
      padding: 1rem;
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.8125rem;
      color: #a78bfa;
      overflow-x: auto;
    }

    .code-tag { color: #f472b6; }
    .code-attr { color: #67e8f9; }
    .code-value { color: #4ade80; }

    .empty {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      padding: 4rem;
      color: #6b7280;
      text-align: center;
    }

    .empty-icon {
      font-size: 3rem;
      margin-bottom: 1rem;
      opacity: 0.5;
    }
  `;

  private renderSidebarIcon(tag: string) {
    switch (tag) {
      case "loading-skeleton":
        return html`<loading-skeleton size="sm" variant="text"></loading-skeleton>`;
      case "ripple-effect":
        return html`<ripple-effect size="sm"></ripple-effect>`;
      case "morphing-blob":
        return html`<morphing-blob size="sm"></morphing-blob>`;
      case "matrix-rain":
        return html`<matrix-rain size="sm"></matrix-rain>`;
      case "particle-explosion":
        return html`<particle-explosion size="sm"></particle-explosion>`;
      case "gradient-sweep":
        return html`<gradient-sweep size="sm"></gradient-sweep>`;
      case "wave-bar":
        return html`<wave-bar size="sm" bars="3"></wave-bar>`;
      default:
        return html``;
    }
  }

  private renderSidebar() {
    const components = this.activeTab === "loading" ? this.loadingComponents : [];

    return html`
      <div class="sidebar">
        ${components.map(
          (c) => html`
            <button
              class="sidebar-item ${this.selectedComponent === c.tag ? "active" : ""}"
              @click=${() => (this.selectedComponent = c.tag)}
            >
              <div class="sidebar-item-icon">
                ${this.renderSidebarIcon(c.tag)}
              </div>
              ${c.name}
            </button>
          `
        )}
      </div>
    `;
  }

  private renderComponentPreview(tag: string) {
    switch (tag) {
      case "loading-skeleton":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <loading-skeleton size="sm" variant="card"></loading-skeleton>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <loading-skeleton size="md" variant="card"></loading-skeleton>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <loading-skeleton size="lg" variant="card"></loading-skeleton>
              <span>Large</span>
            </div>
          </div>
        `;
      case "ripple-effect":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <ripple-effect size="sm"></ripple-effect>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <ripple-effect size="md"></ripple-effect>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <ripple-effect size="lg"></ripple-effect>
              <span>Large</span>
            </div>
          </div>
        `;
      case "morphing-blob":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <morphing-blob size="sm"></morphing-blob>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <morphing-blob size="md"></morphing-blob>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <morphing-blob size="lg"></morphing-blob>
              <span>Large</span>
            </div>
          </div>
        `;
      case "matrix-rain":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <matrix-rain size="sm"></matrix-rain>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <matrix-rain size="md"></matrix-rain>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <matrix-rain size="lg"></matrix-rain>
              <span>Large</span>
            </div>
          </div>
        `;
      case "particle-explosion":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <particle-explosion size="sm"></particle-explosion>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <particle-explosion size="md"></particle-explosion>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <particle-explosion size="lg"></particle-explosion>
              <span>Large</span>
            </div>
          </div>
        `;
      case "gradient-sweep":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <gradient-sweep size="sm"></gradient-sweep>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <gradient-sweep size="md"></gradient-sweep>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <gradient-sweep size="lg"></gradient-sweep>
              <span>Large</span>
            </div>
          </div>
        `;
      case "wave-bar":
        return html`
          <div class="preview-sizes">
            <div class="preview-item">
              <wave-bar size="sm"></wave-bar>
              <span>Small</span>
            </div>
            <div class="preview-item">
              <wave-bar size="md" bars="7"></wave-bar>
              <span>Medium</span>
            </div>
            <div class="preview-item">
              <wave-bar size="lg" bars="9"></wave-bar>
              <span>Large</span>
            </div>
          </div>
        `;
      default:
        return html``;
    }
  }

  private renderCodeExample(component: ComponentInfo) {
    const tag = component.tag;
    
    return html`
      <div class="code-block">
        <span class="code-tag">&lt;${tag}</span>
        <span class="code-attr"> size</span>=<span class="code-value">"md"</span>${tag === "wave-bar" ? html`<span class="code-attr"> bars</span>=<span class="code-value">"7"</span>` : ""}${tag === "loading-skeleton" ? html`<span class="code-attr"> variant</span>=<span class="code-value">"card"</span>` : ""}<span class="code-tag">&gt;&lt;/${tag}&gt;</span>
      </div>
    `;
  }

  private renderMain() {
    const components = this.activeTab === "loading" ? this.loadingComponents : [];
    const component = components.find((c) => c.tag === this.selectedComponent);

    if (!component) {
      return html`
        <div class="main">
          <div class="empty">
            <div class="empty-icon">📦</div>
            <p>Select a component from the sidebar</p>
          </div>
        </div>
      `;
    }

    return html`
      <div class="main">
        <div class="main-header">
          <h2>${component.name}</h2>
          <p>${component.description}</p>
        </div>

        <div class="preview">
          ${this.renderComponentPreview(component.tag)}
        </div>

        <div class="props">
          <h3>Properties</h3>
          <table class="props-table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Type</th>
                <th>Default</th>
                <th>Description</th>
              </tr>
            </thead>
            <tbody>
              ${component.props.map(
                (prop) => html`
                  <tr>
                    <td class="prop-name">${prop.name}</td>
                    <td class="prop-type">${prop.type}</td>
                    <td class="prop-default">${prop.default || "-"}</td>
                    <td>${prop.description}</td>
                  </tr>
                `
              )}
            </tbody>
          </table>
        </div>

        <div class="code">
          <h3>Usage</h3>
          ${this.renderCodeExample(component)}
        </div>
      </div>
    `;
  }

  connectedCallback() {
    super.connectedCallback();
    if (this.loadingComponents.length > 0) {
      this.selectedComponent = this.loadingComponents[0].tag;
    }
  }

  render() {
    return html`
      <div class="page">
        <div class="header">
          <h1>Components</h1>
          <p>Reusable UI components for the web app</p>
        </div>

        <div class="tabs">
          <button
            class="tab ${this.activeTab === "loading" ? "active" : ""}"
            @click=${() => {
              this.activeTab = "loading";
              this.selectedComponent = this.loadingComponents[0]?.tag || null;
            }}
          >
            Loading Indicators
          </button>
          <button class="tab disabled" disabled>Buttons</button>
          <button class="tab disabled" disabled>Inputs</button>
          <button class="tab disabled" disabled>Feedback</button>
        </div>

        <div class="content">
          ${this.renderSidebar()}
          ${this.renderMain()}
        </div>
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "components-page": ComponentsPage;
  }
}
