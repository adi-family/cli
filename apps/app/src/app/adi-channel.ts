import { Logger, trace } from '@adi-family/sdk-plugin';
import type { AdiRequest, AdiResponse, AdiDiscovery, AdiServiceInfo } from './signaling-types.ts';

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

export class AdiError extends Error {
  constructor(
    message: string,
    public readonly code: string,
    public readonly requestId: string,
  ) {
    super(message);
    this.name = 'AdiError';
  }
}

export class AdiTimeoutError extends AdiError {
  constructor(requestId: string) {
    super('Request timed out', 'TIMEOUT', requestId);
    this.name = 'AdiTimeoutError';
  }
}

export class AdiServiceNotFoundError extends AdiError {
  constructor(requestId: string, public readonly service: string) {
    super(`Service not found: ${service}`, 'SERVICE_NOT_FOUND', requestId);
    this.name = 'AdiServiceNotFoundError';
  }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type SendFn = (payload: AdiRequest | AdiDiscovery) => void;

interface PendingRequest {
  resolve(data: unknown): void;
  reject(err: Error): void;
  timer: ReturnType<typeof setTimeout>;
}

interface PendingStream {
  push(data: unknown): void;
  finish(): void;
  reject(err: Error): void;
  timer: ReturnType<typeof setTimeout>;
}

export interface AdiChannelOptions {
  timeoutMs?: number;
}

export interface AdiChannel {
  request<T>(service: string, method: string, params?: Record<string, unknown>): Promise<T>;
  stream<T>(service: string, method: string, params?: Record<string, unknown>): AsyncGenerator<T>;
  listServices(): Promise<AdiServiceInfo[]>;
  handleResponse(msg: AdiResponse | AdiDiscovery): void;
  cancelAll(): void;
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

const DEFAULT_TIMEOUT_MS = 30_000;

let idCounter = 0;
const nextId = (): string => `req-${Date.now()}-${++idCounter}`;

class AdiChannelClient implements AdiChannel {
  private readonly log = new Logger('adi-channel', () => ({
    pending: this.pending.size,
    streams: this.streams.size,
  }));
  private readonly timeoutMs: number;
  private readonly pending = new Map<string, PendingRequest>();
  private readonly streams = new Map<string, PendingStream>();
  private pendingDiscoveryId: string | null = null;

  constructor(
    private readonly send: SendFn,
    options?: AdiChannelOptions,
  ) {
    this.timeoutMs = options?.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  }

  @trace('requesting')
  request<T>(service: string, method: string, params?: Record<string, unknown>): Promise<T> {
    const requestId = nextId();
    return new Promise<T>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(requestId);
        reject(new AdiTimeoutError(requestId));
      }, this.timeoutMs);

      this.pending.set(requestId, {
        resolve: resolve as (data: unknown) => void,
        reject,
        timer,
      });

      this.send({ request_id: requestId, service, method, params: params ?? {} });
    });
  }

  @trace('streaming')
  stream<T>(service: string, method: string, params?: Record<string, unknown>): AsyncGenerator<T> {
    const requestId = nextId();

    const buffer: T[] = [];
    let done = false;
    let error: Error | null = null;
    let notify: (() => void) | null = null;

    const wait = (): Promise<void> =>
      new Promise<void>((resolve) => {
        notify = resolve;
      });

    const timer = setTimeout(() => {
      this.streams.delete(requestId);
      error = new AdiTimeoutError(requestId);
      done = true;
      notify?.();
    }, this.timeoutMs);

    const streamsMap = this.streams;
    streamsMap.set(requestId, {
      push(data: unknown) {
        buffer.push(data as T);
        notify?.();
      },
      finish() {
        done = true;
        clearTimeout(timer);
        streamsMap.delete(requestId);
        notify?.();
      },
      reject(err: Error) {
        error = err;
        done = true;
        clearTimeout(timer);
        streamsMap.delete(requestId);
        notify?.();
      },
      timer,
    });

    this.send({ request_id: requestId, service, method, params: params ?? {} });

    return (async function* () {
      while (true) {
        while (buffer.length > 0) {
          yield buffer.shift()!;
        }
        if (error) throw error;
        if (done) return;
        await wait();
      }
    })();
  }

  @trace('listing services')
  listServices(): Promise<AdiServiceInfo[]> {
    const requestId = nextId();
    return new Promise<AdiServiceInfo[]>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(requestId);
        reject(new AdiTimeoutError(requestId));
      }, this.timeoutMs);

      this.pending.set(requestId, {
        resolve: resolve as (data: unknown) => void,
        reject,
        timer,
      });

      this.send({ type: 'list_services' });
      this.pendingDiscoveryId = requestId;
    });
  }

  handleResponse(msg: AdiResponse | AdiDiscovery): void {
    if ('type' in msg && msg.type === 'services_list') {
      const discoveryMsg = msg as Extract<AdiDiscovery, { type: 'services_list' }>;
      if (this.pendingDiscoveryId) {
        const req = this.pending.get(this.pendingDiscoveryId);
        if (req) {
          clearTimeout(req.timer);
          this.pending.delete(this.pendingDiscoveryId);
          req.resolve(discoveryMsg.services);
        }
        this.pendingDiscoveryId = null;
      }
      return;
    }

    if (!('request_id' in msg)) return;
    const response = msg as AdiResponse;
    const { request_id: id } = response;

    switch (response.type) {
      case 'success': {
        const req = this.pending.get(id);
        if (req) {
          this.cleanup(id);
          req.resolve(response.data);
        }
        break;
      }
      case 'stream': {
        const s = this.streams.get(id);
        if (s) {
          s.push(response.data);
          if (response.done) s.finish();
        }
        break;
      }
      case 'error': {
        const err = new AdiError(response.message, response.code, id);
        const req = this.pending.get(id);
        if (req) { this.cleanup(id); req.reject(err); }
        const s = this.streams.get(id);
        if (s) { s.reject(err); }
        break;
      }
      case 'service_not_found': {
        const err = new AdiServiceNotFoundError(id, response.service);
        const req = this.pending.get(id);
        if (req) { this.cleanup(id); req.reject(err); }
        const s = this.streams.get(id);
        if (s) { s.reject(err); }
        break;
      }
      case 'method_not_found': {
        const err = new AdiError(
          `Method not found: ${response.service}.${response.method}`,
          'METHOD_NOT_FOUND',
          id,
        );
        const req = this.pending.get(id);
        if (req) { this.cleanup(id); req.reject(err); }
        const s = this.streams.get(id);
        if (s) { s.reject(err); }
        break;
      }
    }
  }

  @trace('cancelling all')
  cancelAll(): void {
    this.log.trace({ msg: 'cancelling all', pending: this.pending.size, streams: this.streams.size });
    for (const [id, req] of this.pending) {
      clearTimeout(req.timer);
      req.reject(new AdiError('Channel closed', 'CANCELLED', id));
    }
    this.pending.clear();

    for (const [id, s] of this.streams) {
      clearTimeout(s.timer);
      s.reject(new AdiError('Channel closed', 'CANCELLED', id));
    }
    this.streams.clear();

    this.pendingDiscoveryId = null;
  }

  private cleanup(id: string): void {
    const req = this.pending.get(id);
    if (req) {
      clearTimeout(req.timer);
      this.pending.delete(id);
    }
    const s = this.streams.get(id);
    if (s) {
      clearTimeout(s.timer);
      this.streams.delete(id);
    }
  }
}

export const createAdiChannel = (
  send: SendFn,
  options?: AdiChannelOptions,
): AdiChannel => new AdiChannelClient(send, options);
