import type { AdiRequest, AdiResponse, AdiDiscovery, AdiServiceInfo } from './types.ts';

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
// Factory
// ---------------------------------------------------------------------------

const DEFAULT_TIMEOUT_MS = 30_000;

let idCounter = 0;
const nextId = (): string => `req-${Date.now()}-${++idCounter}`;

export const createAdiChannel = (
  send: SendFn,
  options?: AdiChannelOptions,
): AdiChannel => {
  const timeoutMs = options?.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const pending = new Map<string, PendingRequest>();
  const streams = new Map<string, PendingStream>();

  const cleanup = (id: string): void => {
    const req = pending.get(id);
    if (req) {
      clearTimeout(req.timer);
      pending.delete(id);
    }
    const s = streams.get(id);
    if (s) {
      clearTimeout(s.timer);
      streams.delete(id);
    }
  };

  const request = <T>(service: string, method: string, params?: Record<string, unknown>): Promise<T> => {
    const requestId = nextId();
    return new Promise<T>((resolve, reject) => {
      const timer = setTimeout(() => {
        pending.delete(requestId);
        reject(new AdiTimeoutError(requestId));
      }, timeoutMs);

      pending.set(requestId, {
        resolve: resolve as (data: unknown) => void,
        reject,
        timer,
      });

      send({ request_id: requestId, service, method, params: params ?? {} });
    });
  };

  const stream = <T>(service: string, method: string, params?: Record<string, unknown>): AsyncGenerator<T> => {
    const requestId = nextId();

    // Buffered async generator backed by push queue
    const buffer: T[] = [];
    let done = false;
    let error: Error | null = null;
    let notify: (() => void) | null = null;

    const wait = (): Promise<void> =>
      new Promise<void>((resolve) => {
        notify = resolve;
      });

    const timer = setTimeout(() => {
      streams.delete(requestId);
      error = new AdiTimeoutError(requestId);
      done = true;
      notify?.();
    }, timeoutMs);

    streams.set(requestId, {
      push(data: unknown) {
        buffer.push(data as T);
        notify?.();
      },
      finish() {
        done = true;
        clearTimeout(timer);
        streams.delete(requestId);
        notify?.();
      },
      reject(err: Error) {
        error = err;
        done = true;
        clearTimeout(timer);
        streams.delete(requestId);
        notify?.();
      },
      timer,
    });

    send({ request_id: requestId, service, method, params: params ?? {} });

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
  };

  const listServices = (): Promise<AdiServiceInfo[]> => {
    const requestId = nextId();
    return new Promise<AdiServiceInfo[]>((resolve, reject) => {
      const timer = setTimeout(() => {
        pending.delete(requestId);
        reject(new AdiTimeoutError(requestId));
      }, timeoutMs);

      // Reuse pending map with a synthetic key for discovery
      pending.set(requestId, {
        resolve: resolve as (data: unknown) => void,
        reject,
        timer,
      });

      // Discovery uses list_services but we tag with requestId for correlation
      // The server responds with services_list (no request_id), so we handle it
      // specially in handleResponse.
      send({ type: 'list_services' });

      // Store the requestId so handleResponse can find it
      (listServices as { _pendingId?: string })._pendingId = requestId;
    });
  };

  const handleResponse = (msg: AdiResponse | AdiDiscovery): void => {
    // Handle discovery response (no request_id)
    if ('type' in msg && msg.type === 'services_list') {
      const discoveryMsg = msg as Extract<AdiDiscovery, { type: 'services_list' }>;
      const pendingId = (listServices as { _pendingId?: string })._pendingId;
      if (pendingId) {
        const req = pending.get(pendingId);
        if (req) {
          clearTimeout(req.timer);
          pending.delete(pendingId);
          req.resolve(discoveryMsg.services);
        }
        delete (listServices as { _pendingId?: string })._pendingId;
      }
      return;
    }

    // All other responses have request_id
    if (!('request_id' in msg)) return;
    const response = msg as AdiResponse;
    const { request_id: id } = response;

    switch (response.type) {
      case 'success': {
        const req = pending.get(id);
        if (req) {
          cleanup(id);
          req.resolve(response.data);
        }
        break;
      }
      case 'stream': {
        const s = streams.get(id);
        if (s) {
          s.push(response.data);
          if (response.done) s.finish();
        }
        break;
      }
      case 'error': {
        const err = new AdiError(response.message, response.code, id);
        const req = pending.get(id);
        if (req) { cleanup(id); req.reject(err); }
        const s = streams.get(id);
        if (s) { s.reject(err); }
        break;
      }
      case 'service_not_found': {
        const err = new AdiServiceNotFoundError(id, response.service);
        const req = pending.get(id);
        if (req) { cleanup(id); req.reject(err); }
        const s = streams.get(id);
        if (s) { s.reject(err); }
        break;
      }
      case 'method_not_found': {
        const err = new AdiError(
          `Method not found: ${response.service}.${response.method}`,
          'METHOD_NOT_FOUND',
          id,
        );
        const req = pending.get(id);
        if (req) { cleanup(id); req.reject(err); }
        const s = streams.get(id);
        if (s) { s.reject(err); }
        break;
      }
    }
  };

  const cancelAll = (): void => {
    for (const [id, req] of pending) {
      clearTimeout(req.timer);
      req.reject(new AdiError('Channel closed', 'CANCELLED', id));
    }
    pending.clear();

    for (const [id, s] of streams) {
      clearTimeout(s.timer);
      s.reject(new AdiError('Channel closed', 'CANCELLED', id));
    }
    streams.clear();

    delete (listServices as { _pendingId?: string })._pendingId;
  };

  return { request, stream, listServices, handleResponse, cancelAll };
};
