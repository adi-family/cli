import '@adi-family/plugin-cocoon';
import { AdiPlugin } from '@adi-family/sdk-plugin';
import { AdiRouterBusKey } from '@adi-family/plugin-router';
import { NavBusKey } from '@adi-family/plugin-actions-feed';
import { AdiSignalingBusKey, type DeviceInfo, type IceServer } from '@adi-family/plugin-signaling';
import type { Connection } from '@adi-family/cocoon-plugin-interface';
import * as api from './generated/adi-client.js';
import { cocoon } from './cocoon.js';
import {
  AdiKnowledgebaseBusKey,
  type AdiKnowledgebaseListNodesEvent,
  type AdiKnowledgebaseGetNodeEvent,
  type AdiKnowledgebaseSearchEvent,
  type AdiKnowledgebaseApproveNodeEvent,
  type AdiKnowledgebaseRejectNodeEvent,
  type AdiKnowledgebaseDeleteNodeEvent,
  type AdiKnowledgebaseGetConflictsEvent,
  type AdiKnowledgebaseGetOrphansEvent,
  type AdiKnowledgebaseGetStatsEvent,
} from './generated/bus-types.js';
import './generated/bus-events.js';

export class KnowledgebasePlugin extends AdiPlugin {
  readonly id = 'adi.knowledgebase';
  readonly version = '0.1.0';

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
    event: AdiKnowledgebaseBusKey,
    handler: (params: P) => Promise<void>,
  ): void {
    this.bus.on(event, async (params: P) => {
      try {
        await handler(params);
      } catch (err) {
        console.error(`[KnowledgebasePlugin] ${event} error:`, err);
        this.bus.emit(AdiKnowledgebaseBusKey.Error, {
          message: err instanceof Error ? err.message : String(err),
          event,
        }, 'knowledgebase');
      }
    }, 'knowledgebase');
  }

  async onRegister(): Promise<void> {
    cocoon.init(this.bus);
    cocoon.connectProvider = (deviceId: string) => this.ensureConnection(deviceId);

    const { AdiKnowledgebaseElement } = await import('./component.js');
    if (!customElements.get('adi-knowledgebase')) {
      customElements.define('adi-knowledgebase', AdiKnowledgebaseElement);
    }

    this.bus.emit(AdiRouterBusKey.RegisterRoute, {
      pluginId: this.id,
      path: '',
      init: () => document.createElement('adi-knowledgebase'),
      label: 'Knowledgebase',
    }, this.id);

    this.bus.emit(NavBusKey.Add, {
      id: this.id,
      label: 'Knowledgebase',
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

    this.onBus<AdiKnowledgebaseListNodesEvent>(AdiKnowledgebaseBusKey.ListNodes, async ({ node_type, approval_status, source }) => {
      const conns = cocoon.connectionsWithPlugin('adi.knowledgebase');
      const results = await Promise.allSettled(
        conns.map(c => api.listNodes(c, { node_type, approval_status, source })),
      );
      const nodes = results.flatMap((r, i) =>
        r.status === 'fulfilled'
          ? r.value.map(node => ({ ...node, cocoonId: conns[i].id }))
          : [],
      );
      this.bus.emit(AdiKnowledgebaseBusKey.NodesChanged, { nodes }, 'knowledgebase');
    });

    this.onBus<AdiKnowledgebaseGetNodeEvent>(AdiKnowledgebaseBusKey.GetNode, async ({ id, cocoonId }) => {
      const node = await api.getNode(this.ensureConnection(cocoonId), id);
      this.bus.emit(AdiKnowledgebaseBusKey.NodeDetail, { node: { ...node, cocoonId } }, 'knowledgebase');
    });

    this.onBus<AdiKnowledgebaseSearchEvent>(AdiKnowledgebaseBusKey.Search, async ({ cocoonId, query, limit }) => {
      const results = await api.search(this.ensureConnection(cocoonId), { query, limit });
      this.bus.emit(AdiKnowledgebaseBusKey.SearchResults, { results, cocoonId }, 'knowledgebase');
    });

    this.onBus<AdiKnowledgebaseApproveNodeEvent>(AdiKnowledgebaseBusKey.ApproveNode, async ({ id, cocoonId }) => {
      const node = await api.approveNode(this.ensureConnection(cocoonId), id);
      this.bus.emit(AdiKnowledgebaseBusKey.NodeMutated, { node: { ...node, cocoonId } }, 'knowledgebase');
    });

    this.onBus<AdiKnowledgebaseRejectNodeEvent>(AdiKnowledgebaseBusKey.RejectNode, async ({ id, cocoonId, reason }) => {
      const node = await api.rejectNode(this.ensureConnection(cocoonId), { id, reason });
      this.bus.emit(AdiKnowledgebaseBusKey.NodeMutated, { node: { ...node, cocoonId } }, 'knowledgebase');
    });

    this.onBus<AdiKnowledgebaseDeleteNodeEvent>(AdiKnowledgebaseBusKey.DeleteNode, async ({ id, cocoonId }) => {
      await api.deleteNode(this.ensureConnection(cocoonId), id);
      this.bus.emit(AdiKnowledgebaseBusKey.NodeDeleted, { id, cocoonId }, 'knowledgebase');
    });

    this.onBus<AdiKnowledgebaseGetConflictsEvent>(AdiKnowledgebaseBusKey.GetConflicts, async ({ cocoonId }) => {
      const conflicts = await api.getConflicts(this.ensureConnection(cocoonId));
      this.bus.emit(AdiKnowledgebaseBusKey.ConflictsChanged, { conflicts, cocoonId }, 'knowledgebase');
    });

    this.onBus<AdiKnowledgebaseGetOrphansEvent>(AdiKnowledgebaseBusKey.GetOrphans, async ({ cocoonId }) => {
      const orphans = await api.getOrphans(this.ensureConnection(cocoonId));
      const orphansWithCocoon = orphans.map(n => ({ ...n, cocoonId }));
      this.bus.emit(AdiKnowledgebaseBusKey.OrphansChanged, { orphans: orphansWithCocoon }, 'knowledgebase');
    });

    this.onBus<AdiKnowledgebaseGetStatsEvent>(AdiKnowledgebaseBusKey.GetStats, async ({ cocoonId }) => {
      const stats = await api.getStats(this.ensureConnection(cocoonId));
      this.bus.emit(AdiKnowledgebaseBusKey.StatsChanged, { stats, cocoonId }, 'knowledgebase');
    });
  }
}
