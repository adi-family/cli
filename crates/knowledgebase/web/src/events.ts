import type { Node, Edge, SearchResult, ConflictPair } from './types.js';

declare module '@adi-family/sdk-plugin' {
  interface EventRegistry {
    'kb:query':       { q: string; limit?: number };
    'kb:query:ok':    { results: SearchResult[]; _cid: string };

    'kb:add':         { user_said: string; derived_knowledge: string; node_type?: string; cocoonId: string };
    'kb:add:ok':      { node: Node; _cid: string };

    'kb:get':         { id: string; cocoonId: string };
    'kb:get:ok':      { node: Node; _cid: string };

    'kb:delete':      { id: string; cocoonId: string };
    'kb:delete:ok':   { _cid: string };

    'kb:approve':     { id: string; cocoonId: string };
    'kb:approve:ok':  { _cid: string };

    'kb:conflicts':   Record<string, never>;
    'kb:conflicts:ok': { conflicts: ConflictPair[]; _cid: string };

    'kb:orphans':     Record<string, never>;
    'kb:orphans:ok':  { nodes: Node[]; _cid: string };

    'kb:link':        { from_id: string; to_id: string; edge_type?: string; weight?: number; cocoonId: string };
    'kb:link:ok':     { edge: Edge; _cid: string };
  }
}

export {};
