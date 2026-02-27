import type { Connection, RenderJob } from './types.js';

const SVC = 'video';

export const startRender = (c: Connection, params: {
  width: number; height: number; fps: number;
  totalFrames: number; format: string;
}) =>
  c.request<{ job_id: string }>(SVC, 'start_render', params);

export const getJobStatus = (c: Connection, jobId: string) =>
  c.request<RenderJob>(SVC, 'get_status', { job_id: jobId });

export const listJobs = (c: Connection) =>
  c.request<RenderJob[]>(SVC, 'list_jobs', {});
