import type { WsState, SignalingMessage } from './types.ts';

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

export const createWebSocket = (url: string, handlers: WsHandlers): WsControl => {
  let ws: WebSocket | null = null;
  let current: WsState = 'disconnected';
  let attempts = 0;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let connectedAt: number | null = null;

  const setState = (next: WsState): void => {
    current = next;
    handlers.onStateChange(next);
  };

  const clearReconnect = (): void => {
    if (reconnectTimer !== null) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
  };

  const scheduleReconnect = (): void => {
    // Reset backoff if connection was stable
    if (connectedAt !== null && Date.now() - connectedAt >= STABLE_CONNECTION_MS) {
      attempts = 0;
    }
    connectedAt = null;

    if (attempts >= MAX_RECONNECT_ATTEMPTS) {
      console.debug('[signaling] max reconnect attempts reached');
      handlers.onError('Max reconnection attempts reached');
      return;
    }

    clearReconnect();
    const delay = Math.min(RECONNECT_BASE_MS * Math.pow(2, attempts), RECONNECT_CAP_MS);
    attempts++;
    console.debug(`[signaling] reconnecting in ${delay}ms (attempt ${attempts})`);
    reconnectTimer = setTimeout(connect, delay);
  };

  function connect(): void {
    if (ws?.readyState === WebSocket.OPEN) return;

    console.debug(`[signaling] connecting to ${url}`);
    setState('connecting');

    try {
      ws = new WebSocket(url);

      ws.onopen = () => {
        console.debug('[signaling] connected');
        setState('connected');
        connectedAt = Date.now();
      };

      ws.onmessage = (event) => {
        try {
          handlers.onMessage(JSON.parse(event.data as string));
        } catch (err) {
          console.debug('[signaling] failed to parse message', err);
        }
      };

      ws.onerror = () => {
        handlers.onError('WebSocket connection error');
        setState('error');
      };

      ws.onclose = () => {
        console.debug('[signaling] disconnected');
        setState('disconnected');
        scheduleReconnect();
      };
    } catch (err) {
      setState('error');
      handlers.onError(`Failed to connect: ${err}`);
    }
  }

  const disconnect = (): void => {
    clearReconnect();
    if (ws) {
      ws.close();
      ws = null;
    }
    setState('disconnected');
  };

  const send = (msg: SignalingMessage): void => {
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(msg));
    } else {
      console.debug('[signaling] cannot send, ws not open', msg.type);
    }
  };

  return { connect, disconnect, send, state: () => current };
};
