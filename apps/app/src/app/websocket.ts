import { Logger, trace } from '@adi-family/sdk-plugin';
import type { WsState, SignalingMessage } from './signaling-types.ts';

export interface WsHandlers {
  onStateChange(state: WsState): void;
  onMessage(msg: SignalingMessage): void;
  onError(error: string): void;
}

export interface WsControl {
  connect(): void;
  disconnect(): void;
  send(msg: SignalingMessage): void;
  state(): WsState;
}

const RECONNECT_BASE_MS = 1000;
const RECONNECT_CAP_MS = 30_000;
const MAX_RECONNECT_ATTEMPTS = 5;
const STABLE_CONNECTION_MS = 5000;

class WebSocketClient implements WsControl {
  private readonly log = new Logger('ws', () => ({
    url: this.url,
    state: this.current,
    attempts: this.attempts,
  }));

  private ws: WebSocket | null = null;
  private current: WsState = 'disconnected';
  private attempts = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private connectedAt: number | null = null;

  constructor(
    private readonly url: string,
    private readonly handlers: WsHandlers,
  ) {}

  state(): WsState {
    return this.current;
  }

  @trace('connecting')
  connect(): void {
    if (this.ws?.readyState === WebSocket.OPEN) return;

    this.setState('connecting');

    try {
      this.ws = new WebSocket(this.url);

      this.ws.onopen = () => {
        this.log.trace({ msg: 'connected' });
        this.setState('connected');
        this.connectedAt = Date.now();
      };

      this.ws.onmessage = (event) => {
        try {
          this.handlers.onMessage(JSON.parse(event.data as string));
        } catch (err) {
          this.log.warn({ msg: 'failed to parse message', error: String(err) });
        }
      };

      this.ws.onerror = () => {
        this.handlers.onError('WebSocket connection error');
        this.setState('error');
      };

      this.ws.onclose = () => {
        this.log.trace({ msg: 'disconnected' });
        this.setState('disconnected');
        this.scheduleReconnect();
      };
    } catch (err) {
      this.setState('error');
      this.handlers.onError(`Failed to connect: ${err}`);
    }
  }

  @trace('disconnecting')
  disconnect(): void {
    this.clearReconnect();
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.setState('disconnected');
  }

  send(msg: SignalingMessage): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    } else {
      this.log.trace({ msg: 'cannot send, ws not open', type: msg.type });
    }
  }

  private setState(next: WsState): void {
    this.current = next;
    this.handlers.onStateChange(next);
  }

  private clearReconnect(): void {
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private scheduleReconnect(): void {
    if (this.connectedAt !== null && Date.now() - this.connectedAt >= STABLE_CONNECTION_MS) {
      this.attempts = 0;
    }
    this.connectedAt = null;

    if (this.attempts >= MAX_RECONNECT_ATTEMPTS) {
      this.log.trace({ msg: 'max reconnect attempts reached' });
      this.handlers.onError('Max reconnection attempts reached');
      return;
    }

    this.clearReconnect();
    const delay = Math.min(RECONNECT_BASE_MS * Math.pow(2, this.attempts), RECONNECT_CAP_MS);
    this.attempts++;
    this.log.trace({ msg: 'scheduling reconnect', delay, attempt: this.attempts });
    this.reconnectTimer = setTimeout(() => this.connect(), delay);
  }
}

export const createWebSocket = (url: string, handlers: WsHandlers): WsControl =>
  new WebSocketClient(url, handlers);
