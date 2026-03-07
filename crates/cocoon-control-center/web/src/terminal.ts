import { LitElement, html, nothing } from 'lit';
import { state } from 'lit/decorators.js';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import type { CocoonClient } from './cocoon-client';
import type { SilkSession } from './silk-session';
import type { SilkCommand } from './silk-command';

type TerminalStatus = 'idle' | 'connecting' | 'ready' | 'closed' | 'error';
type CommandMode = 'prompt' | 'running' | 'interactive';

export class AdiCocoonTerminalElement extends LitElement {
  private _client: CocoonClient | null = null;
  private session: SilkSession | null = null;
  private command: SilkCommand | null = null;
  private xterm: Terminal | null = null;
  private fitAddon: FitAddon | null = null;
  private ro: ResizeObserver | null = null;
  private containerEl: HTMLDivElement | null = null;
  private cleanups: (() => void)[] = [];

  @state() private status: TerminalStatus = 'idle';
  @state() private errorMsg = '';
  @state() private mode: CommandMode = 'prompt';
  @state() private outputLines: string[] = [];
  @state() private commandHistory: { command: string; output: string[]; exitCode?: number }[] = [];

  override createRenderRoot() { return this; }

  set client(c: CocoonClient | null) {
    if (this._client === c) return;
    this.teardown();
    this._client = c;
    if (c) this.startSession(c);
  }

  get client(): CocoonClient | null {
    return this._client;
  }

  private async startSession(client: CocoonClient): Promise<void> {
    this.status = 'connecting';
    try {
      this.session = await client.createSession();
      this.status = 'ready';
      this.mode = 'prompt';
      this.requestUpdate();
      this.updateComplete.then(() => this.focusInput());
    } catch (e) {
      this.status = 'error';
      this.errorMsg = e instanceof Error ? e.message : String(e);
    }
  }

  private focusInput(): void {
    const input = this.querySelector<HTMLInputElement>('#cmd-input');
    input?.focus();
  }

  private submitCommand(e: Event): void {
    e.preventDefault();
    const input = this.querySelector<HTMLInputElement>('#cmd-input');
    if (!input || !this.session) return;

    const cmd = input.value.trim();
    if (!cmd) return;

    input.value = '';
    this.outputLines = [];
    this.mode = 'running';
    this.executeCommand(cmd);
  }

  private executeCommand(cmd: string): void {
    if (!this.session) return;

    const termHost = this.querySelector<HTMLDivElement>('#xterm-host');
    const cols = termHost ? Math.floor(termHost.clientWidth / 8) : 80;
    const rows = termHost ? Math.floor(termHost.clientHeight / 16) : 24;

    this.command = this.session.execute(cmd, { cols, rows });

    this.cleanups.push(
      this.command.onOutput(({ data }) => {
        this.outputLines = [...this.outputLines, ...data.split('\n')];
      }),
    );

    this.cleanups.push(
      this.command.onInteractiveRequired(() => {
        this.mode = 'interactive';
        this.requestUpdate();
        this.updateComplete.then(() => this.mountXterm());
      }),
    );

    this.cleanups.push(
      this.command.onPtyOutput(({ data }) => {
        this.xterm?.write(data);
      }),
    );

    this.cleanups.push(
      this.command.onCompleted(({ exitCode }) => {
        this.commandHistory = [
          ...this.commandHistory,
          { command: cmd, output: this.outputLines, exitCode },
        ];
        this.disposeCommand();
        this.mode = 'prompt';
        this.requestUpdate();
        this.updateComplete.then(() => this.focusInput());
      }),
    );

    this.cleanups.push(
      this.command.onError(({ message }) => {
        this.outputLines = [...this.outputLines, `Error: ${message}`];
        this.commandHistory = [
          ...this.commandHistory,
          { command: cmd, output: this.outputLines, exitCode: -1 },
        ];
        this.disposeCommand();
        this.mode = 'prompt';
        this.requestUpdate();
        this.updateComplete.then(() => this.focusInput());
      }),
    );
  }

  private mountXterm(): void {
    const host = this.querySelector<HTMLDivElement>('#xterm-host');
    if (!host || !this.command || this.xterm) return;

    this.xterm = new Terminal({
      cursorBlink: true,
      fontFamily: '"Cascadia Code", "Fira Code", Consolas, "Courier New", monospace',
      fontSize: 14,
      lineHeight: 1.2,
      theme: {
        background: '#0d0d0d',
        foreground: '#d4d4d4',
        cursor: '#d4d4d4',
        selectionBackground: '#264f78',
        black: '#1e1e1e',
        brightBlack: '#555555',
        red: '#f44747',
        brightRed: '#f44747',
        green: '#6a9955',
        brightGreen: '#6a9955',
        yellow: '#dcdcaa',
        brightYellow: '#dcdcaa',
        blue: '#569cd6',
        brightBlue: '#569cd6',
        magenta: '#c586c0',
        brightMagenta: '#c586c0',
        cyan: '#4ec9b0',
        brightCyan: '#4ec9b0',
        white: '#d4d4d4',
        brightWhite: '#ffffff',
      },
    });

    this.fitAddon = new FitAddon();
    this.xterm.loadAddon(this.fitAddon);
    this.xterm.open(host);

    requestAnimationFrame(() => {
      this.fitAddon?.fit();
      if (this.xterm && this.command) {
        const { cols, rows } = this.xterm;
        this.command.resize(cols, rows);
      }
    });

    this.xterm.onData((data) => this.command?.input(data));
    this.xterm.onResize(({ cols, rows }) => this.command?.resize(cols, rows));

    this.ro = new ResizeObserver(() => this.fitAddon?.fit());
    this.ro.observe(host);
  }

