/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { AdiKnowledgebaseAddEvent, AdiKnowledgebaseApproveEvent, AdiKnowledgebaseConflictsChangedEvent, AdiKnowledgebaseConflictsEvent, AdiKnowledgebaseDeleteEvent, AdiKnowledgebaseGetEvent, AdiKnowledgebaseLinkEvent, AdiKnowledgebaseNodeChangedEvent, AdiKnowledgebaseNodeDeletedEvent, AdiKnowledgebaseOrphansChangedEvent, AdiKnowledgebaseOrphansEvent, AdiKnowledgebaseQueryEvent, AdiKnowledgebaseResultsChangedEvent } from './bus-types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── adi.knowledgebase ──
    'adi.knowledgebase:query': AdiKnowledgebaseQueryEvent;
    'adi.knowledgebase:add': AdiKnowledgebaseAddEvent;
    'adi.knowledgebase:get': AdiKnowledgebaseGetEvent;
    'adi.knowledgebase:delete': AdiKnowledgebaseDeleteEvent;
    'adi.knowledgebase:approve': AdiKnowledgebaseApproveEvent;
    'adi.knowledgebase:conflicts': AdiKnowledgebaseConflictsEvent;
    'adi.knowledgebase:orphans': AdiKnowledgebaseOrphansEvent;
    'adi.knowledgebase:link': AdiKnowledgebaseLinkEvent;
    'adi.knowledgebase:results-changed': AdiKnowledgebaseResultsChangedEvent;
    'adi.knowledgebase:node-changed': AdiKnowledgebaseNodeChangedEvent;
    'adi.knowledgebase:node-deleted': AdiKnowledgebaseNodeDeletedEvent;
    'adi.knowledgebase:conflicts-changed': AdiKnowledgebaseConflictsChangedEvent;
    'adi.knowledgebase:orphans-changed': AdiKnowledgebaseOrphansChangedEvent;
  }
}
