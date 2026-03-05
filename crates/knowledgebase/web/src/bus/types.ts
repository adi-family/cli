/**
 * Auto-generated eventbus types from TypeSpec.
 * DO NOT EDIT.
 */

export interface AdiKnowledgebaseQueryEvent {
  q: string;
  limit?: number;
}

export interface AdiKnowledgebaseAddEvent {
  user_said: string;
  derived_knowledge: string;
  node_type?: string;
  cocoonId: string;
}

export interface AdiKnowledgebaseGetEvent {
  id: string;
  cocoonId: string;
}

export interface AdiKnowledgebaseDeleteEvent {
  id: string;
  cocoonId: string;
}

export interface AdiKnowledgebaseApproveEvent {
  id: string;
  cocoonId: string;
}

export interface AdiKnowledgebaseConflictsEvent {
}

export interface AdiKnowledgebaseOrphansEvent {
}

export interface AdiKnowledgebaseLinkEvent {
  from_id: string;
  to_id: string;
  edge_type?: string;
  weight?: number;
  cocoonId: string;
}

export interface AdiKnowledgebaseResultsChangedEvent {
  results: unknown[];
}

export interface AdiKnowledgebaseNodeChangedEvent {
  node: unknown;
  edges: unknown[];
}

export interface AdiKnowledgebaseNodeDeletedEvent {
  id: string;
  cocoonId: string;
}

export interface AdiKnowledgebaseConflictsChangedEvent {
  conflicts: unknown[];
}

export interface AdiKnowledgebaseOrphansChangedEvent {
  nodes: unknown[];
}

export enum AdiKnowledgebaseBusKey {
  Query = 'adi.knowledgebase:query',
  Add = 'adi.knowledgebase:add',
  Get = 'adi.knowledgebase:get',
  Delete = 'adi.knowledgebase:delete',
  Approve = 'adi.knowledgebase:approve',
  Conflicts = 'adi.knowledgebase:conflicts',
  Orphans = 'adi.knowledgebase:orphans',
  Link = 'adi.knowledgebase:link',
  ResultsChanged = 'adi.knowledgebase:results-changed',
  NodeChanged = 'adi.knowledgebase:node-changed',
  NodeDeleted = 'adi.knowledgebase:node-deleted',
  ConflictsChanged = 'adi.knowledgebase:conflicts-changed',
  OrphansChanged = 'adi.knowledgebase:orphans-changed',
}
