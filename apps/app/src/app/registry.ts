import {
  HttpPluginRegistry,
  type EventBus,
  type PluginDescriptor,
  type RegistryHealth,
} from '@adi-family/sdk-plugin';
import { DEFAULT_REGISTRIES } from './env';
import { get, put } from './database';
import { getBus } from './bus';

const DB_KEY = 'registry-urls';
const SOURCE = 'registry';

const POLL_INTERVAL_MS = 60_000;
const RECONNECT_BASE_MS = 2_000;
const RECONNECT_CAP_MS = 30_000;

export type RegistryState = 'disconnected' | 'connecting' | 'connected';

type RegistryListener = (state: RegistryState) => void;

async function loadUrls(): Promise<string[]> {
  const saved = (await get<string[]>('prefs', DB_KEY)) ?? [];
  return [...new Set([...saved, ...DEFAULT_REGISTRIES])];
}

async function saveUrls(urls: Set<string>): Promise<void> {
  await put('prefs', DB_KEY, Array.from(urls));
}

/** Single registry connection with health polling and automatic reconnect. */
export class Registry {
  readonly url: string;

  private readonly client: HttpPluginRegistry;
  private readonly bus: EventBus;
  private state: RegistryState = 'disconnected';
  private health: RegistryHealth | null = null;
  private plugins: PluginDescriptor[] = [];
  private pollTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectAttempt = 0;
  private disposed = false;
  private readonly listeners: Set<RegistryListener> = new Set();

  constructor(url: string, bus: EventBus = getBus()) {
    this.url = url;
    this.client = new HttpPluginRegistry(url);
    this.bus = bus;
  }

  getState(): RegistryState { return this.state; }

  getHealth(): RegistryHealth | null { return this.health; }

  getPlugins(): readonly PluginDescriptor[] { return this.plugins; }

  /** Subscribe to state changes. Returns unsubscribe function. */
  onChange(listener: RegistryListener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  /** Start connecting, polling, and auto-reconnecting. */
  connect(): void {
    if (this.disposed) return;
    this.scheduleCheck(0);
  }

  /** Stop all timers and mark disposed. */
  disconnect(): void {
    this.disposed = true;
    this.clearTimers();
    this.setState('disconnected');
  }

  /** Check for a newer version of a specific plugin. */
  async checkLatest(id: string, currentVersion: string): Promise<{ version: string } | null> {
    return this.client.checkLatest(id, currentVersion);
  }

  /** Get the bundle URL for a plugin version. */
  async bundleUrl(id: string, version: string): Promise<string> {
    return this.client.bundleUrl(id, version);
  }

  private setState(next: RegistryState): void {
    if (this.state === next) return;
    this.state = next;
    this.listeners.forEach((fn) => fn(next));
  }

  private clearTimers(): void {
    if (this.pollTimer) { clearTimeout(this.pollTimer); this.pollTimer = null; }
    if (this.reconnectTimer) { clearTimeout(this.reconnectTimer); this.reconnectTimer = null; }
  }

  private scheduleCheck(delayMs: number): void {
    this.clearTimers();
    if (this.disposed) return;
    this.pollTimer = setTimeout(() => this.poll(), delayMs);
  }

  private scheduleReconnect(): void {
    if (this.disposed) return;
    const jitter = Math.random() * 500;
    const delay = Math.min(RECONNECT_BASE_MS * 2 ** this.reconnectAttempt, RECONNECT_CAP_MS) + jitter;
    this.reconnectAttempt++;
    this.reconnectTimer = setTimeout(() => this.poll(), delay);
  }

  private async poll(): Promise<void> {
    if (this.disposed) return;
    this.setState('connecting');

    const health = await this.client.checkHealth();
    if (this.disposed) return;

    this.health = health;
    this.bus.emit('registry:health', { url: this.url, health }, SOURCE);

    if (health.online) {
      this.reconnectAttempt = 0;
      this.plugins = await this.client.listPlugins();
      this.setState('connected');
      this.scheduleCheck(POLL_INTERVAL_MS);
    } else {
      this.plugins = [];
      this.setState('disconnected');
      this.scheduleReconnect();
    }
  }
}

export class RegistryHub {
  readonly registries: Set<string> = new Set();
  readonly protectedRegistries: Set<string> = new Set();
  readonly connections: Map<string, Registry> = new Map();
  private readonly bus: EventBus;

  constructor(regs: string[], prot: string[], bus: EventBus = getBus()) {
    this.bus = bus;
    this.registries = new Set(regs);
    this.protectedRegistries = new Set(prot);

    for (const url of this.registries) {
      this.connectRegistry(url);
    }
  }

  static async init(): Promise<RegistryHub> {
    const urls = await loadUrls();

    return new RegistryHub(
      [...DEFAULT_REGISTRIES, ...urls],
      DEFAULT_REGISTRIES,
    );
  }

  /** Get the Registry instance for a URL, if connected. */
  getRegistry(url: string): Registry | undefined {
    return this.connections.get(url);
  }

  /** Aggregated plugin list across all connected registries. */
  allPlugins(): PluginDescriptor[] {
    return [...this.connections.values()].flatMap((r) => r.getPlugins());
  }

  addUrls(...urls: string[]) {
    urls.forEach((v) => this.addUrl(v));
  }

  addUrl(url: string) {
    this.registries.add(url);
    this.connectRegistry(url);
    saveUrls(this.registries);
  }

  removeUrl(url: string) {
    if (this.protectedRegistries.has(url)) return;
    this.registries.delete(url);
    this.disconnectRegistry(url);
    saveUrls(this.registries);
  }

  /** Disconnect all registries and clean up. */
  dispose(): void {
    for (const reg of this.connections.values()) reg.disconnect();
    this.connections.clear();
  }

  private connectRegistry(url: string): void {
    if (this.connections.has(url)) return;
    const reg = new Registry(url, this.bus);
    this.connections.set(url, reg);
    reg.connect();
    this.bus.emit('registry:added', { url }, SOURCE);
  }

  private disconnectRegistry(url: string): void {
    const reg = this.connections.get(url);
    if (!reg) return;
    reg.disconnect();
    this.connections.delete(url);
    this.bus.emit('registry:removed', { url }, SOURCE);
  }
}
