export interface VideoConfig {
  width: number;
  height: number;
  fps: number;
  durationInFrames: number;
}

export type OutputFormat = 'mp4' | 'webm' | 'gif';

export interface RenderOptions {
  format: OutputFormat;
  quality?: number;
  compositionId: string;
}

export type RenderPhase = 'created' | 'capturing' | 'encoding' | 'done' | 'error';

export interface RenderStatus {
  jobId: string;
  phase: RenderPhase;
  progress: number;
  error?: string;
}

export interface InterpolateOptions {
  extrapolateLeft?: 'clamp' | 'extend';
  extrapolateRight?: 'clamp' | 'extend';
}

export interface SpringConfig {
  damping?: number;
  mass?: number;
  stiffness?: number;
  overshootClamping?: boolean;
}
