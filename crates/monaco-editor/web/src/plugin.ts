import { AdiPlugin } from '@adi-family/sdk-plugin';
import type { WithCid } from '@adi-family/sdk-plugin';
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

    this.bus.emit('route:register', { path: '/editor', element: 'adi-monaco-editor' });
    this.bus.send('nav:add', { id: 'editor', label: 'Editor', path: '/editor' }).handle(() => {});

    this.bus.emit('command:register', { id: 'editor:open-page', label: 'Go to Editor' });
    this.bus.on('command:execute', ({ id }) => {
      if (id === 'editor:open-page') {
        this.bus.emit('router:navigate', { path: '/editor' });
      }
    });

    this.bus.on('editor:open', (payload) => {
      const { _cid, content, options } = payload as WithCid<typeof payload>;
      this.bus.emit('router:navigate', { path: '/editor' });

      // Defer content setting to allow the component to mount
      requestAnimationFrame(() => {
        this.bus.emit('editor:set-content', { content });
        if (options) this.bus.emit('editor:set-options', { options });
        this.bus.emit('editor:open:ok', { _cid });
      });
    });

    this.bus.on('app:theme-changed', ({ mode }) => {
      const editorTheme = mode === 'dark' ? 'vs-dark' : 'vs';
      this.bus.emit('editor:set-theme', { theme: editorTheme });
    });
  }
}
