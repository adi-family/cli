/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { AdiKnowledgebaseAddEvent, AdiKnowledgebaseApproveEvent, AdiKnowledgebaseConflictsChangedEvent, AdiKnowledgebaseConflictsEvent, AdiKnowledgebaseDeleteEvent, AdiKnowledgebaseGetEvent, AdiKnowledgebaseLinkEvent, AdiKnowledgebaseNodeChangedEvent, AdiKnowledgebaseNodeDeletedEvent, AdiKnowledgebaseOrphansChangedEvent, AdiKnowledgebaseOrphansEvent, AdiKnowledgebaseQueryEvent, AdiKnowledgebaseResultsChangedEvent } from './types';
import { AdiKnowledgebaseBusKey } from './types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── adi.knowledgebase ──
    [AdiKnowledgebaseBusKey.Query]: AdiKnowledgebaseQueryEvent;
    [AdiKnowledgebaseBusKey.Add]: AdiKnowledgebaseAddEvent;
    [AdiKnowledgebaseBusKey.Get]: AdiKnowledgebaseGetEvent;
    [AdiKnowledgebaseBusKey.Delete]: AdiKnowledgebaseDeleteEvent;
    [AdiKnowledgebaseBusKey.Approve]: AdiKnowledgebaseApproveEvent;
    [AdiKnowledgebaseBusKey.Conflicts]: AdiKnowledgebaseConflictsEvent;
    [AdiKnowledgebaseBusKey.Orphans]: AdiKnowledgebaseOrphansEvent;
    [AdiKnowledgebaseBusKey.Link]: AdiKnowledgebaseLinkEvent;
    [AdiKnowledgebaseBusKey.ResultsChanged]: AdiKnowledgebaseResultsChangedEvent;
    [AdiKnowledgebaseBusKey.NodeChanged]: AdiKnowledgebaseNodeChangedEvent;
    [AdiKnowledgebaseBusKey.NodeDeleted]: AdiKnowledgebaseNodeDeletedEvent;
    [AdiKnowledgebaseBusKey.ConflictsChanged]: AdiKnowledgebaseConflictsChangedEvent;
    [AdiKnowledgebaseBusKey.OrphansChanged]: AdiKnowledgebaseOrphansChangedEvent;
  }
}
