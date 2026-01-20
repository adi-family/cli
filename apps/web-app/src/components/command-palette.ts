import { LitElement, html } from "lit";
import { customElement, property, state, query } from "lit/decorators.js";
import { unsafeSVG } from "lit/directives/unsafe-svg.js";
import { createElement, Search, CornerDownLeft, ArrowUp, ArrowDown } from "lucide";

const icon = (iconData: typeof Search) =>
  unsafeSVG(createElement(iconData).outerHTML);

export interface CommandItem {
  id: string;
  label: string;
  description?: string;
  shortcut?: string[];
  icon?: typeof Search;
  category?: string;
  action: () => void;
}

@customElement("command-palette")
export class CommandPalette extends LitElement {
  @property({ type: Boolean, reflect: true })
  open = false;

  @property({ type: Array })
  commands: CommandItem[] = [];

  @state()
  private searchQuery = "";

  @state()
  private selectedIndex = 0;

  @query(".command-palette__input")
  private inputElement!: HTMLInputElement;

  private boundKeydownHandler = this.handleGlobalKeydown.bind(this);

  createRenderRoot() {
    return this;
  }

  connectedCallback() {
    super.connectedCallback();
    document.addEventListener("keydown", this.boundKeydownHandler);
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    document.removeEventListener("keydown", this.boundKeydownHandler);
  }

  private handleGlobalKeydown(e: KeyboardEvent) {
    // Cmd+K or Ctrl+K to open
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      e.preventDefault();
      this.toggle();
      return;
    }

    if (!this.open) return;

    // Escape to close
    if (e.key === "Escape") {
      e.preventDefault();
      this.close();
      return;
    }

    // Arrow navigation
    if (e.key === "ArrowDown") {
      e.preventDefault();
      this.selectedIndex = Math.min(
        this.selectedIndex + 1,
        this.filteredCommands.length - 1
      );
      this.scrollSelectedIntoView();
      return;
    }

    if (e.key === "ArrowUp") {
      e.preventDefault();
      this.selectedIndex = Math.max(this.selectedIndex - 1, 0);
      this.scrollSelectedIntoView();
      return;
    }

    // Enter to execute
    if (e.key === "Enter") {
      e.preventDefault();
      this.executeSelected();
      return;
    }
  }

  private scrollSelectedIntoView() {
    requestAnimationFrame(() => {
      const selected = this.querySelector(".command-palette__item--selected");
      selected?.scrollIntoView({ block: "nearest" });
    });
  }

  private get filteredCommands(): CommandItem[] {
    if (!this.searchQuery.trim()) {
      return this.commands;
    }

    const query = this.searchQuery.toLowerCase();
    return this.commands.filter(
      (cmd) =>
        cmd.label.toLowerCase().includes(query) ||
        cmd.description?.toLowerCase().includes(query) ||
        cmd.category?.toLowerCase().includes(query)
    );
  }

  private get groupedCommands(): Map<string, CommandItem[]> {
    const groups = new Map<string, CommandItem[]>();
    
    for (const cmd of this.filteredCommands) {
      const category = cmd.category || "General";
      if (!groups.has(category)) {
        groups.set(category, []);
      }
      groups.get(category)!.push(cmd);
    }
    
    return groups;
  }

  toggle() {
    if (this.open) {
      this.close();
    } else {
      this.openPalette();
    }
  }

  openPalette() {
    this.open = true;
    this.searchQuery = "";
    this.selectedIndex = 0;
    
    requestAnimationFrame(() => {
      this.inputElement?.focus();
    });

    this.dispatchEvent(
      new CustomEvent("command-palette-open", {
        bubbles: true,
        composed: true,
      })
    );
  }

  close() {
    this.open = false;
    this.dispatchEvent(
      new CustomEvent("command-palette-close", {
        bubbles: true,
        composed: true,
      })
    );
  }

  private executeSelected() {
    const cmd = this.filteredCommands[this.selectedIndex];
    if (cmd) {
      this.close();
      cmd.action();
      
      this.dispatchEvent(
        new CustomEvent("command-execute", {
          detail: { command: cmd },
          bubbles: true,
          composed: true,
        })
      );
    }
  }

  private handleItemClick(index: number) {
    this.selectedIndex = index;
    this.executeSelected();
  }

  private handleItemMouseEnter(index: number) {
    this.selectedIndex = index;
  }

  private handleSearchInput(e: Event) {
    const input = e.target as HTMLInputElement;
    this.searchQuery = input.value;
    this.selectedIndex = 0;
  }

  private handleBackdropClick(e: Event) {
    if ((e.target as HTMLElement).classList.contains("command-palette__backdrop")) {
      this.close();
    }
  }

  private renderShortcut(shortcut: string[]) {
    return html`
      <div class="command-palette__shortcut">
        ${shortcut.map(
          (key) => html`<kbd class="command-palette__kbd">${key}</kbd>`
        )}
      </div>
    `;
  }

  private renderItem(cmd: CommandItem, globalIndex: number) {
    const isSelected = globalIndex === this.selectedIndex;
    
    return html`
      <button
        class="command-palette__item ${isSelected ? "command-palette__item--selected" : ""}"
        @click=${() => this.handleItemClick(globalIndex)}
        @mouseenter=${() => this.handleItemMouseEnter(globalIndex)}
      >
        <div class="command-palette__item-left">
          ${cmd.icon ? html`<span class="command-palette__item-icon">${icon(cmd.icon)}</span>` : ""}
          <div class="command-palette__item-content">
            <span class="command-palette__item-label">${cmd.label}</span>
            ${cmd.description
              ? html`<span class="command-palette__item-desc">${cmd.description}</span>`
              : ""}
          </div>
        </div>
        ${cmd.shortcut ? this.renderShortcut(cmd.shortcut) : ""}
      </button>
    `;
  }

  render() {
    if (!this.open) return null;

    let globalIndex = 0;

    return html`
      <div class="command-palette__backdrop" @click=${this.handleBackdropClick}>
        <div class="command-palette">
          <div class="command-palette__header">
            <span class="command-palette__search-icon">${icon(Search)}</span>
            <input
              type="text"
              class="command-palette__input"
              placeholder="Type a command or search..."
              .value=${this.searchQuery}
              @input=${this.handleSearchInput}
              autocomplete="off"
              spellcheck="false"
            />
          </div>
          
          <div class="command-palette__content">
            ${this.filteredCommands.length === 0
              ? html`
                  <div class="command-palette__empty">
                    No commands found for "${this.searchQuery}"
                  </div>
                `
              : html`
                  ${Array.from(this.groupedCommands.entries()).map(
                    ([category, cmds]) => html`
                      <div class="command-palette__group">
                        <div class="command-palette__group-title">${category}</div>
                        ${cmds.map((cmd) => this.renderItem(cmd, globalIndex++))}
                      </div>
                    `
                  )}
                `}
          </div>
          
          <div class="command-palette__footer">
            <div class="command-palette__hint">
              <span class="command-palette__hint-item">
                ${icon(ArrowUp)}${icon(ArrowDown)} Navigate
              </span>
              <span class="command-palette__hint-item">
                ${icon(CornerDownLeft)} Select
              </span>
              <span class="command-palette__hint-item">
                <kbd class="command-palette__kbd command-palette__kbd--small">esc</kbd> Close
              </span>
            </div>
          </div>
        </div>
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "command-palette": CommandPalette;
  }
}
