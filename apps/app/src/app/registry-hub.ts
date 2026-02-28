import {
  HttpPluginRegistry,
  Logger,
  type PluginDescriptor,
  type RegistryHealth,
} from '@adi-family/sdk-plugin';
import { DEFAULT_REGISTRIES } from './env';
import type { Context } from './app';

const DB_NAME = 'adi-app';
const DB_VERSION = 1;
const STORE = 'prefs';
const DB_KEY = 'registry-urls';

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

  constructor(url: string) {
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

  connect(): void {
    if (this.disposed) return;
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
    } else {
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
}

export class RegistryHub {
  private readonly log = new Logger('registry-hub');
  private readonly servers = new Map<string, RegistryServer>();
  private readonly protectedUrls: ReadonlySet<string>;

  private constructor(protectedUrls: string[]) {
    this.protectedUrls = new Set(protectedUrls);
  }

  static async init(ctx: Context): Promise<RegistryHub> {
    const hub = new RegistryHub(DEFAULT_REGISTRIES);
    const saved = await hub.loadUrls(ctx);
    const urls = saved.length > 0 ? saved : DEFAULT_REGISTRIES;
    for (const url of urls) hub.connectOne(url);
    return hub;
  }

  getServer(url: string): RegistryServer | undefined {
    return this.servers.get(url);
  }

  allServers(): ReadonlySet<RegistryServer> {
    return new Set(this.servers.values());
  }

  addUrl(ctx: Context, url: string): void {
    this.connectOne(url);
    void this.persist(ctx);
  }

  removeUrl(ctx: Context, url: string): void {
    if (this.protectedUrls.has(url)) return;
    this.disconnectOne(url);
    void this.persist(ctx);
  }

  dispose(): void {
    for (const server of this.servers.values()) server.disconnect();
    this.servers.clear();
  }

  private connectOne(url: string): void {
    if (this.servers.has(url)) return;
    this.log.trace({ msg: 'connecting', url });
    const server = new RegistryServer(url);
    this.servers.set(url, server);
    server.connect();
  }

  private disconnectOne(url: string): void {
    const server = this.servers.get(url);
    if (!server) return;
    this.log.trace({ msg: 'disconnecting', url });
    server.disconnect();
    this.servers.delete(url);
  }

  private async loadUrls(ctx: Context): Promise<string[]> {
    try {
      const db = await ctx.db.open(DB_NAME, DB_VERSION);
      return await new Promise((resolve, reject) => {
        const tx = db.transaction(STORE, 'readonly');
        const req = tx.objectStore(STORE).get(DB_KEY);
        req.onsuccess = () =>
          resolve((req.result as string[] | undefined) ?? []);
        req.onerror = () => reject(req.error);
      });
    } catch {
      return [];
    }
  }

  private async persist(ctx: Context): Promise<void> {
    try {
      const db = await ctx.db.open(DB_NAME, DB_VERSION);
      const tx = db.transaction(STORE, 'readwrite');
      tx.objectStore(STORE).put([...this.servers.keys()], DB_KEY);
    } catch (err) {
      this.log.warn({ msg: 'persist failed', error: String(err) });
    }
  }
}
