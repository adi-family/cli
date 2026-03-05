import { AdiPlugin } from '@adi-family/sdk-plugin';
import { AdiRouterBusKey } from '@adi/router-web-plugin/bus';
import { setupWorkers } from './workers.js';
import './events.js';

export class MonacoEditorPlugin extends AdiPlugin {
  readonly id = 'adi.monaco-editor';
  readonly version = '0.1.0';

  async onRegister(): Promise<void> {
    setupWorkers();

    const { AdiMonacoEditorElement } = await import('./component.js');
    if (!customElements.get('adi-monaco-editor')) {
      customElements.define('adi-monaco-editor', AdiMonacoEditorElement);
    }

    this.bus.emit(AdiRouterBusKey.RegisterRoute, { pluginId: this.id, path: '', init: () => document.createElement('adi-monaco-editor'), label: 'Editor' });
    this.bus.emit('nav:add', { id: this.id, label: 'Editor', path: `/${this.id}` });

    this.bus.on('editor:open', ({ content, options }) => {
      this.bus.emit(AdiRouterBusKey.Navigate, { path: `/${this.id}` });

      // Defer content setting to allow the component to mount
      requestAnimationFrame(() => {
        this.bus.emit('editor:set-content', { content });
        if (options) this.bus.emit('editor:set-options', { options });
      });
    });

    this.bus.on('app:theme-changed', ({ mode }) => {
      const editorTheme = mode === 'dark' ? 'vs-dark' : 'vs';
      this.bus.emit('editor:set-theme', { theme: editorTheme });
    });
  }
}
