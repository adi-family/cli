import { Logger, trace, type EventBus, type PluginStorage } from '@adi-family/sdk-plugin';
import { SignalingServer, type TokenGetter } from './signaling-server';

export { SignalingServer };

const URLS_KEY = 'server-urls';

export class SignalingHub {
  private readonly log = new Logger('signaling-hub', () => ({
    servers: this.servers.size,
    started: this.started,
  }));
  private readonly servers = new Map<string, SignalingServer>();
  private readonly protectedUrls: ReadonlySet<string>;
  private readonly bus: EventBus;
  private readonly getToken: TokenGetter;
  private readonly storage: PluginStorage;
  private started = false;

  private constructor(bus: EventBus, protectedUrls: string[], getToken: TokenGetter, storage: PluginStorage) {
    this.bus = bus;
    this.protectedUrls = new Set(protectedUrls);
    this.getToken = getToken;
    this.storage = storage;
  }

  static init(bus: EventBus, protectedUrls: string[] = [], getToken: TokenGetter, storage: PluginStorage): SignalingHub {
    return new SignalingHub(bus, protectedUrls, getToken, storage);
  }

  @trace('starting')
  async start(): Promise<void> {
    this.started = true;
    const saved = await this.loadUrls();
    const urls = saved.length > 0 ? saved : [...this.protectedUrls];
    for (const url of urls) this.addServer(url);
  }

  allServers(): ReadonlyMap<string, SignalingServer> {
    return this.servers;
  }

  getServer(url: string): SignalingServer | undefined {
    return this.servers.get(url);
  }

  @trace('adding server')
  addServer(url: string): SignalingServer {
    const existing = this.servers.get(url);
    if (existing) return existing;

    const server = new SignalingServer(url, this.bus, () => this.started, this.getToken);
    this.servers.set(url, server);
    server.connect();
    void this.persist();
    return server;
  }

  @trace('removing server')
  removeServer(url: string): void {
    if (this.protectedUrls.has(url)) return;
    const server = this.servers.get(url);
    if (!server) return;

    server.disconnect();
    this.servers.delete(url);
    void this.persist();
  }

  @trace('disposing')
  dispose(): void {
    for (const server of this.servers.values()) server.disconnect();
    this.servers.clear();
  }

  private async loadUrls(): Promise<string[]> {
    try {
      return (await this.storage.get<string[]>(URLS_KEY)) ?? [];
    } catch {
      return [];
    }
  }

  private async persist(): Promise<void> {
    try {
      await this.storage.set(URLS_KEY, [...this.servers.keys()]);
    } catch (err) {
      this.log.warn({ msg: 'persist failed', error: String(err) });
    }
  }
}
