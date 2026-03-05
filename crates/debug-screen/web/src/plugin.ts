import { AdiPlugin } from '@adi-family/sdk-plugin';
import './generated/bus';

const PLUGIN_ID = 'app.debug-screen';

export class DebugScreenPlugin extends AdiPlugin {
  readonly id = PLUGIN_ID;
  readonly version = '0.1.0';

  override onRegister(): void {
    queueMicrotask(() => {
      this.bus.emit('command:register', { id: 'app:ops-log', label: 'Toggle Operations Log', shortcut: '⌘⇧O' }, PLUGIN_ID);
    });
  }
}
