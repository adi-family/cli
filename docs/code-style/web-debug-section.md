# Web Debug Section

Register debug sections in the debug screen plugin to display runtime diagnostics for plugins or app-level subsystems.

## Pattern

1. **Create a LitElement** that renders debug info
2. **Register it** via `AdiDebugScreenBusKey.RegisterSection` on the event bus
3. **Sync data** into the element reactively or on a timer

## From a Web Plugin

Plugins register their debug section in `onRegister()`:

```ts
// debug-section.ts
import { LitElement, html } from 'lit';
import { customElement, state } from 'lit/decorators.js';

@customElement('adi-my-plugin-debug')
export class MyPluginDebugElement extends LitElement {
  @state() items: string[] = [];

  override createRenderRoot() { return this; }

  override render() {
    return html`
      <div style="display:flex;flex-direction:column;gap:1rem">
        <section>
          <div class="text-xs uppercase" style="color:var(--adi-text-muted);font-weight:600;margin-bottom:0.5rem">
            Items (${this.items.length})
          </div>
          <table class="dr-table text-sm">
            <thead><tr><th class="dr-th">Name</th></tr></thead>
            <tbody>
              ${this.items.map((i) => html`<tr class="dr-row"><td class="dr-td">${i}</td></tr>`)}
            </tbody>
          </table>
        </section>
      </div>
    `;
  }
}
```

```ts
// plugin.ts
import { AdiDebugScreenBusKey } from '@adi/debug-screen-web-plugin/bus';
import type { MyPluginDebugElement } from './debug-section.js';

export class MyPlugin extends AdiPlugin {
  private debugEl: MyPluginDebugElement | null = null;

  override async onRegister(): Promise<void> {
    await import('./debug-section.js');
    this.bus.emit(
      AdiDebugScreenBusKey.RegisterSection,
      {
        pluginId: PLUGIN_ID,
        init: () => {
          this.debugEl = document.createElement('adi-my-plugin-debug') as MyPluginDebugElement;
          this.syncDebug();
          return this.debugEl;
        },
        label: 'My Plugin',
      },
      PLUGIN_ID,
    );
  }

  private syncDebug(): void {
    if (!this.debugEl) return;
    this.debugEl.items = this.getItems();
  }
}
```

Key points:
- `pluginId` is used as the tab/section identifier in the debug screen
- `init` is a factory called lazily when the user opens the section
- Call `syncDebug()` whenever state changes to push data into the element

## From App-Level Code (Non-Plugin)

For subsystems that aren't plugins (e.g., RegistryHub), create a sync factory and emit directly on the bus:

```ts
// subsystem-debug.ts
export function createSubsystemDebugSync(subsystem: Subsystem): {
  init: () => HTMLElement;
  dispose: () => void;
} {
  let el: MyDebugElement | null = null;
  let timer: ReturnType<typeof setInterval> | null = null;

  function sync() {
    if (!el) return;
    el.data = subsystem.getData();
  }

  return {
    init: () => {
      el = document.createElement('adi-subsystem-debug') as MyDebugElement;
      sync();
      timer = setInterval(sync, 2_000);
      return el;
    },
    dispose: () => {
      if (timer) clearInterval(timer);
      el = null;
    },
  };
}
```

```ts
// app.ts
import { AdiDebugScreenBusKey } from '@adi/debug-screen-web-plugin/bus';
import { createSubsystemDebugSync } from './subsystem-debug';

const debugSync = createSubsystemDebugSync(subsystem);
bus.emit(
  AdiDebugScreenBusKey.RegisterSection,
  { pluginId: 'app.subsystem', init: debugSync.init, label: 'Subsystem' },
  'app',
);
```

Key difference: since there's no plugin lifecycle to hook into, use `setInterval` for periodic sync instead of pushing on every state change.

## Styling Conventions

- Use `createRenderRoot() { return this; }` to inherit host app styles (no Shadow DOM)
- Use CSS classes: `dr-table`, `dr-th`, `dr-td`, `dr-row` for debug tables
- Use `text-xs`, `text-sm` for text sizing
- Use `var(--adi-text-muted)` for labels, `var(--adi-accent)` for active indicators
- Use `var(--adi-error, #ef4444)` for error states

## Dependencies

The debug-screen bus must be importable. Add to `package.json`:

```json
"@adi/debug-screen-web-plugin": "file:../../crates/debug-screen/web"
```

Import bus keys from the `/bus` subpath to avoid pulling in UI deps:

```ts
import { AdiDebugScreenBusKey } from '@adi/debug-screen-web-plugin/bus';
```
