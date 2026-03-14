import { AdiPlugin } from '@adi-family/sdk-plugin';
import { SlotsBusKey } from '@adi-family/plugin-slots';
import { ActionsBusKey } from './generated/bus-types';
import type { ActionCard, RenderFn, KindMode } from './types.js';
import './generated/bus';

const PLUGIN_ID = 'adi.actions-feed';
const kindKey = (plugin: string, kind: string) => `${plugin}::${kind}`;

const store = {
  bus: null as import('@adi-family/sdk-plugin').EventBus | null,
  actions: [] as ActionCard[],
  renderers: new Map<string, RenderFn>(),
  kindModes: new Map<string, KindMode>(),
  listeners: new Set<() => void>(),
  notify() {
    for (const fn of this.listeners) fn();
  },
};

export { store as actionStore };

export class ActionsFeedPlugin extends AdiPlugin {
  readonly id = PLUGIN_ID;
  readonly version = '0.1.0';

  override async onRegister(): Promise<void> {
    store.bus = this.bus;
    const { AdiActionsFeedElement } = await import('./component.js');
    if (!customElements.get('adi-actions-feed')) {
      customElements.define('adi-actions-feed', AdiActionsFeedElement);
    }

    this.bus.emit(SlotsBusKey.Place, {
      slot: 'right',
      elementRef: document.createElement('adi-actions-feed'),
      priority: 0,
      pluginId: PLUGIN_ID,
    }, PLUGIN_ID);

    this.bus.on(ActionsBusKey.RegisterKind, ({ plugin, kind, mode }) => {
      store.kindModes.set(kindKey(plugin, kind), mode as KindMode);
    }, PLUGIN_ID);

    this.bus.on(ActionsBusKey.RegisterRenderer, ({ plugin, kind, render }) => {
      store.renderers.set(kindKey(plugin, kind), render as RenderFn);
    }, PLUGIN_ID);

    this.bus.on(ActionsBusKey.Push, ({ id, plugin, kind, data, priority }) => {
      const key = kindKey(plugin, kind);
      const mode = store.kindModes.get(key);

      if (mode === 'exclusive') {
        const dismissed = store.actions.filter(
          (a) => a.plugin === plugin && a.kind === kind && a.id !== id,
        );
        if (dismissed.length > 0) {
          store.actions = store.actions.filter(
            (a) => !(a.plugin === plugin && a.kind === kind && a.id !== id),
          );
          for (const d of dismissed) {
            this.bus.emit(ActionsBusKey.Dismissed, { id: d.id, plugin: d.plugin, kind: d.kind }, PLUGIN_ID);
          }
        }
      }

      const card: ActionCard = { id, plugin, kind, data, priority: priority ?? 'normal' };
      const idx = store.actions.findIndex((a) => a.id === id);
      store.actions = idx >= 0
        ? store.actions.map((a, i) => (i === idx ? card : a))
        : [...store.actions, card];
      store.notify();
    }, PLUGIN_ID);

    this.bus.on(ActionsBusKey.Dismiss, ({ id }) => {
      const card = store.actions.find((a) => a.id === id);
      if (!card) return;
      store.actions = store.actions.filter((a) => a.id !== id);
      this.bus.emit(ActionsBusKey.Dismissed, { id: card.id, plugin: card.plugin, kind: card.kind }, PLUGIN_ID);
      store.notify();
    }, PLUGIN_ID);
  }
}
