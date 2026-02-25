import type { AdiChannel } from './adi-channel.ts';

export interface Connection {
  id: string;
  services: string[];
  request<T>(service: string, method: string, params?: unknown): Promise<T>;
  stream<T>(service: string, method: string, params?: unknown): AsyncGenerator<T>;
  httpProxy(service: string, path: string, init?: RequestInit): Promise<Response>;
  httpDirect(url: string, init?: RequestInit): Promise<Response>;
}

export const createConnection = (
  deviceId: string,
  services: string[],
  adi: AdiChannel,
): Connection => ({
  id: deviceId,
  services,

  request: <T>(service: string, method: string, params?: unknown) =>
    adi.request<T>(service, method, params as Record<string, unknown>),

  stream: <T>(service: string, method: string, params?: unknown) =>
    adi.stream<T>(service, method, params as Record<string, unknown>),

  httpProxy: (service: string, path: string, init?: RequestInit) =>
    adi.request<Response>('proxy', 'forward', { service, path, init }),

  httpDirect: (url: string, init?: RequestInit) =>
    fetch(url, init),
});
