import { AdiPlugin } from '@adi-family/sdk-plugin';
import { AdiDebugScreenBusKey } from '@adi-family/plugin-debug-screen';
import { SignalingHub } from './signaling-hub';
import { SignalingServer } from './signaling-server';
import { AdiSignalingBusKey } from './generated';
import type { DeviceInfo } from './generated';
import { PLUGIN_ID, PLUGIN_VERSION } from './config';
import type { AdiSignalingDebugElement, SignalingServerDebugInfo } from './debug-section';

export interface SignalingApi {
  readonly hub: SignalingHub;
  getServer(url: string): SignalingServer | undefined;
  allServers(): ReadonlyMap<string, SignalingServer>;
  allDevices(): readonly DeviceInfo[];
  addServer(url: string): SignalingServer;
  removeServer(url: string): void;
}

export class SignalingPlugin extends AdiPlugin implements SignalingApi {
  readonly id = PLUGIN_ID;
  readonly version = PLUGIN_VERSION;
  hub!: SignalingHub;
  private debugEl: AdiSignalingDebugElement | null = null;
  private readonly debugUnsubs: (() => void)[] = [];

  private authApi() {
    return this.app.api('adi.auth');
  }

  get api(): SignalingApi {
    return this;
  }

  getServer(url: string): SignalingServer | undefined {
    return this.hub.getServer(url);
  }

  allServers(): ReadonlyMap<string, SignalingServer> {
    return this.hub.allServers();
  }

  addServer(url: string): SignalingServer {
    return this.hub.addServer(url);
  }

  removeServer(url: string): void {
    this.hub.removeServer(url);
  }

  allDevices(): readonly DeviceInfo[] {
    return [...this.hub.allServers().values()]
      .flatMap(server => server.getDevices());
  }

  override async onRegister(): Promise<void> {
    this.hub = SignalingHub.init(
      this.bus,
      this.app.env('DEFAULT_SIGNALING_URLS'),
      async (domain) => {
        const auth = await this.authApi();
        return auth.getToken(domain);
      },
      this.app.storage(this.id),
    );
    await this.hub.start();

    await import('./debug-section.js');
    this.bus.emit(
      AdiDebugScreenBusKey.RegisterSection,
      {
        pluginId: PLUGIN_ID,
        init: () => {
          this.debugEl = document.createElement('adi-signaling-debug') as AdiSignalingDebugElement;
          this.syncDebug();
          return this.debugEl;
        },
        label: 'Signaling',
      },
      PLUGIN_ID,
    );

    this.debugUnsubs.push(
      this.bus.on(AdiSignalingBusKey.State, () => this.syncDebug(), PLUGIN_ID),
      this.bus.on(AdiSignalingBusKey.AuthOk, () => this.syncDebug(), PLUGIN_ID),
      this.bus.on(AdiSignalingBusKey.DeviceRegistered, () => this.syncDebug(), PLUGIN_ID),
      this.bus.on(AdiSignalingBusKey.DeviceDeregistered, () => this.syncDebug(), PLUGIN_ID),
      this.bus.on(AdiSignalingBusKey.PeerConnected, () => this.syncDebug(), PLUGIN_ID),
      this.bus.on(AdiSignalingBusKey.PeerDisconnected, () => this.syncDebug(), PLUGIN_ID),
      this.bus.on(AdiSignalingBusKey.Devices, () => this.syncDebug(), PLUGIN_ID),
      this.bus.on(AdiSignalingBusKey.RoomUpdated, () => this.syncDebug(), PLUGIN_ID),
      this.bus.on(AdiSignalingBusKey.RoomActorJoined, () => this.syncDebug(), PLUGIN_ID),
      this.bus.on(AdiSignalingBusKey.RoomActorLeft, () => this.syncDebug(), PLUGIN_ID),
    );
  }

  override onUnregister(): void {
    this.debugUnsubs.forEach((fn) => fn());
    this.debugUnsubs.length = 0;
    this.hub.dispose();
  }

  private syncDebug(): void {
    if (!this.debugEl) return;
    const infos: SignalingServerDebugInfo[] = [];
    for (const [url, server] of this.hub.allServers()) {
      infos.push({
        url,
        state: server.getState(),
        authenticated: server.isAuthenticated(),
        userId: server.getUserId(),
        deviceId: server.getDeviceId(),
        peers: [...server.getPeers()],
        devices: [...server.getDevices()],
        rooms: [...server.getRooms().values()],
      });
    }
    this.debugEl.servers = infos;
  }
}
