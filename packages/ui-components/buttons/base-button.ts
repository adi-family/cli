import { LitElement } from "lit";
import { property, state } from "lit/decorators.js";

export type AsyncClickHandler = (e: MouseEvent) => Promise<void> | void;

/// Abstract base class with async onClick support and loading state management.
/// Sizing is inherited from the ADID AX system (--l, --t, --r variables).
/// Wrap in .compact, .dense, or .spacious to change size contextually.
export abstract class BaseButton extends LitElement {
  @property({ type: String }) label = "Button";
  @property({ type: String }) loadingText = "";
  @property({ type: Boolean }) disabled = false;
  @property({ type: Boolean }) loading = false;
  @property({ attribute: false }) onClick?: AsyncClickHandler;

  @state() protected _internalLoading = false;

  createRenderRoot() {
    return this;
  }

  protected get isLoading(): boolean {
    return this.loading || this._internalLoading;
  }

  protected get isDisabled(): boolean {
    return this.disabled || this.isLoading;
  }

  protected get displayText(): string {
    return this.isLoading && this.loadingText ? this.loadingText : this.label;
  }

  protected async handleClick(e: MouseEvent) {
    if (this.isDisabled || !this.onClick) return;

    this._internalLoading = true;
    try {
      await this.onClick(e);
    } finally {
      this._internalLoading = false;
    }
  }
}
