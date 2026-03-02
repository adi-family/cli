import { AdiPlugin } from '@adi-family/sdk-plugin';
import * as api from './api.js';
import type { Connection, RenderJob } from './types.js';
import { setBus, connections } from './context.js';
import './events.js';

function connectionsWithVideo(): Connection[] {
  return [...connections.values()]
    .filter(c => c.services.includes('video'));
}

export class VideoPlugin extends AdiPlugin {
  readonly id = 'adi.video';
  readonly version = '0.1.0';

  async onRegister(): Promise<void> {
    setBus(this.bus);

    this.bus.on('connection:added', ({ id, connection }) => {
      connections.set(id, connection as Connection);
    }, 'video');
    this.bus.on('connection:removed', ({ id }) => {
      connections.delete(id);
    }, 'video');

    const { AdiVideoElement } = await import('./component.js');
    if (!customElements.get('adi-video')) {
      customElements.define('adi-video', AdiVideoElement);
    }

    this.bus.emit('route:register', { path: '/video', element: 'adi-video' }, 'video');
    this.bus.emit('nav:add', { id: 'video', label: 'Video', path: '/video' }, 'video');

    this.bus.emit('command:register', { id: 'video:open', label: 'Go to Video page' }, 'video');
    this.bus.on('command:execute', ({ id }) => {
      if (id === 'video:open') this.bus.emit('router:navigate', { path: '/video' }, 'video');
    }, 'video');

    this.bus.on('video:render', async ({ compositionId: _, format, width, height, fps, durationInFrames }) => {
      try {
        const conns = connectionsWithVideo();
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
        const conns = connectionsWithVideo();
        if (conns.length === 0) throw new Error('No video service connected');
        const job = await api.getJobStatus(conns[0]!, jobId);
        this.bus.emit('video:status-changed', { job }, 'video');
      } catch (err) {
        console.error('[VideoPlugin] video:status error:', err);
      }
    }, 'video');

    this.bus.on('video:jobs', async () => {
      try {
        const conns = connectionsWithVideo();
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
