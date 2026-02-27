export interface Connection {
  id: string;
  services: string[];
  request<T>(service: string, method: string, params?: unknown): Promise<T>;
  httpProxy(service: string, path: string, init?: RequestInit): Promise<Response>;
}

export interface RenderJob {
  id: string;
  phase: 'created' | 'capturing' | 'encoding' | 'done' | 'error';
  progress: number;
  error?: string;
  framesReceived: number;
  totalFrames: number;
}

export interface CompositionEntry {
  id: string;
  label: string;
  width: number;
  height: number;
  fps: number;
  durationInFrames: number;
}
