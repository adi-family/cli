import { AdiPlugin } from '@adi-family/sdk-plugin';
import { AdiRouterBusKey } from '@adi-family/plugin-router/bus';
import * as api from './api.js';
import type { RenderJob } from './types.js';
import { cocoon } from './cocoon.js';
import './events.js';

export class VideoPlugin extends AdiPlugin {
  readonly id = 'adi.video';
  readonly version = '0.1.0';

  async onRegister(): Promise<void> {
    cocoon.init(this.bus);

    const { AdiVideoElement } = await import('./component.js');
    if (!customElements.get('adi-video')) {
      customElements.define('adi-video', AdiVideoElement);
    }

    this.bus.emit(AdiRouterBusKey.RegisterRoute, { pluginId: this.id, path: '', init: () => document.createElement('adi-video'), label: 'Video' }, this.id);
    this.bus.emit('nav:add', { id: this.id, label: 'Video', path: `/${this.id}` }, this.id);

    this.bus.on('video:render', async ({ compositionId: _, format, width, height, fps, durationInFrames }) => {
      try {
        const conns = cocoon.connectionsWithService('video');
        if (conns.length === 0) throw new Error('No video service connected');
        const result = await api.startRender(conns[0]!, {
          width, height, fps, totalFrames: durationInFrames, format,
        });
        this.bus.emit('video:render-started', { jobId: result.job_id }, 'video');
      } catch (err) {
        console.error('[VideoPlugin] video:render error:', err);
      }
    }, 'video');

    this.bus.on('video:status', async ({ jobId }) => {
      try {
        const conns = cocoon.connectionsWithService('video');
        if (conns.length === 0) throw new Error('No video service connected');
        const job = await api.getJobStatus(conns[0]!, jobId);
        this.bus.emit('video:status-changed', { job }, 'video');
      } catch (err) {
        console.error('[VideoPlugin] video:status error:', err);
      }
    }, 'video');

    this.bus.on('video:jobs', async () => {
      try {
        const conns = cocoon.connectionsWithService('video');
        const results = await Promise.allSettled(conns.map(c => api.listJobs(c)));
        const jobs: RenderJob[] = results.flatMap(r =>
          r.status === 'fulfilled' ? r.value : []
        );
        this.bus.emit('video:jobs-changed', { jobs }, 'video');
      } catch (err) {
        console.error('[VideoPlugin] video:jobs error:', err);
        this.bus.emit('video:jobs-changed', { jobs: [] }, 'video');
      }
    }, 'video');
  }
}
