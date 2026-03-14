import '@adi-family/plugin-cocoon';
import { AdiPlugin } from '@adi-family/sdk-plugin';
import { AdiRouterBusKey } from '@adi-family/plugin-router';
import { NavBusKey } from '@adi-family/plugin-actions-feed';
import { AdiSignalingBusKey, type DeviceInfo, type IceServer } from '@adi-family/plugin-signaling';
import type { Connection } from '@adi-family/cocoon-plugin-interface';
import * as api from './generated/adi-client.js';
import { cocoon } from './cocoon.js';
import {
  AdiLlmProxyBusKey,
  type AdiLlmProxyListKeysEvent,
  type AdiLlmProxyListTokensEvent,
  type AdiLlmProxyCreateKeyEvent,
  type AdiLlmProxyDeleteKeyEvent,
  type AdiLlmProxyVerifyKeyEvent,
  type AdiLlmProxyCreateTokenEvent,
  type AdiLlmProxyDeleteTokenEvent,
  type AdiLlmProxyRotateTokenEvent,
  type AdiLlmProxyQueryUsageEvent,
  type AdiLlmProxyListProvidersEvent,
} from './generated/bus-types.js';
import './generated/bus-events.js';

export class LlmProxyPlugin extends AdiPlugin {
  readonly id = 'adi.llm-proxy';
  readonly version = '0.8.4';

  private readonly devices = new Map<string, { info: DeviceInfo; signalingUrl: string }>();
  private iceServers: IceServer[] | undefined;

  get api() { return api; }

  private async ensureConnection(cocoonId: string): Promise<Connection> {
    try {
      return cocoon.getConnection(cocoonId);
    } catch {
      const tracked = this.devices.get(cocoonId);
      if (!tracked) throw new Error(`Device '${cocoonId}' not found`);
      const cocoonApi = await this.app.api('adi.cocoon');
      const rtcConfig = this.iceServers ? { iceServers: this.iceServers } : undefined;
      await cocoonApi.createClient(cocoonId, tracked.signalingUrl, rtcConfig);
      return cocoon.getConnection(cocoonId);
    }
  }

  private onBus<P>(
    event: AdiLlmProxyBusKey,
    handler: (params: P) => Promise<void>,
  ): void {
    this.bus.on(event, async (params: P) => {
      try {
        await handler(params);
      } catch (err) {
        console.error(`[LlmProxyPlugin] ${event} error:`, err);
        this.bus.emit(AdiLlmProxyBusKey.Error, {
          message: err instanceof Error ? err.message : String(err),
          event,
        }, 'llm-proxy');
      }
    }, 'llm-proxy');
  }

