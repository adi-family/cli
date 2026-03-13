import { AdiPlugin } from '@adi-family/sdk-plugin';
import { AdiRouterBusKey } from '@adi-family/plugin-router';
import { AdiKnowledgebaseBusKey } from './generated/bus/index.js';
import * as api from './api.js';
import { cocoon } from './cocoon.js';
import type { Node, SearchResult, ConflictPair } from './types.js';

const PLUGIN_ID = 'adi.knowledgebase';

export class KnowledgebasePlugin extends AdiPlugin {
  readonly id = PLUGIN_ID;
  readonly version = '0.1.0';

  override async onRegister(): Promise<void> {
    cocoon.init(this.bus);

    const { AdiKnowledgebaseElement } = await import('./component.js');
    if (!customElements.get('adi-knowledgebase')) {
      customElements.define('adi-knowledgebase', AdiKnowledgebaseElement);
    }

    this.bus.emit(
      AdiRouterBusKey.RegisterRoute,
      {
        pluginId: PLUGIN_ID,
        path: '',
        init: () => document.createElement('adi-knowledgebase'),
        label: 'Knowledgebase',
      },
      PLUGIN_ID,
    );
    this.bus.emit(
      'adi.actions-feed:nav-add',
      { id: PLUGIN_ID, label: 'Knowledge', path: `/${PLUGIN_ID}` },
      PLUGIN_ID,
    );

    this.bus.on(
      AdiKnowledgebaseBusKey.Query,
      async ({ q, limit }) => {
        try {
          const conns = cocoon.connectionsWithPlugin('adi.knowledgebase');
          const results = await Promise.allSettled(
            conns.map((c) => api.query(c, q, limit)),
          );
          const allResults: SearchResult[] = results.flatMap((r, i) =>
            r.status === 'fulfilled'
              ? r.value.map((sr) => ({
                  ...sr,
                  node: { ...sr.node, cocoonId: conns[i].id },
                }))
              : [],
          );
          allResults.sort((a, b) => b.score - a.score);
          this.bus.emit(
            'kb:results-changed',
            { results: allResults },
            PLUGIN_ID,
          );
        } catch (err) {
          console.error('[KnowledgebasePlugin] kb:query error:', err);
          this.bus.emit('kb:results-changed', { results: [] }, PLUGIN_ID);
        }
      },
      PLUGIN_ID,
    );

    this.bus.on(
      'kb:add',
      async ({ cocoonId, user_said, derived_knowledge, node_type }) => {
        try {
          const raw = await api.addNode(cocoon.getConnection(cocoonId), {
            user_said,
            derived_knowledge,
            node_type,
          });
          this.bus.emit(
            'kb:node-changed',
            { node: { ...raw, cocoonId }, edges: [] },
            PLUGIN_ID,
          );
        } catch (err) {
          console.error('[KnowledgebasePlugin] kb:add error:', err);
        }
      },
      PLUGIN_ID,
    );

    this.bus.on(
      'kb:get',
      async ({ id, cocoonId }) => {
        try {
          const raw = await api.getNode(cocoon.getConnection(cocoonId), id);
          this.bus.emit(
            'kb:node-changed',
            { node: { ...raw, cocoonId }, edges: [] },
            PLUGIN_ID,
          );
        } catch (err) {
          console.error('[KnowledgebasePlugin] kb:get error:', err);
        }
      },
      PLUGIN_ID,
    );

    this.bus.on(
      'kb:delete',
      async ({ id, cocoonId }) => {
        try {
          await api.deleteNode(cocoon.getConnection(cocoonId), id);
          this.bus.emit('kb:node-deleted', { id, cocoonId }, PLUGIN_ID);
        } catch (err) {
          console.error('[KnowledgebasePlugin] kb:delete error:', err);
        }
      },
      PLUGIN_ID,
    );

    this.bus.on(
      'kb:approve',
      async ({ id, cocoonId }) => {
        try {
          await api.approveNode(cocoon.getConnection(cocoonId), id);
          const raw = await api.getNode(cocoon.getConnection(cocoonId), id);
          this.bus.emit(
            'kb:node-changed',
            { node: { ...raw, cocoonId }, edges: [] },
            PLUGIN_ID,
          );
        } catch (err) {
          console.error('[KnowledgebasePlugin] kb:approve error:', err);
        }
      },
      PLUGIN_ID,
    );

    this.bus.on(
      'kb:conflicts',
      async () => {
        try {
          const conns = cocoon.connectionsWithPlugin('adi.knowledgebase');
          const results = await Promise.allSettled(
            conns.map((c) => api.getConflicts(c)),
          );
          const conflicts: ConflictPair[] = results.flatMap((r, i) =>
            r.status === 'fulfilled'
              ? r.value.map((cp) => ({
                  node_a: { ...cp.node_a, cocoonId: conns[i].id },
                  node_b: { ...cp.node_b, cocoonId: conns[i].id },
                }))
              : [],
          );
          this.bus.emit('kb:conflicts-changed', { conflicts }, PLUGIN_ID);
        } catch (err) {
          console.error('[KnowledgebasePlugin] kb:conflicts error:', err);
          this.bus.emit('kb:conflicts-changed', { conflicts: [] }, PLUGIN_ID);
        }
      },
      PLUGIN_ID,
    );

    this.bus.on(
      'kb:orphans',
      async () => {
        try {
          const conns = cocoon.connectionsWithPlugin('adi.knowledgebase');
          const results = await Promise.allSettled(
            conns.map((c) => api.getOrphans(c)),
          );
          const nodes: Node[] = results.flatMap((r, i) =>
            r.status === 'fulfilled'
              ? r.value.map((n) => ({ ...n, cocoonId: conns[i].id }))
              : [],
          );
          this.bus.emit('kb:orphans-changed', { nodes }, PLUGIN_ID);
        } catch (err) {
          console.error('[KnowledgebasePlugin] kb:orphans error:', err);
          this.bus.emit('kb:orphans-changed', { nodes: [] }, PLUGIN_ID);
        }
      },
      PLUGIN_ID,
    );

    this.bus.on(
      'kb:link',
      async ({ cocoonId, from_id, to_id, edge_type, weight }) => {
        try {
          const edge = await api.addEdge(cocoon.getConnection(cocoonId), {
            from_id,
            to_id,
            edge_type,
            weight,
          });
          const raw = await api.getNode(cocoon.getConnection(cocoonId), from_id);
          this.bus.emit(
            'kb:node-changed',
            { node: { ...raw, cocoonId }, edges: [edge] },
            PLUGIN_ID,
          );
        } catch (err) {
          console.error('[KnowledgebasePlugin] kb:link error:', err);
        }
      },
      PLUGIN_ID,
    );
  }
}
