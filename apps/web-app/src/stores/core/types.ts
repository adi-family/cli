/**
 * Core types for multi-source reactive stores.
 * 
 * Architecture:
 * - Multiple HTTP sources can be connected (local, cloud-prod, cloud-dev, etc.)
 * - Items from all sources are merged into a unified view
 * - Each item is tagged with source metadata for identification
 * - Offline mutations are queued and synced when connectivity is restored
 */

import type { ReadableAtom } from 'nanostores';

// =============================================================================
// Base Entity Constraint
// =============================================================================

/**
 * All entities managed by stores must have an id field.
 * This is the minimum contract for CRUD operations.
 */
export interface Identifiable {
  id: string;
}

// =============================================================================
// Source Metadata
// =============================================================================

/**
 * Sync status for individual items.
 * - synced: Item is in sync with its source
 * - pending: Local changes waiting to be pushed
 * - conflict: Local and remote changes conflict (needs resolution)
 * - error: Sync failed (will retry)
 */
export type SyncStatus = 'synced' | 'pending' | 'conflict' | 'error';

/**
 * Metadata attached to every item in the store.
 * This enables unified view across multiple sources.
 */
export interface ItemMeta {
  /** Identifier of the source this item came from */
  source: string;
  
  /** Current sync status */
  syncStatus: SyncStatus;
  
  /** Last successful sync timestamp */
  lastSynced?: Date;
  
  /** Error message if syncStatus is 'error' */
  errorMessage?: string;
  
  /** Local version for optimistic updates */
  localVersion?: number;
}

/**
 * Wrapper that adds metadata to any entity.
 * Components receive StoreItem<T> from the store.
 */
export interface StoreItem<T extends Identifiable> {
  data: T;
  _meta: ItemMeta;
}

// =============================================================================
// Source Interface
// =============================================================================

/**
 * Health status of a connected source.
 */
export interface SourceStatus {
  id: string;
  name: string;
  connected: boolean;
  lastCheck: Date;
  error?: string;
}

/**
 * Configuration for creating an HTTP source.
 */
export interface SourceConfig {
  /** Unique identifier for this source (e.g., 'cloud-prod', 'local') */
  id: string;
  
  /** Human-readable name for UI display */
  name: string;
  
  /** Base URL for the API (e.g., '/api/credentials' or 'https://api.example.com') */
  baseUrl: string;
  
  /** Whether this source is read-only (no create/update/delete) */
  readOnly?: boolean;
  
  /** Priority for conflict resolution (higher wins) */
  priority?: number;
  
  /** Custom headers to include in requests */
  headers?: Record<string, string>;
}

/**
 * Interface for data sources.
 * Each source (local, cloud, etc.) implements this interface.
 * 
 * @template T - The entity type (must have an id field)
 */
export interface Source<T extends Identifiable> {
  /** Unique identifier for this source */
  readonly id: string;
  
  /** Human-readable name */
  readonly name: string;
  
  /** Whether this source is read-only */
  readonly readOnly: boolean;
  
  /** Fetch all items from the source */
  list(): Promise<T[]>;
  
  /** Fetch a single item by id */
  get(id: string): Promise<T>;
  
  /** Create a new item (throws if readOnly) */
  create(item: Omit<T, 'id'>): Promise<T>;
  
  /** Update an existing item (throws if readOnly) */
  update(id: string, item: Partial<T>): Promise<T>;
  
  /** Delete an item (throws if readOnly) */
  delete(id: string): Promise<void>;
  
  /** Check if the source is reachable */
  healthCheck(): Promise<boolean>;
}

// =============================================================================
// Sync Queue Types
// =============================================================================

/**
 * Types of mutations that can be queued for offline sync.
 */
export type MutationType = 'create' | 'update' | 'delete';

/**
 * A queued mutation waiting to be synced.
 */
export interface QueuedMutation<T extends Identifiable> {
  /** Unique id for this mutation */
  id: string;
  
  /** Type of operation */
  type: MutationType;
  
  /** Target source id */
  sourceId: string;
  
  /** Entity id (for update/delete) */
  entityId?: string;
  
  /** Payload (for create/update) */
  payload?: Partial<T> | Omit<T, 'id'>;
  
  /** When the mutation was queued */
  createdAt: Date;
  
  /** Number of retry attempts */
  retryCount: number;
  
  /** Last error if any */
  lastError?: string;
}

// =============================================================================
// Store Interface
// =============================================================================

/**
 * Options for store operations.
 */
export interface StoreOptions {
  /** Skip cache and force fetch from source */
  forceRefresh?: boolean;
}

/**
 * The main multi-source store interface.
 * Provides reactive atoms for UI binding and methods for CRUD operations.
 * 
 * @template T - The entity type
 */
export interface MultiSourceStore<T extends Identifiable> {
  // ===========================================================================
  // Reactive Atoms (subscribe for UI updates)
  // ===========================================================================
  
  /** All items from all sources, merged and tagged with metadata */
  readonly $items: ReadableAtom<StoreItem<T>[]>;
  
  /** Loading state (true during fetch operations) */
  readonly $loading: ReadableAtom<boolean>;
  
  /** Current error if any */
  readonly $error: ReadableAtom<Error | null>;
  
  /** Status of all connected sources */
  readonly $sources: ReadableAtom<SourceStatus[]>;
  
  /** Number of pending mutations in the offline queue */
  readonly $pendingCount: ReadableAtom<number>;
  
  /** Online/offline status */
  readonly $online: ReadableAtom<boolean>;
  
  // ===========================================================================
  // Read Operations
  // ===========================================================================
  
  /** Refresh data from all sources */
  refresh(options?: StoreOptions): Promise<void>;
  
  /** Get a single item by id (from cache or fetch) */
  getById(id: string): StoreItem<T> | undefined;
  
  /** Get items filtered by source */
  getBySource(sourceId: string): StoreItem<T>[];
  
  // ===========================================================================
  // Write Operations
  // ===========================================================================
  
  /**
   * Create a new item in the specified source.
   * If offline, queues the mutation for later sync.
   */
  create(item: Omit<T, 'id'>, targetSource: string): Promise<T>;
  
  /**
   * Update an existing item.
   * Automatically targets the item's original source.
   */
  update(id: string, changes: Partial<T>): Promise<T>;
  
  /**
   * Delete an item.
   * Automatically targets the item's original source.
   */
  delete(id: string): Promise<void>;
  
  // ===========================================================================
  // Source Management
  // ===========================================================================
  
  /** Add a new source to the store */
  addSource(config: SourceConfig): void;
  
  /** Remove a source from the store */
  removeSource(sourceId: string): void;
  
  /** Get configuration for a source */
  getSource(sourceId: string): Source<T> | undefined;
  
  // ===========================================================================
  // Sync Operations
  // ===========================================================================
  
  /** Manually trigger sync of pending mutations */
  syncPending(): Promise<void>;
  
  /** Clear all pending mutations (use with caution) */
  clearPending(): Promise<void>;
}

// =============================================================================
// Factory Types
// =============================================================================

/**
 * Configuration for creating a multi-source store.
 */
export interface CreateStoreConfig<T extends Identifiable> {
  /** Unique name for this store (used for IndexedDB keys) */
  name: string;
  
  /** Initial sources to connect */
  sources: SourceConfig[];
  
  /** 
   * Function to extract a unique key for deduplication across sources.
   * Defaults to using the id field.
   */
  getKey?: (item: T) => string;
  
  /**
   * Function to merge items with the same key from different sources.
   * Defaults to preferring the source with higher priority.
   */
  merge?: (items: StoreItem<T>[]) => StoreItem<T>;
}