  async onRegister(): Promise<void> {
    cocoon.init(this.bus);
    cocoon.connectProvider = (deviceId: string) => this.ensureConnection(deviceId);

    const { LlmProxyElement } = await import('./component.js');
    if (!customElements.get('adi-llm-proxy')) {
      customElements.define('adi-llm-proxy', LlmProxyElement);
    }

    this.bus.emit(AdiRouterBusKey.RegisterRoute, {
      pluginId: this.id,
      path: '',
      init: () => document.createElement('adi-llm-proxy'),
      label: 'LLM Proxy',
    }, this.id);

    this.bus.emit(NavBusKey.Add, {
      id: this.id,
      label: 'LLM Proxy',
      path: `/${this.id}`,
    }, this.id);

    this.bus.on(AdiSignalingBusKey.ConnectionInfo, ({ connectionInfo }) => {
      this.iceServers = connectionInfo.ice_servers;
    }, this.id);

    this.bus.on(AdiSignalingBusKey.Devices, ({ url, devices }) => {
      for (const d of devices) {
        if (d.device_type === 'cocoon') {
          this.devices.set(d.device_id, { info: d, signalingUrl: url });
        }
      }
    }, this.id);

    // Keys
    this.onBus<AdiLlmProxyListKeysEvent>(AdiLlmProxyBusKey.ListKeys, async ({ cocoonId }) => {
      const conn = this.ensureConnection(cocoonId);
      const keys = await api.listKeys(conn);
      this.bus.emit(AdiLlmProxyBusKey.KeysChanged, {
        keys: keys.map(k => ({ ...k, cocoonId })),
      }, 'llm-proxy');
    });

    this.onBus<AdiLlmProxyCreateKeyEvent>(AdiLlmProxyBusKey.CreateKey, async ({ cocoonId, ...params }) => {
      const conn = this.ensureConnection(cocoonId);
      await api.createKey(conn, params);
      const keys = await api.listKeys(conn);
      this.bus.emit(AdiLlmProxyBusKey.KeysChanged, {
        keys: keys.map(k => ({ ...k, cocoonId })),
      }, 'llm-proxy');
    });

    this.onBus<AdiLlmProxyDeleteKeyEvent>(AdiLlmProxyBusKey.DeleteKey, async ({ cocoonId, id }) => {
      const conn = this.ensureConnection(cocoonId);
      await api.deleteKey(conn, id);
      this.bus.emit(AdiLlmProxyBusKey.KeyDeleted, { id, cocoonId }, 'llm-proxy');
    });

    this.onBus<AdiLlmProxyVerifyKeyEvent>(AdiLlmProxyBusKey.VerifyKey, async ({ cocoonId, id }) => {
      const conn = this.ensureConnection(cocoonId);
      const result = await api.verifyKey(conn, id);
      this.bus.emit(AdiLlmProxyBusKey.KeyVerified, { id, cocoonId, ...result }, 'llm-proxy');
    });

    // Tokens
    this.onBus<AdiLlmProxyListTokensEvent>(AdiLlmProxyBusKey.ListTokens, async ({ cocoonId }) => {
      const conn = this.ensureConnection(cocoonId);
      const tokens = await api.listTokens(conn);
      this.bus.emit(AdiLlmProxyBusKey.TokensChanged, {
        tokens: tokens.map(t => ({ ...t, cocoonId })),
      }, 'llm-proxy');
    });

    this.onBus<AdiLlmProxyCreateTokenEvent>(AdiLlmProxyBusKey.CreateToken, async ({ cocoonId, ...params }) => {
      const conn = this.ensureConnection(cocoonId);
      const result = await api.createToken(conn, params);
      this.bus.emit(AdiLlmProxyBusKey.TokenCreated, {
        token: { ...result.token, cocoonId },
        secret: result.secret,
      }, 'llm-proxy');
    });

    this.onBus<AdiLlmProxyDeleteTokenEvent>(AdiLlmProxyBusKey.DeleteToken, async ({ cocoonId, id }) => {
      const conn = this.ensureConnection(cocoonId);
      await api.deleteToken(conn, id);
      this.bus.emit(AdiLlmProxyBusKey.TokenDeleted, { id, cocoonId }, 'llm-proxy');
    });

    this.onBus<AdiLlmProxyRotateTokenEvent>(AdiLlmProxyBusKey.RotateToken, async ({ cocoonId, id }) => {
      const conn = this.ensureConnection(cocoonId);
      const result = await api.rotateToken(conn, id);
      this.bus.emit(AdiLlmProxyBusKey.TokenRotated, {
        token: { ...result.token, cocoonId },
        secret: result.secret,
      }, 'llm-proxy');
    });

    // Usage
    this.onBus<AdiLlmProxyQueryUsageEvent>(AdiLlmProxyBusKey.QueryUsage, async ({ cocoonId, ...params }) => {
      const conn = this.ensureConnection(cocoonId);
      const result = await api.queryUsage(conn, params);
      this.bus.emit(AdiLlmProxyBusKey.UsageLoaded, {
        logs: result.logs,
        total: result.total,
        cocoonId,
      }, 'llm-proxy');
    });

    // Providers
    this.onBus<AdiLlmProxyListProvidersEvent>(AdiLlmProxyBusKey.ListProviders, async ({ cocoonId }) => {
      const conn = this.ensureConnection(cocoonId);
      const providers = await api.listProviders(conn);
      this.bus.emit(AdiLlmProxyBusKey.ProvidersChanged, { providers, cocoonId }, 'llm-proxy');
    });
  }
}
