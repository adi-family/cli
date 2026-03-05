import { LitElement, html, nothing } from 'lit';
import { customElement, property, state } from 'lit/decorators.js';
import { AdiPlugin } from '@adi-family/sdk-plugin';
import { SlotsBusKey } from './bus';
import type {
  SlotsDefineEvent,
  SlotsPlaceEvent,
  SlotsRemoveEvent,
  SlotsRemoveAllEvent,
} from './bus';
import { PLUGIN_ID, PLUGIN_VERSION } from './config';

// ── Types ────────────────────────────────────────────────

interface SlotDefinition {
  id: string;
  multiple: boolean;
}

export interface SlotEntry {
  slot: string;
  elementRef: HTMLElement;
  priority: number;
  pluginId: string;
}

// ── <adi-slot> Custom Element ────────────────────────────

@customElement('adi-slot')
export class AdiSlotElement extends LitElement {
  @property({ type: String }) name = '';
  @state() private entries: SlotEntry[] = [];

  private unsubChanged?: () => void;
  private pluginRef?: SlotsPlugin;

  override createRenderRoot() {
    return this;
  }

  override connectedCallback(): void {
    super.connectedCallback();
    this.sync();
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.unsubChanged?.();
    this.unsubChanged = undefined;
  }

  bind(plugin: SlotsPlugin): void {
    this.pluginRef = plugin;
    this.sync();
  }

  override updated(changed: Map<string | number | symbol, unknown>): void {
    if (changed.has('name')) {
      this.unsubChanged?.();
      this.unsubChanged = undefined;
      this.sync();
    }
  }

  override render() {
    if (this.entries.length === 0) return nothing;
    return html`${this.entries.map((entry) => entry.elementRef)}`;
  }

  private sync(): void {
    if (!this.pluginRef || !this.name) return;

    this.entries = this.pluginRef.getContent(this.name);

    if (!this.unsubChanged) {
      this.unsubChanged = this.pluginRef.onChange(this.name, () => {
        this.entries = this.pluginRef!.getContent(this.name);
      });
    }
  }
}

// ── SlotsPlugin ──────────────────────────────────────────

export class SlotsPlugin extends AdiPlugin {
  readonly id = PLUGIN_ID;
  readonly version = PLUGIN_VERSION;

  private readonly slots = new Map<string, SlotDefinition>();
  private readonly entries = new Map<string, SlotEntry[]>();
  private readonly changeListeners = new Map<string, Set<() => void>>();
  private observer?: MutationObserver;

  get api() {
    return this;
  }

  override onRegister(): void {
    this.bus.on(
      SlotsBusKey.Define,
      ({ id, multiple }: SlotsDefineEvent) => {
        if (!this.slots.has(id)) {
          this.slots.set(id, { id, multiple: multiple ?? true });
        }
      },
      PLUGIN_ID,
    );

    this.bus.on(
      SlotsBusKey.Place,
      ({ slot, elementRef, priority, pluginId }: SlotsPlaceEvent) => {
        if (!this.slots.has(slot)) {
          this.slots.set(slot, { id: slot, multiple: true });
        }

        const slotDef = this.slots.get(slot)!;
        const current = this.entries.get(slot) ?? [];

        if (current.some((e) => e.elementRef === elementRef && e.pluginId === pluginId)) {
          return;
        }

        const entry: SlotEntry = {
          slot,
          elementRef,
          priority: priority ?? 0,
          pluginId,
        };

        this.entries.set(
          slot,
          slotDef.multiple ? [...current, entry] : [entry],
        );

        this.notifyChanged(slot);
      },
      PLUGIN_ID,
    );

    this.bus.on(
      SlotsBusKey.Remove,
      ({ slot, elementRef }: SlotsRemoveEvent) => {
        const current = this.entries.get(slot);
        if (!current) return;

        const filtered = current.filter((e) => e.elementRef !== elementRef);
        if (filtered.length === current.length) return;

        this.entries.set(slot, filtered);
        this.notifyChanged(slot);
      },
      PLUGIN_ID,
    );

    this.bus.on(
      SlotsBusKey.RemoveAll,
      ({ pluginId }: SlotsRemoveAllEvent) => {
        const affected = new Set<string>();

        for (const [slotId, items] of this.entries) {
          const filtered = items.filter((e) => e.pluginId !== pluginId);
          if (filtered.length !== items.length) {
            this.entries.set(slotId, filtered);
            affected.add(slotId);
          }
        }

        for (const slotId of affected) {
          this.notifyChanged(slotId);
        }
      },
      PLUGIN_ID,
    );

    this.bindExistingSlots();
  }

  override onUnregister(): void {
    this.observer?.disconnect();
    this.observer = undefined;
    this.slots.clear();
    this.entries.clear();
    this.changeListeners.clear();
  }

  // ── Public API ─────────────────────────────────────────

  getContent(slotId: string): SlotEntry[] {
    const items = this.entries.get(slotId) ?? [];
    return [...items].sort((a, b) => a.priority - b.priority);
  }

  hasContent(slotId: string): boolean {
    return (this.entries.get(slotId)?.length ?? 0) > 0;
  }

  getSlotIds(): string[] {
    return [...this.slots.keys()];
  }

  onChange(slotId: string, callback: () => void): () => void {
    if (!this.changeListeners.has(slotId)) {
      this.changeListeners.set(slotId, new Set());
    }
    this.changeListeners.get(slotId)!.add(callback);
    return () => {
      this.changeListeners.get(slotId)?.delete(callback);
    };
  }

  // ── Internal ───────────────────────────────────────────

  private notifyChanged(slotId: string): void {
    this.bus.emit(SlotsBusKey.Changed, { slot: slotId }, PLUGIN_ID);
    const listeners = this.changeListeners.get(slotId);
    if (listeners) {
      for (const cb of listeners) cb();
    }
  }

  private bindExistingSlots(): void {
    for (const el of document.querySelectorAll('adi-slot')) {
      (el as AdiSlotElement).bind(this);
    }

    this.observer = new MutationObserver((mutations) => {
      for (const m of mutations) {
        for (const node of m.addedNodes) {
          if (!(node instanceof HTMLElement)) continue;

          if (node.tagName === 'ADI-SLOT') {
            (node as AdiSlotElement).bind(this);
          }

          for (const child of node.querySelectorAll('adi-slot')) {
            (child as AdiSlotElement).bind(this);
          }
        }
      }
    });

    this.observer.observe(document.body, { childList: true, subtree: true });
  }
}
