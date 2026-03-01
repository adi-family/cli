import { type HttpPluginRegistry, Logger } from '@adi-family/sdk-plugin';
import { RegistryServer } from './registry-server';
import { DEFAULT_REGISTRIES } from './env';
import type { Context } from './app';

export { RegistryServer };

const DB_NAME = 'adi-app';
const DB_VERSION = 1;
const STORE = 'prefs';
const DB_KEY = 'registry-urls';

export class RegistryHub {
  private readonly log = new Logger('registry-hub');
  private readonly servers = new Map<string, RegistryServer>();
  private readonly registries = new Map<string, HttpPluginRegistry>();
  private readonly protectedUrls: ReadonlySet<string>;
  private readonly ctx: Context;

  private constructor(ctx: Context, protectedUrls: string[]) {
    this.ctx = ctx;
    this.protectedUrls = new Set(protectedUrls);
  }

  static async init(ctx: Context): Promise<RegistryHub> {
    const hub = new RegistryHub(ctx, DEFAULT_REGISTRIES);
    const saved = await hub.loadUrls();
    const urls = saved.length > 0 ? saved : DEFAULT_REGISTRIES;
    for (const url of urls) hub.addRegistry(url);
    return hub;
  }

  allRegistries(): ReadonlyMap<string, HttpPluginRegistry> {
    return this.registries;
  }

  getServer(url: string): RegistryServer | undefined {
    return this.servers.get(url);
  }

  addRegistry(url: string): HttpPluginRegistry {
    const existing = this.servers.get(url);
    if (existing) return existing.getClient();

    this.log.trace({ msg: 'connecting', url });
    const server = new RegistryServer(url);
    this.servers.set(url, server);
    this.registries.set(url, server.getClient());
    server.connect();
    void this.persist();
    return server.getClient();
  }

  removeRegistry(url: string): void {
    if (this.protectedUrls.has(url)) return;
    const server = this.servers.get(url);
    if (!server) return;

    this.log.trace({ msg: 'disconnecting', url });
    server.disconnect();
    this.servers.delete(url);
    this.registries.delete(url);
    void this.persist();
  }

  dispose(): void {
    for (const server of this.servers.values()) server.disconnect();
    this.servers.clear();
    this.registries.clear();
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
