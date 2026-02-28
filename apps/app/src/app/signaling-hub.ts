import { Logger, type EventBus } from '@adi-family/sdk-plugin';
import type { WsState, CocoonInfo, HiveInfo } from '../services/signaling';
import type { Connection } from '../services/signaling';
import { SignalingManager } from '../services/signaling';
import { DEFAULT_SIGNALING_SERVERS } from './env';
import type { Context } from './app';

const DB_NAME = 'adi-app';
const DB_VERSION = 1;
const STORE = 'prefs';
const DB_KEY = 'signaling-urls';

export class SignalingServer {
  readonly url: string;

  private readonly log = new Logger('signaling-server');
  private readonly manager: SignalingManager;
  private state: WsState = 'disconnected';
  private cocoons: CocoonInfo[] = [];
  private hives: HiveInfo[] = [];
  private disposed = false;
  private readonly unsubscribers: (() => void)[] = [];

  constructor(
    url: string,
    bus: EventBus,
    connections: Map<string, Connection>,
  ) {
    this.url = url;
    this.manager = new SignalingManager(url, connections, bus);

    this.unsubscribers.push(
      bus.on(
        'signaling:state',
        ({ url: u, state }) => {
          if (u !== url) return;
          this.state = state;
        },
        'signaling-server',
      ),
      bus.on(
        'signaling:cocoons',
        ({ url: u, cocoons }) => {
          if (u !== url) return;
          this.cocoons = cocoons;
        },
        'signaling-server',
      ),
      bus.on(
        'signaling:hives',
        ({ url: u, hives }) => {
          if (u !== url) return;
          this.hives = hives;
        },
        'signaling-server',
      ),
    );
  }

  getState(): WsState {
    return this.state;
  }

  getCocoons(): readonly CocoonInfo[] {
    return this.cocoons;
  }

  getHives(): readonly HiveInfo[] {
    return this.hives;
  }

  getManager(): SignalingManager {
    return this.manager;
  }

  connect(): void {
    if (this.disposed) return;
    this.log.trace({ msg: 'connecting', url: this.url });
    this.manager.connect();
  }

  disconnect(): void {
    this.log.trace({ msg: 'disconnecting', url: this.url });
    this.disposed = true;
    this.manager.disconnect();
    this.unsubscribers.forEach((fn) => fn());
    this.unsubscribers.length = 0;
  }
}

export class SignalingHub {
  private readonly log = new Logger('signaling-hub');
  private readonly servers = new Map<string, SignalingServer>();
  private readonly connections = new Map<string, Connection>();
  private readonly protectedUrls: ReadonlySet<string>;

  private constructor(protectedUrls: string[]) {
    this.protectedUrls = new Set(protectedUrls);
  }

  static async init(ctx: Context): Promise<SignalingHub> {
    const hub = new SignalingHub(DEFAULT_SIGNALING_SERVERS);
    const saved = await hub.loadUrls(ctx);
    const urls = saved.length > 0 ? saved : DEFAULT_SIGNALING_SERVERS;
    for (const url of urls) hub.connectOne(ctx, url);
    return hub;
  }

  getServer(url: string): SignalingServer | undefined {
    return this.servers.get(url);
  }

  allServers(): ReadonlySet<SignalingServer> {
    return new Set(this.servers.values());
  }

  getConnection(deviceId: string): Connection | undefined {
    return this.connections.get(deviceId);
  }

  addUrl(ctx: Context, url: string): void {
    this.connectOne(ctx, url);
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

  private connectOne(ctx: Context, url: string): void {
    if (this.servers.has(url)) return;
    this.log.trace({ msg: 'connecting', url });
    const server = new SignalingServer(url, ctx.bus, this.connections);
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