  private disposeCommand(): void {
    for (const cleanup of this.cleanups) cleanup();
    this.cleanups = [];
    this.ro?.disconnect();
    this.ro = null;
    this.xterm?.dispose();
    this.xterm = null;
    this.fitAddon = null;
    this.command?.dispose();
    this.command = null;
  }

  private teardown(): void {
    this.disposeCommand();
    this.session?.close();
    this.session = null;
    this._client = null;
    this.containerEl = null;
    this.status = 'idle';
    this.errorMsg = '';
    this.mode = 'prompt';
    this.outputLines = [];
    this.commandHistory = [];
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.teardown();
  }

  restart(): void {
    const client = this._client;
    this.teardown();
    this._client = client;
    if (client) this.startSession(client);
  }

  override render() {
    if (this.status === 'idle') return nothing;

    if (this.status === 'connecting') {
      return html`
        <div style="width:100%;height:100%;display:flex;align-items:center;justify-content:center;background:#0d0d0d;">
          <div style="display:flex;flex-direction:column;align-items:center;gap:12px;">
            <div style="width:24px;height:24px;border:2px solid #444;border-top-color:#6366f1;border-radius:50%;animation:spin 0.8s linear infinite;"></div>
            <span style="color:#888;font-size:13px;">Starting session...</span>
          </div>
          <style>@keyframes spin{from{transform:rotate(0deg)}to{transform:rotate(360deg)}}</style>
        </div>
      `;
    }

    if (this.status === 'error') {
      return html`
        <div style="width:100%;height:100%;display:flex;align-items:center;justify-content:center;background:#0d0d0d;">
          <div style="display:flex;flex-direction:column;align-items:center;gap:12px;max-width:400px;text-align:center;">
            <span style="color:#f87171;font-size:28px;">&#x2715;</span>
            <span style="color:#f87171;font-size:14px;">${this.errorMsg || 'Session error'}</span>
            <button
              style="padding:6px 16px;border:1px solid #444;border-radius:6px;background:transparent;color:#d4d4d4;font-size:13px;cursor:pointer;"
              @click=${() => this.restart()}
            >Retry</button>
          </div>
        </div>
      `;
    }

    if (this.status === 'closed') {
      return html`
        <div style="width:100%;height:100%;display:flex;align-items:center;justify-content:center;background:#0d0d0d;">
          <div style="display:flex;flex-direction:column;align-items:center;gap:12px;">
            <span style="color:#888;font-size:14px;">Session closed.</span>
            <button
              style="padding:6px 16px;border:1px solid #444;border-radius:6px;background:transparent;color:#d4d4d4;font-size:13px;cursor:pointer;"
              @click=${() => this.restart()}
            >New Session</button>
          </div>
        </div>
      `;
    }

    if (this.mode === 'interactive') {
      return html`
        <div id="xterm-host" style="width:100%;height:100%;overflow:hidden;background:#0d0d0d;"></div>
      `;
    }

    return html`
      <div style="width:100%;height:100%;display:flex;flex-direction:column;background:#0d0d0d;font-family:'Cascadia Code','Fira Code',Consolas,'Courier New',monospace;font-size:14px;color:#d4d4d4;overflow:hidden;">
        <div style="flex:1;overflow-y:auto;padding:12px 16px;">
          ${this.commandHistory.map(
            (entry) => html`
              <div style="margin-bottom:8px;">
                <div style="color:#6366f1;">$ ${entry.command}</div>
                ${entry.output.map((line) => html`<div style="white-space:pre-wrap;color:#d4d4d4;">${line}</div>`)}
                ${entry.exitCode != null && entry.exitCode !== 0
                  ? html`<div style="color:#f87171;font-size:12px;">exit ${entry.exitCode}</div>`
                  : nothing}
              </div>
            `,
          )}
          ${this.mode === 'running'
            ? html`
                <div style="margin-bottom:8px;">
                  <div style="color:#6366f1;">$ <span style="opacity:0.6;">running...</span></div>
                  ${this.outputLines.map((line) => html`<div style="white-space:pre-wrap;color:#d4d4d4;">${line}</div>`)}
                </div>
              `
            : nothing}
        </div>
        ${this.mode === 'prompt'
          ? html`
              <form @submit=${this.submitCommand} style="display:flex;align-items:center;padding:8px 16px;border-top:1px solid #333;">
                <span style="color:#6366f1;margin-right:8px;">$</span>
                <input
                  id="cmd-input"
                  type="text"
                  autocomplete="off"
                  spellcheck="false"
                  style="flex:1;background:transparent;border:none;outline:none;color:#d4d4d4;font-family:inherit;font-size:inherit;"
                  placeholder="Type a command..."
                />
              </form>
            `
          : nothing}
      </div>
    `;
  }
}
