/**
 * Core store infrastructure exports.
 * 
 * This module provides the building blocks for creating multi-source
 * reactive stores with offline-first support.
 */

// Types
export type {
  Identifiable,
  SyncStatus,
  ItemMeta,
  StoreItem,
  SourceStatus,
  SourceConfig,
  Source,
  MutationType,
  QueuedMutation,
  StoreOptions,
  MultiSourceStore,
  CreateStoreConfig,
} from './types';

// Factory
export { createMultiSourceStore } from './create-store';

// HTTP Source
export { HttpSource, createHttpSource } from './http-source';

// Sync Queue
export { SyncQueue, getRetryDelay, createMutationHelper } from './sync-queue';
export type { ProcessResult } from './sync-queue';
