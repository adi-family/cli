import type { Node, Edge, SearchResult, ConflictPair } from './types.js';

declare module '@adi-family/sdk-plugin' {
  interface EventRegistry {
    'kb:query':     { q: string; limit?: number };
    'kb:add':       { user_said: string; derived_knowledge: string; node_type?: string; cocoonId: string };
    'kb:get':       { id: string; cocoonId: string };
    'kb:delete':    { id: string; cocoonId: string };
    'kb:approve':   { id: string; cocoonId: string };
    'kb:conflicts': Record<string, never>;
    'kb:orphans':   Record<string, never>;
    'kb:link':      { from_id: string; to_id: string; edge_type?: string; weight?: number; cocoonId: string };

    'kb:results-changed':   { results: SearchResult[] };
    'kb:node-changed':      { node: Node; edges: Edge[] };
    'kb:node-deleted':      { id: string; cocoonId: string };
    'kb:conflicts-changed': { conflicts: ConflictPair[] };
    'kb:orphans-changed':   { nodes: Node[] };
  }
}

export {};
