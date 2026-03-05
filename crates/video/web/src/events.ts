import type { RenderJob } from './types.js';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    'video:render':  { compositionId: string; format: string; width: number; height: number; fps: number; durationInFrames: number };
    'video:status':  { jobId: string };
    'video:jobs':    Record<string, never>;

    'video:jobs-changed':    { jobs: RenderJob[] };
    'video:render-started':  { jobId: string };
    'video:status-changed':  { job: RenderJob };
  }
}

export {};
