import { AdiPlugin } from '@adi-family/sdk-plugin';
import type { WithCid } from '@adi-family/sdk-plugin';
import * as api from './api.js';
import type { Connection, Node, SearchResult, ConflictPair } from './types.js';
import './events.js';

function connectionsWithKb(): Connection[] {
  return [...window.sdk.getConnections().values()]
    .filter(c => c.services.includes('kb'));
}

function getConnection(cocoonId: string): Connection {
  const c = window.sdk.getConnections().get(cocoonId);
  if (!c) throw new Error(`Connection '${cocoonId}' not found`);
  return c;
}

export class KnowledgebasePlugin extends AdiPlugin {
  readonly id = 'adi.knowledgebase';
  readonly version = '0.1.0';

  async onRegister(): Promise<void> {
    const { AdiKnowledgebaseElement } = await import('./component.js');
    if (!customElements.get('adi-knowledgebase')) {
      customElements.define('adi-knowledgebase', AdiKnowledgebaseElement);
    }

    this.bus.emit('route:register', { path: '/knowledgebase', element: 'adi-knowledgebase' }, 'knowledgebase');
    this.bus.send('nav:add', { id: 'knowledgebase', label: 'Knowledge', path: '/knowledgebase' }, 'knowledgebase').handle(() => {});

    this.bus.emit('command:register', { id: 'kb:open', label: 'Go to Knowledgebase page' }, 'knowledgebase');
    this.bus.on('command:execute', ({ id }) => {
      if (id === 'kb:open') this.bus.emit('router:navigate', { path: '/knowledgebase' }, 'knowledgebase');
    }, 'knowledgebase');

    // Query — aggregate across all cocoons with kb service
    this.bus.on('kb:query', async (p) => {
      const { _cid, q, limit } = p as WithCid<typeof p>;
      try {
        const conns = connectionsWithKb();
        const results = await Promise.allSettled(conns.map(c => api.query(c, q, limit)));
        const allResults: SearchResult[] = results.flatMap((r, i) =>
          r.status === 'fulfilled'
            ? r.value.map(sr => ({ ...sr, node: { ...sr.node, cocoonId: conns[i].id } }))
            : []
        );
        allResults.sort((a, b) => b.score - a.score);
        this.bus.emit('kb:query:ok', { results: allResults, _cid }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:query error:', err);
        this.bus.emit('kb:query:ok', { results: [], _cid }, 'knowledgebase');
      }
    }, 'knowledgebase');

    // Add — send to specific cocoon
    this.bus.on('kb:add', async (p) => {
      const { _cid, cocoonId, user_said, derived_knowledge, node_type } = p as WithCid<typeof p>;
      try {
        const raw = await api.addNode(getConnection(cocoonId), { user_said, derived_knowledge, node_type });
        this.bus.emit('kb:add:ok', { node: { ...raw, cocoonId }, _cid }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:add error:', err);
      }
    }, 'knowledgebase');

    // Get
    this.bus.on('kb:get', async (p) => {
      const { _cid, id, cocoonId } = p as WithCid<typeof p>;
      try {
        const raw = await api.getNode(getConnection(cocoonId), id);
        this.bus.emit('kb:get:ok', { node: { ...raw, cocoonId }, _cid }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:get error:', err);
      }
    }, 'knowledgebase');

    // Delete
    this.bus.on('kb:delete', async (p) => {
      const { _cid, id, cocoonId } = p as WithCid<typeof p>;
      try {
        await api.deleteNode(getConnection(cocoonId), id);
        this.bus.emit('kb:delete:ok', { _cid }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:delete error:', err);
      }
    }, 'knowledgebase');

    // Approve
    this.bus.on('kb:approve', async (p) => {
      const { _cid, id, cocoonId } = p as WithCid<typeof p>;
      try {
        await api.approveNode(getConnection(cocoonId), id);
        this.bus.emit('kb:approve:ok', { _cid }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:approve error:', err);
      }
    }, 'knowledgebase');

    // Conflicts — aggregate
    this.bus.on('kb:conflicts', async (p) => {
      const { _cid } = p as WithCid<typeof p>;
      try {
        const conns = connectionsWithKb();
        const results = await Promise.allSettled(conns.map(c => api.getConflicts(c)));
        const conflicts: ConflictPair[] = results.flatMap((r, i) =>
          r.status === 'fulfilled'
            ? r.value.map(cp => ({
                node_a: { ...cp.node_a, cocoonId: conns[i].id },
                node_b: { ...cp.node_b, cocoonId: conns[i].id },
              }))
            : []
        );
        this.bus.emit('kb:conflicts:ok', { conflicts, _cid }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:conflicts error:', err);
        this.bus.emit('kb:conflicts:ok', { conflicts: [], _cid }, 'knowledgebase');
      }
    }, 'knowledgebase');

    // Orphans — aggregate
    this.bus.on('kb:orphans', async (p) => {
      const { _cid } = p as WithCid<typeof p>;
      try {
        const conns = connectionsWithKb();
        const results = await Promise.allSettled(conns.map(c => api.getOrphans(c)));
        const nodes: Node[] = results.flatMap((r, i) =>
          r.status === 'fulfilled'
            ? r.value.map(n => ({ ...n, cocoonId: conns[i].id }))
            : []
        );
        this.bus.emit('kb:orphans:ok', { nodes, _cid }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:orphans error:', err);
        this.bus.emit('kb:orphans:ok', { nodes: [], _cid }, 'knowledgebase');
      }
    }, 'knowledgebase');

    // Link
    this.bus.on('kb:link', async (p) => {
      const { _cid, cocoonId, from_id, to_id, edge_type, weight } = p as WithCid<typeof p>;
      try {
        const edge = await api.addEdge(getConnection(cocoonId), { from_id, to_id, edge_type, weight });
        this.bus.emit('kb:link:ok', { edge, _cid }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:link error:', err);
      }
    }, 'knowledgebase');
  }
}
