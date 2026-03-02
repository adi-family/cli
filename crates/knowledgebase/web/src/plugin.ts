import { AdiPlugin } from '@adi-family/sdk-plugin';
import * as api from './api.js';
import type { Connection, Node, SearchResult, ConflictPair } from './types.js';
import { setBus, connections } from './context.js';
import './events.js';

function connectionsWithKb(): Connection[] {
  return [...connections.values()]
    .filter(c => c.services.includes('kb'));
}

function getConnection(cocoonId: string): Connection {
  const c = connections.get(cocoonId);
  if (!c) throw new Error(`Connection '${cocoonId}' not found`);
  return c;
}

export class KnowledgebasePlugin extends AdiPlugin {
  readonly id = 'adi.knowledgebase';
  readonly version = '0.1.0';

  async onRegister(): Promise<void> {
    setBus(this.bus);

    this.bus.on('connection:added', ({ id, connection }) => {
      connections.set(id, connection as Connection);
    }, 'knowledgebase');
    this.bus.on('connection:removed', ({ id }) => {
      connections.delete(id);
    }, 'knowledgebase');

    const { AdiKnowledgebaseElement } = await import('./component.js');
    if (!customElements.get('adi-knowledgebase')) {
      customElements.define('adi-knowledgebase', AdiKnowledgebaseElement);
    }

    this.bus.emit('route:register', { path: '/knowledgebase', element: 'adi-knowledgebase' }, 'knowledgebase');
    this.bus.emit('nav:add', { id: 'knowledgebase', label: 'Knowledge', path: '/knowledgebase' }, 'knowledgebase');

    this.bus.emit('command:register', { id: 'kb:open', label: 'Go to Knowledgebase page' }, 'knowledgebase');
    this.bus.on('command:execute', ({ id }) => {
      if (id === 'kb:open') this.bus.emit('router:navigate', { path: '/knowledgebase' }, 'knowledgebase');
    }, 'knowledgebase');

    this.bus.on('kb:query', async ({ q, limit }) => {
      try {
        const conns = connectionsWithKb();
        const results = await Promise.allSettled(conns.map(c => api.query(c, q, limit)));
        const allResults: SearchResult[] = results.flatMap((r, i) =>
          r.status === 'fulfilled'
            ? r.value.map(sr => ({ ...sr, node: { ...sr.node, cocoonId: conns[i].id } }))
            : []
        );
        allResults.sort((a, b) => b.score - a.score);
        this.bus.emit('kb:results-changed', { results: allResults }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:query error:', err);
        this.bus.emit('kb:results-changed', { results: [] }, 'knowledgebase');
      }
    }, 'knowledgebase');

    this.bus.on('kb:add', async ({ cocoonId, user_said, derived_knowledge, node_type }) => {
      try {
        const raw = await api.addNode(getConnection(cocoonId), { user_said, derived_knowledge, node_type });
        this.bus.emit('kb:node-changed', { node: { ...raw, cocoonId }, edges: [] }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:add error:', err);
      }
    }, 'knowledgebase');

    this.bus.on('kb:get', async ({ id, cocoonId }) => {
      try {
        const raw = await api.getNode(getConnection(cocoonId), id);
        this.bus.emit('kb:node-changed', { node: { ...raw, cocoonId }, edges: [] }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:get error:', err);
      }
    }, 'knowledgebase');

    this.bus.on('kb:delete', async ({ id, cocoonId }) => {
      try {
        await api.deleteNode(getConnection(cocoonId), id);
        this.bus.emit('kb:node-deleted', { id, cocoonId }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:delete error:', err);
      }
    }, 'knowledgebase');

    this.bus.on('kb:approve', async ({ id, cocoonId }) => {
      try {
        await api.approveNode(getConnection(cocoonId), id);
        const raw = await api.getNode(getConnection(cocoonId), id);
        this.bus.emit('kb:node-changed', { node: { ...raw, cocoonId }, edges: [] }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:approve error:', err);
      }
    }, 'knowledgebase');

    this.bus.on('kb:conflicts', async () => {
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
        this.bus.emit('kb:conflicts-changed', { conflicts }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:conflicts error:', err);
        this.bus.emit('kb:conflicts-changed', { conflicts: [] }, 'knowledgebase');
      }
    }, 'knowledgebase');

    this.bus.on('kb:orphans', async () => {
      try {
        const conns = connectionsWithKb();
        const results = await Promise.allSettled(conns.map(c => api.getOrphans(c)));
        const nodes: Node[] = results.flatMap((r, i) =>
          r.status === 'fulfilled'
            ? r.value.map(n => ({ ...n, cocoonId: conns[i].id }))
            : []
        );
        this.bus.emit('kb:orphans-changed', { nodes }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:orphans error:', err);
        this.bus.emit('kb:orphans-changed', { nodes: [] }, 'knowledgebase');
      }
    }, 'knowledgebase');

    this.bus.on('kb:link', async ({ cocoonId, from_id, to_id, edge_type, weight }) => {
      try {
        const edge = await api.addEdge(getConnection(cocoonId), { from_id, to_id, edge_type, weight });
        const raw = await api.getNode(getConnection(cocoonId), from_id);
        this.bus.emit('kb:node-changed', { node: { ...raw, cocoonId }, edges: [edge] }, 'knowledgebase');
      } catch (err) {
        console.error('[KnowledgebasePlugin] kb:link error:', err);
      }
    }, 'knowledgebase');
  }
}
