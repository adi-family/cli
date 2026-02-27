import type { RenderJob } from './types.js';

declare module '@adi-family/sdk-plugin' {
  interface EventRegistry {
    'video:render':       { compositionId: string; format: string; width: number; height: number; fps: number; durationInFrames: number };
    'video:render:ok':    { jobId: string; _cid: string };

    'video:status':       { jobId: string };
    'video:status:ok':    { job: RenderJob; _cid: string };

    'video:jobs':         Record<string, never>;
    'video:jobs:ok':      { jobs: RenderJob[]; _cid: string };
  }
}

export {};
