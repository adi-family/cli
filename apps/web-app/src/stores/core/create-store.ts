/**
 * Factory for creating multi-source reactive stores.
 * 
 * This is the main entry point for creating stores. It wires together:
 * - Multiple HTTP sources
 * - Nanostores atoms for reactivity
 * - Offline sync queue
 * - Online/offline detection
 */

import { atom, computed } from 'nanostores';
import type { ReadableAtom } from 'nanostores';
import { HttpSource, createHttpSource } from './http-source';
import { SyncQueue } from './sync-queue';
import { isOnline } from '../../api/base-client';
import type {
  Identifiable,
  StoreItem,
  ItemMeta,
  SourceConfig,
  SourceStatus,
  Source,
  MultiSourceStore,
  CreateStoreConfig,
  StoreOptions,
} from './types';

// =============================================================================
// Store Implementation
// =============================================================================

/**
 * Create a multi-source reactive store.
 * 
 * Usage:
 * ```ts
 * // Create a credentials store with local and cloud sources
 * const credentialsStore = createMultiSourceStore<Credential>({
 *   name: 'credentials',
 *   sources: [
 *     { id: 'local', name: 'Local', baseUrl: '/api/local/credentials' },
 *     { id: 'cloud', name: 'Cloud', baseUrl: '/api/credentials' },
 *   ],
 * });
 * 
 * // Subscribe to items in a Lit component
 * import { StoreController } from '@nanostores/lit';
 * 
 * class MyComponent extends LitElement {
 *   private items = new StoreController(this, credentialsStore.$items);
 *   
 *   render() {
 *     return html`${this.items.value.map(item => ...)}`;
 *   }
 * }
 * ```
 * 
 * @template T - Entity type
 * @param config - Store configuration
 * @returns Multi-source store instance
 */
