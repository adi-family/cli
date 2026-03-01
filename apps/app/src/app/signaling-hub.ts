import { Logger, type EventBus } from '@adi-family/sdk-plugin';
import type { Connection } from '../services/signaling/connection.ts';
import { SignalingServer } from './signaling-server';
import { DEFAULT_SIGNALING_SERVERS } from './env';
import type { Context } from './app';

export { SignalingServer };

const DB_NAME = 'adi-app';
const DB_VERSION = 1;
const STORE = 'prefs';
const DB_KEY = 'signaling-urls';

export class SignalingHub {
  private readonly log = new Logger('signaling-hub');
  private readonly servers = new Map<string, SignalingServer>();
  private readonly connections = new Map<string, Connection>();
  private readonly protectedUrls: ReadonlySet<string>;
  private readonly bus: EventBus;
  private ctx!: Context;
  private started = false;

  private constructor(bus: EventBus, protectedUrls: string[]) {
    this.bus = bus;
    this.protectedUrls = new Set(protectedUrls);
  }

  static init(bus: EventBus): SignalingHub {
    return new SignalingHub(bus, DEFAULT_SIGNALING_SERVERS);
  }

  async start(ctx: Context): Promise<void> {
    this.ctx = ctx;
    this.started = true;
    const saved = await this.loadUrls();
    const urls = saved.length > 0 ? saved : DEFAULT_SIGNALING_SERVERS;
    for (const url of urls) this.addServer(url);
  }

  allServers(): ReadonlyMap<string, SignalingServer> {
    return this.servers;
  }

  getServer(url: string): SignalingServer | undefined {
    return this.servers.get(url);
  }

  getConnection(deviceId: string): Connection | undefined {
    return this.connections.get(deviceId);
  }

  addServer(url: string): SignalingServer {
    const existing = this.servers.get(url);
    if (existing) return existing;

    this.log.trace({ msg: 'connecting', url });
    const server = new SignalingServer(url, this.connections, this.bus, () => this.started);
    this.servers.set(url, server);
    server.connect();
    void this.persist();
    return server;
  }

  removeServer(url: string): void {
    if (this.protectedUrls.has(url)) return;
    const server = this.servers.get(url);
    if (!server) return;

    this.log.trace({ msg: 'disconnecting', url });
    server.disconnect();
    this.servers.delete(url);
    void this.persist();
  }

  dispose(): void {
    for (const server of this.servers.values()) server.disconnect();
    this.servers.clear();
  }

  private async loadUrls(): Promise<string[]> {
    try {
      const db = await this.ctx.db.open(DB_NAME, DB_VERSION);
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

  private async persist(): Promise<void> {
    try {
      const db = await this.ctx.db.open(DB_NAME, DB_VERSION);
      const tx = db.transaction(STORE, 'readwrite');
      tx.objectStore(STORE).put([...this.servers.keys()], DB_KEY);
    } catch (err) {
      this.log.warn({ msg: 'persist failed', error: String(err) });
    }
  }
}
