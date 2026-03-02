import { LitElement, html } from 'lit';
import { property, state } from 'lit/decorators.js';
import * as monaco from 'monaco-editor';
import type { EditorOptions } from './types.js';

export class AdiMonacoEditorElement extends LitElement {
  @property({ type: String }) content = '';
  @property({ type: String }) language = 'plaintext';
  @property({ type: String }) editorTheme = 'vs-dark';
  @property({ type: Boolean }) readOnly = false;

  @state() private editor: monaco.editor.IStandaloneCodeEditor | null = null;

  private container: HTMLDivElement | null = null;
  private resizeObserver: ResizeObserver | null = null;
  private changeDebounce: ReturnType<typeof setTimeout> | null = null;
  private unsubs: Array<() => void> = [];
  private suppressChange = false;

  override createRenderRoot() {
    return this;
  }

  private get bus() {
    return window.sdk.bus;
  }

  override render() {
    return html`<div id="monaco-container" style="width:100%;height:100%;min-height:300px;"></div>`;
  }

  override firstUpdated(): void {
    this.container = this.querySelector('#monaco-container') as HTMLDivElement;
    if (!this.container) return;

    this.editor = monaco.editor.create(this.container, {
      value: this.content,
      language: this.language,
      theme: this.editorTheme,
      readOnly: this.readOnly,
      minimap: { enabled: true },
      automaticLayout: false,
    });

    this.editor.onDidChangeModelContent(() => {
      if (this.suppressChange) return;
      if (this.changeDebounce) clearTimeout(this.changeDebounce);
      this.changeDebounce = setTimeout(() => {
        const content = this.editor?.getValue() ?? '';
        this.bus.emit('editor:changed', { content });
      }, 300);
    });

    this.resizeObserver = new ResizeObserver(() => this.editor?.layout());
    this.resizeObserver.observe(this.container);

    this.registerBusListeners();
  }

  private registerBusListeners(): void {
    this.unsubs.push(
      this.bus.on('editor:set-content', ({ content }) => {
        this.setContent(content);
      }),

      this.bus.on('editor:set-options', ({ options }) => {
        this.applyOptions(options);
      }),

      this.bus.on('editor:set-theme', ({ theme }) => {
        monaco.editor.setTheme(theme);
      }),

      this.bus.on('editor:get-content', () => {
        const content = this.editor?.getValue() ?? '';
        this.bus.emit('editor:changed', { content });
      }),
    );
  }

  private setContent(content: string): void {
    if (!this.editor) return;
    this.suppressChange = true;
    this.editor.setValue(content);
    this.suppressChange = false;
  }

  private applyOptions(options: EditorOptions): void {
    if (!this.editor) return;

    const updateOptions: monaco.editor.IStandaloneEditorConstructionOptions = {};

    if (options.readOnly !== undefined) updateOptions.readOnly = options.readOnly;
    if (options.minimap !== undefined) updateOptions.minimap = { enabled: options.minimap };
    if (options.lineNumbers !== undefined) updateOptions.lineNumbers = options.lineNumbers;
    if (options.wordWrap !== undefined) updateOptions.wordWrap = options.wordWrap;
    if (options.fontSize !== undefined) updateOptions.fontSize = options.fontSize;
    if (options.tabSize !== undefined) updateOptions.tabSize = options.tabSize;

    this.editor.updateOptions(updateOptions);

    if (options.language) {
      const model = this.editor.getModel();
      if (model) monaco.editor.setModelLanguage(model, options.language);
    }

    if (options.theme) {
      monaco.editor.setTheme(options.theme);
    }
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.unsubs.forEach(fn => fn());
    this.unsubs = [];
    if (this.changeDebounce) clearTimeout(this.changeDebounce);
    this.resizeObserver?.disconnect();
    this.editor?.dispose();
    this.editor = null;
  }
}