export const createMultiSourceStore = <T extends Identifiable>(
  config: CreateStoreConfig<T>
): MultiSourceStore<T> => {
  // ===========================================================================
  // Internal State
  // ===========================================================================
  
  /** Map of source ID to Source instance */
  const sources = new Map<string, HttpSource<T>>();
  
  /** Offline mutation queue */
  const syncQueue = new SyncQueue<T>(config.name);
  
  /** Key extractor (defaults to id) */
  const getKey = config.getKey ?? ((item: T) => item.id);
  
  // ===========================================================================
  // Reactive Atoms
  // ===========================================================================
  
  /** Raw items from all sources (before merging) */
  const $rawItems = atom<Map<string, StoreItem<T>[]>>(new Map());
  
  /** Loading state */
  const $loading = atom<boolean>(false);
  
  /** Current error */
  const $error = atom<Error | null>(null);
  
  /** Source statuses */
  const $sourceStatuses = atom<Map<string, SourceStatus>>(new Map());
  
  /** Pending mutation count */
  const $pendingCount = atom<number>(0);
  
  /** Online status */
  const $online = atom<boolean>(isOnline());
  
  // ===========================================================================
  // Computed Atoms
  // ===========================================================================
  
  /**
   * Merged items from all sources.
   * Items with the same key are merged using the provided merge function
   * or default to highest priority source.
   */
  const $items: ReadableAtom<StoreItem<T>[]> = computed($rawItems, (rawMap) => {
    // Collect all items by key
    const byKey = new Map<string, StoreItem<T>[]>();
    
    for (const [_sourceId, items] of rawMap) {
      for (const item of items) {
        const key = getKey(item.data);
        const existing = byKey.get(key) ?? [];
        existing.push(item);
        byKey.set(key, existing);
      }
    }
    
    // Merge items with same key
    const merged: StoreItem<T>[] = [];
    for (const items of byKey.values()) {
      if (items.length === 1) {
        merged.push(items[0]);
      } else if (config.merge) {
        merged.push(config.merge(items));
      } else {
        // Default: prefer highest priority source
        merged.push(defaultMerge(items, sources));
      }
    }
    
    return merged;
  });
  
  /**
   * Source statuses as array.
   */
  const $sources: ReadableAtom<SourceStatus[]> = computed(
    $sourceStatuses,
    (map) => Array.from(map.values())
  );
  
  // ===========================================================================
  // Online/Offline Handling
  // ===========================================================================
  
  // Listen for online/offline events
  if (typeof window !== 'undefined') {
    window.addEventListener('online', () => {
      $online.set(true);
      // Trigger sync when coming back online
      void syncPending();
    });
    
    window.addEventListener('offline', () => {
      $online.set(false);
    });
  }
  
  // ===========================================================================
  // Source Management
  // ===========================================================================
  
  /**
   * Add a source to the store.
   */
  const addSource = (sourceConfig: SourceConfig): void => {
    const source = createHttpSource<T>(sourceConfig);
    sources.set(sourceConfig.id, source);
    
    // Initialize source status
    const statuses = new Map($sourceStatuses.get());
    statuses.set(sourceConfig.id, {
      id: sourceConfig.id,
      name: sourceConfig.name,
      connected: false,
      lastCheck: new Date(),
    });
    $sourceStatuses.set(statuses);
  };
  
  /**
   * Remove a source from the store.
   */
  const removeSource = (sourceId: string): void => {
    sources.delete(sourceId);
    
    // Remove from statuses
    const statuses = new Map($sourceStatuses.get());
    statuses.delete(sourceId);
    $sourceStatuses.set(statuses);
    
    // Remove items from this source
    const rawMap = new Map($rawItems.get());
    rawMap.delete(sourceId);
    $rawItems.set(rawMap);
  };
  
  /**
   * Get a source by ID.
   */
  const getSource = (sourceId: string): Source<T> | undefined => {
    return sources.get(sourceId);
  };
  
  // Initialize sources from config
  for (const sourceConfig of config.sources) {
    addSource(sourceConfig);
  }
  
  // ===========================================================================
  // Read Operations
  // ===========================================================================
  
  /**
   * Refresh data from all sources.
   */
  const refresh = async (_options?: StoreOptions): Promise<void> => {
    $loading.set(true);
    $error.set(null);
    
    try {
      // Fetch from all sources in parallel
      const results = await Promise.allSettled(
        Array.from(sources.entries()).map(async ([sourceId, source]) => {
          const items = await source.list();
          return { sourceId, items };
        })
      );
      
      // Update raw items and source statuses
      const rawMap = new Map<string, StoreItem<T>[]>();
      const statuses = new Map($sourceStatuses.get());
      
      for (let i = 0; i < results.length; i++) {
        const result = results[i];
        const sourceId = Array.from(sources.keys())[i];
        const source = sources.get(sourceId)!;
        
        if (result.status === 'fulfilled') {
          // Wrap items with metadata
          const storeItems: StoreItem<T>[] = result.value.items.map((item) => ({
            data: item,
            _meta: {
              source: sourceId,
              syncStatus: 'synced' as const,
              lastSynced: new Date(),
            },
          }));
          rawMap.set(sourceId, storeItems);
          
          // Update status
          statuses.set(sourceId, {
            id: sourceId,
            name: source.name,
            connected: true,
            lastCheck: new Date(),
          });
        } else {
          // Keep existing items on error
          const existing = $rawItems.get().get(sourceId);
          if (existing) {
            rawMap.set(sourceId, existing);
          }
          
          // Update status with error
          statuses.set(sourceId, {
            id: sourceId,
            name: source.name,
            connected: false,
            lastCheck: new Date(),
            error: result.reason instanceof Error 
              ? result.reason.message 
              : String(result.reason),
          });
        }
      }
      
      $rawItems.set(rawMap);
      $sourceStatuses.set(statuses);
      
      // Update pending count
      $pendingCount.set(await syncQueue.count());
    } catch (error) {
      $error.set(error instanceof Error ? error : new Error(String(error)));
    } finally {
      $loading.set(false);
    }
  };
  
  /**
   * Get item by ID from cache.
   */
  const getById = (id: string): StoreItem<T> | undefined => {
    return $items.get().find((item) => item.data.id === id);
  };
  
  /**
   * Get items by source.
   */
  const getBySource = (sourceId: string): StoreItem<T>[] => {
    return $items.get().filter((item) => item._meta.source === sourceId);
  };
  
  // ===========================================================================
  // Write Operations
  // ===========================================================================
  
  /**
   * Create a new item.
   */
  const create = async (
    item: Omit<T, 'id'>,
    targetSource: string
  ): Promise<T> => {
    const source = sources.get(targetSource);
    if (!source) {
      throw new Error(`Source "${targetSource}" not found`);
    }
    
    // If offline, queue the mutation
    if (!$online.get()) {
      await syncQueue.enqueue({
        type: 'create',
        sourceId: targetSource,
        payload: item,
      });
      $pendingCount.set(await syncQueue.count());
      
      // Create optimistic item with temporary ID
      const tempId = `temp-${Date.now()}`;
      const optimisticItem: StoreItem<T> = {
        data: { ...item, id: tempId } as T,
        _meta: {
          source: targetSource,
          syncStatus: 'pending',
        },
      };
      
      // Add to raw items
      const rawMap = new Map($rawItems.get());
      const sourceItems = rawMap.get(targetSource) ?? [];
      rawMap.set(targetSource, [...sourceItems, optimisticItem]);
      $rawItems.set(rawMap);
      
      return optimisticItem.data;
    }
    
    // Online: create directly
    const created = await source.create(item);
    
    // Add to store
    const storeItem: StoreItem<T> = {
      data: created,
      _meta: {
        source: targetSource,
        syncStatus: 'synced',
        lastSynced: new Date(),
      },
    };
    
    const rawMap = new Map($rawItems.get());
    const sourceItems = rawMap.get(targetSource) ?? [];
    rawMap.set(targetSource, [...sourceItems, storeItem]);
    $rawItems.set(rawMap);
    
    return created;
  };
  
  /**
   * Update an existing item.
   */
  const update = async (id: string, changes: Partial<T>): Promise<T> => {
    const existing = getById(id);
    if (!existing) {
      throw new Error(`Item "${id}" not found`);
    }
    
    const source = sources.get(existing._meta.source);
    if (!source) {
      throw new Error(`Source "${existing._meta.source}" not found`);
    }
    
    // If offline, queue the mutation
    if (!$online.get()) {
      await syncQueue.enqueue({
        type: 'update',
        sourceId: existing._meta.source,
        entityId: id,
        payload: changes,
      });
      $pendingCount.set(await syncQueue.count());
      
      // Optimistic update
      updateItemInStore(id, changes, 'pending');
      return { ...existing.data, ...changes };
    }
    
    // Online: update directly
    const updated = await source.update(id, changes);
    updateItemInStore(id, updated, 'synced');
    
    return updated;
  };
  
  /**
   * Delete an item.
   */
  const deleteItem = async (id: string): Promise<void> => {
    const existing = getById(id);
    if (!existing) {
      throw new Error(`Item "${id}" not found`);
    }
    
    const source = sources.get(existing._meta.source);
    if (!source) {
      throw new Error(`Source "${existing._meta.source}" not found`);
    }
    
    // If offline, queue the mutation
    if (!$online.get()) {
      await syncQueue.enqueue({
        type: 'delete',
        sourceId: existing._meta.source,
        entityId: id,
      });
      $pendingCount.set(await syncQueue.count());
      
      // Optimistic delete (mark as pending)
      updateItemInStore(id, {}, 'pending');
      return;
    }
    
    // Online: delete directly
    await source.delete(id);
    removeItemFromStore(id);
  };
  
  // ===========================================================================
  // Sync Operations
  // ===========================================================================
  
  /**
   * Process pending mutations.
   */
  const syncPending = async (): Promise<void> => {
    if (!$online.get()) {
      return;
    }
    
    await syncQueue.process(async (mutation) => {
      const source = sources.get(mutation.sourceId);
      if (!source) {
        throw new Error(`Source "${mutation.sourceId}" not found`);
      }
      
      switch (mutation.type) {
        case 'create':
          await source.create(mutation.payload as Omit<T, 'id'>);
          break;
        case 'update':
          await source.update(mutation.entityId!, mutation.payload as Partial<T>);
          break;
        case 'delete':
          await source.delete(mutation.entityId!);
          break;
      }
    });
    
    // Refresh to get server state
    await refresh();
    $pendingCount.set(await syncQueue.count());
  };
  
  /**
   * Clear all pending mutations.
   */
  const clearPending = async (): Promise<void> => {
    await syncQueue.clear();
    $pendingCount.set(0);
  };
  
  // ===========================================================================
  // Helper Functions
  // ===========================================================================
  
  /**
   * Update an item in the store.
   */
  const updateItemInStore = (
    id: string,
    changes: Partial<T> | T,
    syncStatus: ItemMeta['syncStatus']
  ): void => {
    const rawMap = new Map($rawItems.get());
    
    for (const [sourceId, items] of rawMap) {
      const index = items.findIndex((item) => item.data.id === id);
      if (index !== -1) {
        const updated = [...items];
        updated[index] = {
          data: { ...updated[index].data, ...changes },
          _meta: {
            ...updated[index]._meta,
            syncStatus,
            lastSynced: syncStatus === 'synced' ? new Date() : undefined,
          },
        };
        rawMap.set(sourceId, updated);
        break;
      }
    }
    
    $rawItems.set(rawMap);
  };
  
  /**
   * Remove an item from the store.
   */
  const removeItemFromStore = (id: string): void => {
    const rawMap = new Map($rawItems.get());
    
    for (const [sourceId, items] of rawMap) {
      const filtered = items.filter((item) => item.data.id !== id);
      if (filtered.length !== items.length) {
        rawMap.set(sourceId, filtered);
        break;
      }
    }
    
    $rawItems.set(rawMap);
  };
  
  // ===========================================================================
  // Return Store Interface
  // ===========================================================================
  
  return {
    // Atoms
    $items,
    $loading,
    $error,
    $sources,
    $pendingCount,
    $online,
    
    // Read operations
    refresh,
    getById,
    getBySource,
    
    // Write operations
    create,
    update,
    delete: deleteItem,
    
    // Source management
    addSource,
    removeSource,
    getSource,
    
    // Sync operations
    syncPending,
    clearPending,
  };
};

// =============================================================================
// Default Merge Strategy
// =============================================================================

/**
 * Default merge strategy: prefer highest priority source.
 */
const defaultMerge = <T extends Identifiable>(
  items: StoreItem<T>[],
  sources: Map<string, HttpSource<T>>
): StoreItem<T> => {
  // Sort by source priority (highest first)
  const sorted = [...items].sort((a, b) => {
    const priorityA = sources.get(a._meta.source)?.getPriority() ?? 0;
    const priorityB = sources.get(b._meta.source)?.getPriority() ?? 0;
    return priorityB - priorityA;
  });
  
  return sorted[0];
};
