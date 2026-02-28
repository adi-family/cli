import {
  HttpPluginRegistry,
  Logger,
  type EventBus,
  type PluginDescriptor,
  type RegistryHealth,
} from '@adi-family/sdk-plugin';
import { DEFAULT_REGISTRIES } from './env';
import type { PreferencesStore } from './database';
import type { Context } from './app';

const DB_KEY = 'registry-urls';
const SOURCE = 'registry';

const POLL_INTERVAL_MS = 60_000;
const RECONNECT_BASE_MS = 2_000;
const RECONNECT_CAP_MS = 30_000;

export type RegistryState = 'disconnected' | 'connecting' | 'connected';

type RegistryListener = (state: RegistryState) => void;

export class Registry {
  readonly url: string;

  private readonly log = new Logger('registry');
  private readonly client: HttpPluginRegistry;
  private state: RegistryState = 'disconnected';
  private health: RegistryHealth | null = null;
  private plugins: PluginDescriptor[] = [];
  private pollTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectAttempt = 0;
  private disposed = false;
  private readonly listeners: Set<RegistryListener> = new Set();

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

  onChange(listener: RegistryListener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  connect(): void {
    if (this.disposed) return;
    this.scheduleCheck(0);
  }

  disconnect(): void {
    this.disposed = true;
    this.clearTimers();
    this.setState('disconnected');
  }

  async checkLatest(
    id: string,
    currentVersion: string,
  ): Promise<{ version: string } | null> {
    return this.client.checkLatest(id, currentVersion);
  }

  async bundleUrl(id: string, version: string): Promise<string> {
    return this.client.bundleUrl(id, version);
  }

  private setState(next: RegistryState): void {
    if (this.state === next) return;
    this.state = next;
    this.listeners.forEach((fn) => fn(next));
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

  private scheduleCheck(ctx: Context, delayMs: number): void {
    this.clearTimers();
    if (this.disposed) return;
    this.pollTimer = setTimeout(() => this.poll(ctx), delayMs);
  }

  private scheduleReconnect(ctx: Context): void {
    if (this.disposed) return;
    const jitter = Math.random() * 500;
    const delay =
      Math.min(
        RECONNECT_BASE_MS * 2 ** this.reconnectAttempt,
        RECONNECT_CAP_MS,
      ) + jitter;
    this.reconnectAttempt++;
    this.reconnectTimer = setTimeout(() => this.poll(ctx), delay);
  }

  private async poll(ctx: Context): Promise<void> {
    if (this.disposed) return;
    this.setState('connecting');
    this.log.trace(ctx.bus, { msg: 'polling', url: this.url });

    const health = await this.client.checkHealth();
    if (this.disposed) return;

    this.health = health;
    ctx.bus.emit('registry:health', { url: this.url, health }, SOURCE);

    if (health.online) {
      this.reconnectAttempt = 0;
      this.plugins = await this.client.listPlugins();
      this.log.trace(ctx.bus, { msg: 'connected', url: this.url, plugins: this.plugins.length });
      this.setState('connected');
      this.scheduleCheck(ctx, POLL_INTERVAL_MS);
    } else {
      this.log.warn(ctx.bus, { msg: 'offline', url: this.url, attempt: this.reconnectAttempt });
      this.plugins = [];
      this.setState('disconnected');
      this.scheduleReconnect(ctx);
    }
  }
}

interface RegistryHubContext {
  bus: EventBus;
  prefs: PreferencesStore;
}

export class RegistryHub {
  private readonly log = new Logger('registry-hub');
  readonly registryUrls: Set<string> = new Set();
  readonly protectedRegistryUrls: Set<string> = new Set();
  readonly connections: Map<string, Registry> = new Map();

  constructor(urls: string[], protectedUrls: string[]) {
    this.registryUrls = new Set(urls);
    this.protectedRegistryUrls = new Set(protectedUrls);
  }

  static async init(ctx: RegistryHubContext): Promise<RegistryHub> {
    const saved = (await ctx.prefs.get<string[]>(DB_KEY)) ?? [];
    return new RegistryHub(
      [...DEFAULT_REGISTRIES, ...saved],
      DEFAULT_REGISTRIES,
    );
  }

  getRegistry(url: string): Registry | undefined {
    return this.connections.get(url);
  }

  allPlugins(): PluginDescriptor[] {
    return [...this.connections.values()].flatMap((r) => r.getPlugins());
  }

  addUrl(ctx: RegistryHubContext, url: string): void {
    this.registryUrls.add(url);
    this.connectOne(ctx, url);
  }

  removeUrl(ctx: RegistryHubContext, url: string): void {
    if (this.protectedRegistryUrls.has(url)) return;
    this.registryUrls.delete(url);
    this.disconnectOne(ctx, url);
  }

  dispose(): void {
    for (const reg of this.connections.values()) reg.disconnect();
    this.connections.clear();
  }

  private connectAll(ctx: RegistryHubContext): void {
    for (const url of this.registryUrls) this.connectOne(ctx, url);
  }

  private connectOne(ctx: RegistryHubContext, url: string): void {
    if (this.connections.has(url)) return;
    this.log.trace(ctx.bus, { msg: 'connecting', url });
    const reg = new Registry(url);
    this.connections.set(url, reg);
    reg.connect();
    ctx.bus.emit('registry:added', { url }, SOURCE);
  }

  private disconnectOne(ctx: RegistryHubContext, url: string): void {
    const reg = this.connections.get(url);
    if (!reg) return;
    this.log.trace(ctx.bus, { msg: 'disconnecting', url });
    reg.disconnect();
    this.connections.delete(url);
    ctx.bus.emit('registry:removed', { url }, SOURCE);
  }
}
