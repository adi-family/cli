import { LitElement, html } from 'lit'
import { customElement, property } from 'lit/decorators.js'
import { unsafeSVG } from 'lit/directives/unsafe-svg.js'
import { createElement, LayoutGrid, Search, Bell } from 'lucide'

const icon = (iconData: typeof LayoutGrid) => unsafeSVG(createElement(iconData).outerHTML)

@customElement('app-header')
export class AppHeader extends LitElement {
  @property({ type: String })
  activeNav = 'board'

  @property({ type: String })
  title = 'Kanban Execution'

  @property({ type: String })
  userAvatar = ''

  private navItems = [
    { id: 'board', label: 'Board' },
    { id: 'list', label: 'List View' },
    { id: 'timeline', label: 'Timeline' },
    { id: 'insights', label: 'Insights' },
  ]

  createRenderRoot() {
    return this
  }

  private handleNavClick(navId: string) {
    this.activeNav = navId
    this.dispatchEvent(new CustomEvent('nav-change', {
      detail: { navId },
      bubbles: true,
      composed: true
    }))
  }

  private handleSearch(e: Event) {
    const input = e.target as HTMLInputElement
    this.dispatchEvent(new CustomEvent('search', {
      detail: { query: input.value },
      bubbles: true,
      composed: true
    }))
  }

  private handleNotificationClick() {
    this.dispatchEvent(new CustomEvent('notification-click', {
      bubbles: true,
      composed: true
    }))
  }

  private handleProfileClick() {
    this.dispatchEvent(new CustomEvent('profile-click', {
      bubbles: true,
      composed: true
    }))
  }

  private getNavLinkClasses(isActive: boolean) {
    const base = 'text-sm font-medium no-underline pb-5 mt-5 border-b-2 transition-colors duration-150 hover:text-accent-500'
    return isActive
      ? `${base} text-accent-500 font-semibold border-b-accent-600`
      : `${base} text-text-secondary border-transparent`
  }

  render() {
    return html`
      <header class="flex items-center justify-between border-b border-dark-700 bg-dark-800 px-6 h-16 shrink-0 z-50">
        <div class="flex items-center gap-6">
          <div class="flex items-center gap-3 text-accent-500">
            <div class="w-8 h-8 bg-accent-600/15 rounded-md flex items-center justify-center">
              ${icon(LayoutGrid)}
            </div>
            <h2 class="text-text-heading text-lg font-bold tracking-tight m-0">${this.title}</h2>
          </div>
          <nav class="hidden md:flex items-center gap-6 ml-4">
            ${this.navItems.map(item => html`
              <a
                href="#"
                class=${this.getNavLinkClasses(this.activeNav === item.id)}
                @click=${(e: Event) => {
                  e.preventDefault()
                  this.handleNavClick(item.id)
                }}
              >
                ${item.label}
              </a>
            `)}
          </nav>
        </div>
        <div class="flex items-center gap-4">
          <div class="relative hidden lg:block w-64">
            <span class="absolute left-3 top-1/2 -translate-y-1/2 text-text-secondary text-base">${icon(Search)}</span>
            <input
              type="text"
              class="w-full bg-dark-700 border border-dark-600 rounded-lg py-2 pr-3 pl-10 text-sm text-text-body outline-none transition-all duration-150 placeholder:text-text-secondary focus:border-accent-600 focus:ring-2 focus:ring-accent-600/20"
              placeholder="Quick search (Cmd+K)"
              @input=${this.handleSearch}
            />
          </div>
          <button
            class="flex items-center justify-center w-10 h-10 rounded-lg border-none bg-transparent text-text-secondary cursor-pointer transition-all duration-150 hover:bg-dark-700 hover:text-accent-500"
            @click=${this.handleNotificationClick}
          >
            ${icon(Bell)}
          </button>
          <div class="h-8 w-px bg-dark-700 mx-2"></div>
          <button
            class="w-9 h-9 rounded-full bg-gradient-to-br from-accent-600 to-accent-500 bg-cover bg-center border-2 border-dark-600 cursor-pointer p-0 transition-colors duration-150 hover:border-accent-600"
            @click=${this.handleProfileClick}
          ></button>
        </div>
      </header>
    `
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'app-header': AppHeader
  }
}
