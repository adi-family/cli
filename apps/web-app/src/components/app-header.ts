import { LitElement, html } from "lit";
import { customElement, property } from "lit/decorators.js";
import { unsafeSVG } from "lit/directives/unsafe-svg.js";
import { createElement, LayoutGrid, Search } from "lucide";

const icon = (iconData: typeof LayoutGrid) =>
  unsafeSVG(createElement(iconData).outerHTML);

@customElement("app-header")
export class AppHeader extends LitElement {
  @property({ type: String })
  activeNav = "board";

  @property({ type: String })
  title = "Kanban Execution";

  @property({ type: String })
  userAvatar = "";

  private navItems = [
    { id: "board", label: "Board" },
    { id: "credentials", label: "Credentials" },
  ];

  createRenderRoot() {
    return this;
  }

  private handleNavClick(navId: string) {
    this.activeNav = navId;
    this.dispatchEvent(
      new CustomEvent("nav-change", {
        detail: { navId },
        bubbles: true,
        composed: true,
      }),
    );
  }

  private handleSearch(e: Event) {
    const input = e.target as HTMLInputElement;
    this.dispatchEvent(
      new CustomEvent("search", {
        detail: { query: input.value },
        bubbles: true,
        composed: true,
      }),
    );
  }

  private handleProfileClick() {
    this.dispatchEvent(
      new CustomEvent("profile-click", {
        bubbles: true,
        composed: true,
      }),
    );
  }

  private getNavLinkClasses(isActive: boolean) {
    const base =
      "header-nav-item text-sm font-medium no-underline h-full flex items-center px-3 border-b-0 transition-all duration-300 hover:text-accent-400";
    return isActive
      ? `${base} header-nav-item--active text-accent-400`
      : `${base} text-text-secondary`;
  }

  render() {
    return html`
      <header
        class="header-animated flex items-center justify-between gap-4 h-16 shrink-0 z-50"
      >
        <div class="flex items-center gap-4 h-full">
          <div class="flex items-center text-accent-500 px-3 h-full">
            <div
              class="header-logo-box w-9 h-9 bg-accent-600/20 rounded-lg flex items-center justify-center cursor-pointer"
            >
              ${icon(LayoutGrid)}
            </div>
          </div>
          <nav class="hidden md:flex items-center gap-2 h-full">
            ${this.navItems.map(
              (item) => html`
                <a
                  href="#"
                  class=${this.getNavLinkClasses(this.activeNav === item.id)}
                  @click=${(e: Event) => {
                    e.preventDefault();
                    this.handleNavClick(item.id);
                  }}
                >
                  ${item.label}
                </a>
              `,
            )}
          </nav>
        </div>
        <div class="flex items-center gap-4 h-full">
          <div
            class="header-search relative hidden lg:flex items-center w-64 h-full px-2"
          >
            <span
              class="absolute left-5 top-1/2 -translate-y-1/2 text-text-secondary text-base transition-colors duration-200"
              >${icon(Search)}</span
            >
            <input
              type="text"
              class="w-full bg-dark-700/80 border border-dark-600 rounded-lg py-2 pr-3 pl-10 text-sm text-text-body outline-none transition-all duration-300 placeholder:text-text-secondary focus:border-accent-500 focus:ring-2 focus:ring-accent-500/30 focus:bg-dark-700"
              placeholder="Quick search (Cmd+K)"
              @input=${this.handleSearch}
            />
          </div>

          <button
            class="header-avatar-btn flex items-center justify-center h-full px-4 border-none bg-transparent cursor-pointer transition-all duration-300 hover:bg-dark-700/80"
            @click=${this.handleProfileClick}
          >
            <div
              class="header-avatar w-9 h-9 rounded-full bg-gradient-to-br from-accent-600 via-accent-500 to-violet-glow bg-cover bg-center border-2 border-dark-600/50"
            ></div>
          </button>
        </div>
      </header>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "app-header": AppHeader;
  }
}
