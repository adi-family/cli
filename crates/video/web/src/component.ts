import { LitElement, html, nothing } from 'lit';
import { state } from 'lit/decorators.js';
import type { RenderJob, CompositionEntry } from './types.js';
import './events.js';
import { DEMO_COMPOSITIONS } from './compositions/demo.js';

declare global {
  interface Window {
    sdk: {
      bus: import('@adi-family/sdk-plugin').EventBus;
      getConnections(): Map<string, unknown>;
    };
  }
}

type Tab = 'editor' | 'render-queue';

export class AdiVideoElement extends LitElement {
  @state() private tab: Tab = 'editor';
  @state() private compositions: CompositionEntry[] = DEMO_COMPOSITIONS;
  @state() private selectedId = '';
  @state() private jobs: RenderJob[] = [];
  @state() private loading = false;
  @state() private renderProgress: number | null = null;
  @state() private renderPhase: string | null = null;

  override createRenderRoot() { return this; }

  override connectedCallback(): void {
    super.connectedCallback();
    if (this.compositions.length > 0 && !this.selectedId) {
      this.selectedId = this.compositions[0]!.id;
    }
    this._loadJobs();
  }

  private get bus() { return window.sdk.bus; }

  private get selected(): CompositionEntry | undefined {
    return this.compositions.find(c => c.id === this.selectedId);
  }

  private async _loadJobs(): Promise<void> {
    this.loading = true;
    try {
      const result = await this.bus.send('video:jobs', {}, 'video-ui').wait();
      this.jobs = result.jobs;
    } catch {
      this.jobs = [];
    } finally {
      this.loading = false;
    }
  }

  private async _startRender(): Promise<void> {
    const comp = this.selected;
    if (!comp) return;

    this.renderProgress = 0;
    this.renderPhase = 'starting';

    try {
      const result = await this.bus.send('video:render', {
        compositionId: comp.id,
        format: 'mp4',
        width: comp.width,
        height: comp.height,
        fps: comp.fps,
        durationInFrames: comp.durationInFrames,
      }, 'video-ui').wait();

      this.renderPhase = 'submitted';
      void this._pollStatus(result.jobId);
    } catch {
      this.renderPhase = 'error';
      this.renderProgress = null;
    }
  }

  private async _pollStatus(jobId: string): Promise<void> {
    let done = false;
    while (!done) {
      await new Promise(r => setTimeout(r, 1000));
      try {
        const result = await this.bus.send('video:status', { jobId }, 'video-ui').wait();
        const job = result.job;
        this.renderPhase = job.phase;
        this.renderProgress = job.progress;
        if (job.phase === 'done' || job.phase === 'error') {
          done = true;
          void this._loadJobs();
        }
      } catch {
        done = true;
        this.renderPhase = 'error';
      }
    }
  }

  private _phaseBadge(phase: string) {
    const colors: Record<string, string> = {
      created: 'bg-gray-500',
      capturing: 'bg-blue-500',
      encoding: 'bg-yellow-500',
      done: 'bg-green-500',
      error: 'bg-red-500',
    };
    return html`<span class="inline-block px-2 py-0.5 text-xs rounded-full text-white ${colors[phase] ?? 'bg-gray-500'}">${phase}</span>`;
  }

  override render() {
    return html`
      <div class="p-6 max-w-6xl mx-auto">
        <h1 class="text-2xl font-bold mb-4 text-white">Video</h1>

        <div class="flex gap-2 mb-6">
          <button
            class="px-4 py-2 rounded text-sm ${this.tab === 'editor' ? 'bg-blue-600 text-white' : 'bg-zinc-800 text-zinc-400'}"
            @click=${() => { this.tab = 'editor'; }}
          >Editor</button>
          <button
            class="px-4 py-2 rounded text-sm ${this.tab === 'render-queue' ? 'bg-blue-600 text-white' : 'bg-zinc-800 text-zinc-400'}"
            @click=${() => { this.tab = 'render-queue'; void this._loadJobs(); }}
          >Render Queue</button>
        </div>

        ${this.tab === 'editor' ? this._renderEditor() : this._renderQueue()}
      </div>
    `;
  }

  private _renderEditor() {
    const comp = this.selected;
    return html`
      <div class="flex gap-6">
        <div class="w-48 space-y-2">
          <h3 class="text-sm font-medium text-zinc-400 mb-2">Compositions</h3>
          ${this.compositions.map(c => html`
            <button
              class="block w-full text-left px-3 py-2 rounded text-sm ${c.id === this.selectedId ? 'bg-zinc-700 text-white' : 'text-zinc-400 hover:bg-zinc-800'}"
              @click=${() => { this.selectedId = c.id; }}
            >${c.label}</button>
          `)}
        </div>

        <div class="flex-1">
          ${comp ? html`
            <div class="mb-4">
              <video-player
                .width=${comp.width}
                .height=${comp.height}
                .fps=${comp.fps}
                .durationInFrames=${comp.durationInFrames}
                .scale=${0.5}
              >
                <slot name="composition-${comp.id}"></slot>
              </video-player>
            </div>

            <div class="flex items-center gap-4">
              <button
                class="px-4 py-2 bg-blue-600 text-white rounded text-sm hover:bg-blue-500 disabled:opacity-50"
                ?disabled=${this.renderPhase !== null && this.renderPhase !== 'done' && this.renderPhase !== 'error'}
                @click=${this._startRender}
              >Render MP4</button>

              ${this.renderPhase ? html`
                <div class="flex items-center gap-2 text-sm text-zinc-400">
                  ${this._phaseBadge(this.renderPhase)}
                  ${this.renderProgress !== null ? html`
                    <span>${Math.round(this.renderProgress * 100)}%</span>
                  ` : nothing}
                </div>
              ` : nothing}
            </div>

            <div class="mt-4 text-xs text-zinc-500">
              ${comp.width}x${comp.height} @ ${comp.fps}fps &middot; ${comp.durationInFrames} frames
            </div>
          ` : html`<p class="text-zinc-500">Select a composition</p>`}
        </div>
      </div>
    `;
  }

  private _renderQueue() {
    if (this.loading) {
      return html`<p class="text-zinc-500">Loading...</p>`;
    }

    if (this.jobs.length === 0) {
      return html`<p class="text-zinc-500">No render jobs yet</p>`;
    }

    return html`
      <div class="space-y-2">
        ${this.jobs.map(job => html`
          <div class="flex items-center justify-between bg-zinc-800 rounded px-4 py-3">
            <div class="flex items-center gap-3">
              <span class="text-sm text-zinc-300 font-mono">${job.id.slice(0, 8)}</span>
              ${this._phaseBadge(job.phase)}
            </div>
            <div class="text-sm text-zinc-400">
              ${job.framesReceived}/${job.totalFrames} frames
              ${job.phase === 'done' ? html`
                <span class="ml-2 text-green-400">Ready</span>
              ` : nothing}
            </div>
          </div>
        `)}
      </div>
    `;
  }
}
