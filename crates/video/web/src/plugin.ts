import { AdiPlugin } from '@adi-family/sdk-plugin';
import type { WithCid } from '@adi-family/sdk-plugin';
import * as api from './api.js';
import type { Connection, RenderJob } from './types.js';
import './events.js';

function connectionsWithVideo(): Connection[] {
  return [...window.sdk.getConnections().values()]
    .filter(c => c.services.includes('video'));
}

export class VideoPlugin extends AdiPlugin {
  readonly id = 'adi.video';
  readonly version = '0.1.0';

  async onRegister(): Promise<void> {
    const { AdiVideoElement } = await import('./component.js');
    if (!customElements.get('adi-video')) {
      customElements.define('adi-video', AdiVideoElement);
    }

    this.bus.emit('route:register', { path: '/video', element: 'adi-video' }, 'video');
    this.bus.send('nav:add', { id: 'video', label: 'Video', path: '/video' }, 'video').handle(() => {});

    this.bus.emit('command:register', { id: 'video:open', label: 'Go to Video page' }, 'video');
    this.bus.on('command:execute', ({ id }) => {
      if (id === 'video:open') this.bus.emit('router:navigate', { path: '/video' }, 'video');
    }, 'video');

    this.bus.on('video:render', async (p) => {
      const { _cid, compositionId: _, format, width, height, fps, durationInFrames } = p as WithCid<typeof p>;
      try {
        const conns = connectionsWithVideo();
        if (conns.length === 0) throw new Error('No video service connected');
        const result = await api.startRender(conns[0]!, {
          width, height, fps, totalFrames: durationInFrames, format,
        });
        this.bus.emit('video:render:ok', { jobId: result.job_id, _cid }, 'video');
      } catch (err) {
        console.error('[VideoPlugin] video:render error:', err);
      }
    }, 'video');

    this.bus.on('video:status', async (p) => {
      const { _cid, jobId } = p as WithCid<typeof p>;
      try {
        const conns = connectionsWithVideo();
        if (conns.length === 0) throw new Error('No video service connected');
        const job = await api.getJobStatus(conns[0]!, jobId);
        this.bus.emit('video:status:ok', { job, _cid }, 'video');
      } catch (err) {
        console.error('[VideoPlugin] video:status error:', err);
      }
    }, 'video');

    this.bus.on('video:jobs', async (p) => {
      const { _cid } = p as WithCid<typeof p>;
      try {
        const conns = connectionsWithVideo();
        const results = await Promise.allSettled(conns.map(c => api.listJobs(c)));
        const jobs: RenderJob[] = results.flatMap(r =>
          r.status === 'fulfilled' ? r.value : []
        );
        this.bus.emit('video:jobs:ok', { jobs, _cid }, 'video');
      } catch (err) {
        console.error('[VideoPlugin] video:jobs error:', err);
        this.bus.emit('video:jobs:ok', { jobs: [], _cid }, 'video');
      }
    }, 'video');
  }
}
