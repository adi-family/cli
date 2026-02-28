import { Logger, type EventBus } from '@adi-family/sdk-plugin';
import type { WsState, CocoonInfo, HiveInfo } from '../services/signaling';
import type { Connection } from '../services/signaling';
import {
  createSignalingManager,
  type SignalingManager,
} from '../services/signaling';
import { DEFAULT_SIGNALING_SERVERS } from './env';
import type { PreferencesStore } from './database';

const DB_KEY = 'signaling-urls';
const SOURCE = 'signaling';

export type SignalingServerState = WsState;

type ServerListener = (state: SignalingServerState) => void;

export class SignalingServer {
  readonly url: string;

  private readonly log = new Logger('signaling-server');
  private readonly manager: SignalingManager;
  private state: SignalingServerState = 'disconnected';
  private cocoons: CocoonInfo[] = [];
  private hives: HiveInfo[] = [];
  private disposed = false;
  private readonly listeners: Set<ServerListener> = new Set();
  private readonly unsubscribers: (() => void)[] = [];

  private readonly bus: EventBus;

  constructor(url: string, bus: EventBus, sharedConnections: Map<string, Connection>) {
    this.url = url;
    this.bus = bus;
    this.manager = createSignalingManager(url, sharedConnections, bus);

    this.unsubscribers.push(
      bus.on(
        'signaling:state',
        ({ url: u, state }) => {
          if (u !== url) return;
          this.state = state;
          this.listeners.forEach((fn) => fn(state));
        },
        SOURCE,
      ),
      bus.on(
        'signaling:cocoons',
        ({ url: u, cocoons }) => {
          if (u !== url) return;
          this.cocoons = cocoons;
        },
        SOURCE,
      ),
      bus.on(
        'signaling:hives',
        ({ url: u, hives }) => {
          if (u !== url) return;
          this.hives = hives;
        },
        SOURCE,
      ),
    );
  }

  getState(): SignalingServerState {
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

  onChange(listener: ServerListener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  connect(): void {
    if (this.disposed) return;
    this.log.trace(this.bus, { msg: 'connecting', url: this.url });
    this.manager.connect();
  }

  disconnect(): void {
    this.log.trace(this.bus, { msg: 'disconnecting', url: this.url });
    this.disposed = true;
    this.manager.disconnect();
    this.unsubscribers.forEach((fn) => fn());
    this.listeners.clear();
  }
}

interface SignalingHubContext {
  bus: EventBus;
  prefs: PreferencesStore;
}

export class SignalingServerHub {
  private readonly log = new Logger('signaling-hub');
  readonly servers: Set<string> = new Set();
  readonly protectedServers: Set<string> = new Set();
  readonly connections: Map<string, SignalingServer> = new Map();
  private readonly ctx: SignalingHubContext;
  private readonly sharedConnections: Map<string, Connection>;

  constructor(
    urls: string[],
    protectedUrls: string[],
    ctx: SignalingHubContext,
    sharedConnections: Map<string, Connection>,
  ) {
    this.ctx = ctx;
    this.sharedConnections = sharedConnections;
    this.servers = new Set(urls);
    this.protectedServers = new Set(protectedUrls);
  }

  static async init(
    ctx: SignalingHubContext,
    sharedConnections: Map<string, Connection>,
  ): Promise<SignalingServerHub> {
    const saved = (await ctx.prefs.get<string[]>(DB_KEY)) ?? [];
    const hub = new SignalingServerHub(
      [...DEFAULT_SIGNALING_SERVERS, ...saved],
      DEFAULT_SIGNALING_SERVERS,
      ctx,
      sharedConnections,
    );
    hub.connectAll();
    return hub;
  }

  getServer(url: string): SignalingServer | undefined {
    return this.connections.get(url);
  }

  allCocoons(): CocoonInfo[] {
    return [...this.connections.values()].flatMap((s) => [...s.getCocoons()]);
  }

  allHives(): HiveInfo[] {
    return [...this.connections.values()].flatMap((s) => [...s.getHives()]);
  }

  addUrl(url: string): void {
    this.servers.add(url);
    this.connectOne(url);
  }

  removeUrl(url: string): void {
    if (this.protectedServers.has(url)) return;
    this.servers.delete(url);
    this.disconnectOne(url);
  }

  dispose(): void {
    for (const server of this.connections.values()) server.disconnect();
    this.connections.clear();
  }

  private connectAll(): void {
    for (const url of this.servers) this.connectOne(url);
  }

  private connectOne(url: string): void {
    if (this.connections.has(url)) return;
    this.log.trace(this.ctx.bus, { msg: 'connecting', url });
    const server = new SignalingServer(url, this.ctx.bus, this.sharedConnections);
    this.connections.set(url, server);
    server.connect();
    this.ctx.bus.emit(
      'signaling:server-added' as never,
      { url } as never,
      SOURCE,
    );
  }

  private disconnectOne(url: string): void {
    const server = this.connections.get(url);
    if (!server) return;
    this.log.trace(this.ctx.bus, { msg: 'disconnecting', url });
    server.disconnect();
    this.connections.delete(url);
    this.ctx.bus.emit(
      'signaling:server-removed' as never,
      { url } as never,
      SOURCE,
    );
  }
}
