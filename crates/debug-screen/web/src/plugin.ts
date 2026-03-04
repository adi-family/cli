import { AdiPlugin } from '@adi-family/sdk-plugin';
import './generated/bus';

const PLUGIN_ID = 'app.debug-screen';
const DEBUG_ROUTE = '/debug';

export class DebugScreenPlugin extends AdiPlugin {
  readonly id = PLUGIN_ID;
  readonly version = '0.1.0';

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
