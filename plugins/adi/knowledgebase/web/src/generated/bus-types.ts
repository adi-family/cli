/**
 * Auto-generated eventbus types from TypeSpec.
 * DO NOT EDIT.
 */

import type { ConflictPair, NodeStats, NodeWithCocoon, SearchResult } from './models';

import { ApprovalStatus, NodeType } from './enums';

export interface AdiKnowledgebaseListNodesEvent {
  node_type?: NodeType;
  approval_status?: ApprovalStatus;
  source?: string;
}

export interface AdiKnowledgebaseGetNodeEvent {
  id: string;
  cocoonId: string;
}

export interface AdiKnowledgebaseSearchEvent {
  cocoonId: string;
  query: string;
  limit?: number;
}

export interface AdiKnowledgebaseApproveNodeEvent {
  id: string;
  cocoonId: string;
}

export interface AdiKnowledgebaseRejectNodeEvent {
  id: string;
  cocoonId: string;
  reason?: string;
}

export interface AdiKnowledgebaseDeleteNodeEvent {
  id: string;
  cocoonId: string;
}

export interface AdiKnowledgebaseGetConflictsEvent {
  cocoonId: string;
}

export interface AdiKnowledgebaseGetOrphansEvent {
  cocoonId: string;
}

export interface AdiKnowledgebaseGetStatsEvent {
  cocoonId: string;
}

export interface AdiKnowledgebaseNodesChangedEvent {
  nodes: NodeWithCocoon[];
}

export interface AdiKnowledgebaseNodeDetailEvent {
  node: NodeWithCocoon;
}

export interface AdiKnowledgebaseSearchResultsEvent {
  results: SearchResult[];
  cocoonId: string;
}

export interface AdiKnowledgebaseNodeMutatedEvent {
  node: NodeWithCocoon;
}

export interface AdiKnowledgebaseNodeDeletedEvent {
  id: string;
  cocoonId: string;
}

export interface AdiKnowledgebaseConflictsChangedEvent {
  conflicts: ConflictPair[];
  cocoonId: string;
}

export interface AdiKnowledgebaseOrphansChangedEvent {
  orphans: NodeWithCocoon[];
}

export interface AdiKnowledgebaseStatsChangedEvent {
  stats: NodeStats;
  cocoonId: string;
}

export interface AdiKnowledgebaseErrorEvent {
  message: string;
  event: string;
}

export enum AdiKnowledgebaseBusKey {
  ListNodes = 'adi.knowledgebase:list-nodes',
  GetNode = 'adi.knowledgebase:get-node',
  Search = 'adi.knowledgebase:search',
  ApproveNode = 'adi.knowledgebase:approve-node',
  RejectNode = 'adi.knowledgebase:reject-node',
  DeleteNode = 'adi.knowledgebase:delete-node',
  GetConflicts = 'adi.knowledgebase:get-conflicts',
  GetOrphans = 'adi.knowledgebase:get-orphans',
  GetStats = 'adi.knowledgebase:get-stats',
  NodesChanged = 'adi.knowledgebase:nodes-changed',
  NodeDetail = 'adi.knowledgebase:node-detail',
  SearchResults = 'adi.knowledgebase:search-results',
  NodeMutated = 'adi.knowledgebase:node-mutated',
  NodeDeleted = 'adi.knowledgebase:node-deleted',
  ConflictsChanged = 'adi.knowledgebase:conflicts-changed',
  OrphansChanged = 'adi.knowledgebase:orphans-changed',
  StatsChanged = 'adi.knowledgebase:stats-changed',
  Error = 'adi.knowledgebase:error',
}
