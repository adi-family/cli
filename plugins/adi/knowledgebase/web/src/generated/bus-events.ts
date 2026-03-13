/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { AdiKnowledgebaseApproveNodeEvent, AdiKnowledgebaseConflictsChangedEvent, AdiKnowledgebaseDeleteNodeEvent, AdiKnowledgebaseErrorEvent, AdiKnowledgebaseGetConflictsEvent, AdiKnowledgebaseGetNodeEvent, AdiKnowledgebaseGetOrphansEvent, AdiKnowledgebaseGetStatsEvent, AdiKnowledgebaseListNodesEvent, AdiKnowledgebaseNodeDeletedEvent, AdiKnowledgebaseNodeDetailEvent, AdiKnowledgebaseNodeMutatedEvent, AdiKnowledgebaseNodesChangedEvent, AdiKnowledgebaseOrphansChangedEvent, AdiKnowledgebaseRejectNodeEvent, AdiKnowledgebaseSearchEvent, AdiKnowledgebaseSearchResultsEvent, AdiKnowledgebaseStatsChangedEvent } from './bus-types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── adi.knowledgebase ──
    'adi.knowledgebase:list-nodes': AdiKnowledgebaseListNodesEvent;
    'adi.knowledgebase:get-node': AdiKnowledgebaseGetNodeEvent;
    'adi.knowledgebase:search': AdiKnowledgebaseSearchEvent;
    'adi.knowledgebase:approve-node': AdiKnowledgebaseApproveNodeEvent;
    'adi.knowledgebase:reject-node': AdiKnowledgebaseRejectNodeEvent;
    'adi.knowledgebase:delete-node': AdiKnowledgebaseDeleteNodeEvent;
    'adi.knowledgebase:get-conflicts': AdiKnowledgebaseGetConflictsEvent;
    'adi.knowledgebase:get-orphans': AdiKnowledgebaseGetOrphansEvent;
    'adi.knowledgebase:get-stats': AdiKnowledgebaseGetStatsEvent;
    'adi.knowledgebase:nodes-changed': AdiKnowledgebaseNodesChangedEvent;
    'adi.knowledgebase:node-detail': AdiKnowledgebaseNodeDetailEvent;
    'adi.knowledgebase:search-results': AdiKnowledgebaseSearchResultsEvent;
    'adi.knowledgebase:node-mutated': AdiKnowledgebaseNodeMutatedEvent;
    'adi.knowledgebase:node-deleted': AdiKnowledgebaseNodeDeletedEvent;
    'adi.knowledgebase:conflicts-changed': AdiKnowledgebaseConflictsChangedEvent;
    'adi.knowledgebase:orphans-changed': AdiKnowledgebaseOrphansChangedEvent;
    'adi.knowledgebase:stats-changed': AdiKnowledgebaseStatsChangedEvent;
    'adi.knowledgebase:error': AdiKnowledgebaseErrorEvent;
  }
}
