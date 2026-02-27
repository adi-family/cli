import { captureFrame } from './frame-capturer.js';
import type { RenderStatus, OutputFormat } from './types.js';
import type { VideoComposition } from './composition.js';

interface RenderApi {
  createJob(config: {
    width: number; height: number; fps: number;
    totalFrames: number; format: OutputFormat;
  }): Promise<string>;
  submitFrame(jobId: string, index: number, blob: Blob): Promise<void>;
  finishUpload(jobId: string): Promise<void>;
  getStatus(jobId: string): Promise<RenderStatus>;
}

export async function executeRender(
  composition: VideoComposition,
  captureTarget: HTMLElement,
  format: OutputFormat,
  api: RenderApi,
  onProgress?: (status: RenderStatus) => void,
): Promise<string> {
  const { width, height, fps, durationInFrames } = composition.config;

  const jobId = await api.createJob({
    width, height, fps, totalFrames: durationInFrames, format,
  });

  onProgress?.({ jobId, phase: 'capturing', progress: 0 });

  for (let i = 0; i < durationInFrames; i++) {
    composition.seekTo(i);
    await new Promise(r => requestAnimationFrame(r));
    await new Promise(r => requestAnimationFrame(r));

    const blob = await captureFrame(captureTarget, { width, height });
    await api.submitFrame(jobId, i, blob);

    onProgress?.({
      jobId,
      phase: 'capturing',
      progress: (i + 1) / durationInFrames,
    });
  }

  await api.finishUpload(jobId);
  onProgress?.({ jobId, phase: 'encoding', progress: 0 });

  let status: RenderStatus;
  do {
    await new Promise(r => setTimeout(r, 500));
    status = await api.getStatus(jobId);
    onProgress?.(status);
  } while (status.phase === 'encoding');

  return jobId;
}
