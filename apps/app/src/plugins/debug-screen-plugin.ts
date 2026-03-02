import { AdiPlugin } from '@adi-family/sdk-plugin';
import type { AppContext } from '../app/app';

const PLUGIN_ID = 'app.debug-screen';
const DEBUG_ROUTE = '/debug';

export class DebugScreenPlugin extends AdiPlugin {
  readonly id = PLUGIN_ID;
  readonly version = '1.0.0';

  private constructor() {
    super();
  }

  static init(_ctx: AppContext): DebugScreenPlugin {
    return new DebugScreenPlugin();
  }

  override onRegister(): void {
    queueMicrotask(() => {
      this.bus.emit('command:register', { id: 'app:debug', label: 'Open Debug Screen', shortcut: '⌘⇧D' }, PLUGIN_ID);
      this.bus.emit('command:register', { id: 'app:ops-log', label: 'Toggle Operations Log', shortcut: '⌘⇧O' }, PLUGIN_ID);
    });

    this.bus.on('command:execute', ({ id }) => {
      if (id === 'app:debug') {
        this.bus.emit('router:navigate', { path: DEBUG_ROUTE }, PLUGIN_ID);
      }
    }, PLUGIN_ID);
  }
}
