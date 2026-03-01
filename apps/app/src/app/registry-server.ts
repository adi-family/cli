import {
  HttpPluginRegistry,
  Logger,
  type PluginDescriptor,
  type RegistryHealth,
} from '@adi-family/sdk-plugin';

const POLL_MS = 60_000;
const RECONNECT_BASE_MS = 2_000;
const RECONNECT_CAP_MS = 30_000;

export type RegistryState = 'disconnected' | 'connecting' | 'connected';

export class RegistryServer {
  readonly url: string;

  private readonly log = new Logger('registry-server');
  private readonly client: HttpPluginRegistry;
  private state: RegistryState = 'disconnected';
  private health: RegistryHealth | null = null;
  private plugins: PluginDescriptor[] = [];
  private pollTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectAttempt = 0;
  private disposed = false;

  constructor(url: string, private readonly isStarted: () => boolean) {
    this.url = url;
    this.client = new HttpPluginRegistry(url);
  }

  getState(): RegistryState {
    return this.state;
  }

  getHealth(): RegistryHealth | null {
    return this.health;
  }

  getPlugins(): readonly PluginDescriptor[] {
    return this.plugins;
  }

  getClient(): HttpPluginRegistry {
    return this.client;
  }

  connect(): void {
    if (this.disposed || !this.isStarted()) return;
    this.scheduleCheck(0);
  }

  disconnect(): void {
    this.disposed = true;
    this.clearTimers();
    this.state = 'disconnected';
  }

  private clearTimers(): void {
    if (this.pollTimer) {
      clearTimeout(this.pollTimer);
      this.pollTimer = null;
    }
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private scheduleCheck(delayMs: number): void {
    this.clearTimers();
    if (this.disposed) return;
    this.pollTimer = setTimeout(() => this.poll(), delayMs);
  }

  private scheduleReconnect(): void {
    if (this.disposed) return;
    const jitter = Math.random() * 500;
    const delay =
      Math.min(
        RECONNECT_BASE_MS * 2 ** this.reconnectAttempt,
        RECONNECT_CAP_MS,
      ) + jitter;
    this.reconnectAttempt++;
    this.reconnectTimer = setTimeout(() => this.poll(), delay);
  }

  private async poll(): Promise<void> {
    if (this.disposed) return;
    this.state = 'connecting';
    this.log.trace({ msg: 'polling', url: this.url });

    const health = await this.client.checkHealth();
    if (this.disposed) return;

    this.health = health;

    if (health.online) {
      this.reconnectAttempt = 0;
      this.plugins = await this.client.listPlugins();
      this.log.trace({
        msg: 'connected',
        url: this.url,
        plugins: this.plugins.length,
      });
      this.state = 'connected';
      this.scheduleCheck(POLL_MS);
      return;
    }

    this.log.warn({
      msg: 'offline',
      url: this.url,
      attempt: this.reconnectAttempt,
    });
    this.plugins = [];
    this.state = 'disconnected';
    this.scheduleReconnect();
  }
}
