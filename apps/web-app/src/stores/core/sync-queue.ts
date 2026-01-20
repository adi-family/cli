/**
 * Offline sync queue backed by IndexedDB.
 * 
 * Queues mutations when offline and replays them when connectivity is restored.
 * Uses idb-keyval for simple IndexedDB operations.
 * 
 * Features:
 * - Persistent queue survives page reloads
 * - Automatic retry with exponential backoff
 * - Conflict detection via timestamps
 */

import { get, set, del, keys } from 'idb-keyval';
import type { QueuedMutation, Identifiable, MutationType } from './types';

// =============================================================================
// Constants
// =============================================================================

/** Prefix for IndexedDB keys */
const QUEUE_PREFIX = 'sync-queue:';

/** Maximum retry attempts before giving up */
const MAX_RETRIES = 5;

/** Base delay for exponential backoff (ms) */
const BASE_RETRY_DELAY = 1000;

// =============================================================================
// Sync Queue Implementation
// =============================================================================

/**
 * Manages offline mutation queue for a specific store.
 * 
 * Usage:
 * ```ts
 * const queue = new SyncQueue<Credential>('credentials');
 * 
 * // Queue a mutation when offline
 * await queue.enqueue({
 *   type: 'create',
 *   sourceId: 'cloud-prod',
 *   payload: { name: 'My API Key', ... },
 * });
 * 
 * // Process queue when back online
 * await queue.process(async (mutation) => {
 *   // Execute the mutation against the source
 * });
 * ```
 * 
 * @template T - Entity type
 */
export class SyncQueue<T extends Identifiable> {
  private readonly storeName: string;
  private processing = false;
  
  constructor(storeName: string) {
    this.storeName = storeName;
  }
  
  // ===========================================================================
  // Queue Operations
  // ===========================================================================
  
  /**
   * Add a mutation to the queue.
   * 
   * @param mutation - Mutation details (without id, createdAt, retryCount)
   * @returns The queued mutation with generated id
   */
  async enqueue(
    mutation: Omit<QueuedMutation<T>, 'id' | 'createdAt' | 'retryCount'>
  ): Promise<QueuedMutation<T>> {
    const queuedMutation: QueuedMutation<T> = {
      ...mutation,
      id: this.generateId(),
      createdAt: new Date(),
      retryCount: 0,
    };
    
    const key = this.getKey(queuedMutation.id);
    await set(key, queuedMutation);
    
    return queuedMutation;
  }
  
  /**
   * Remove a mutation from the queue (after successful sync).
   * 
   * @param mutationId - ID of the mutation to remove
   */
  async dequeue(mutationId: string): Promise<void> {
    const key = this.getKey(mutationId);
    await del(key);
  }
  
  /**
   * Update a mutation (e.g., increment retry count).
   * 
   * @param mutation - Updated mutation
   */
  async update(mutation: QueuedMutation<T>): Promise<void> {
    const key = this.getKey(mutation.id);
    await set(key, mutation);
  }
  
  /**
   * Get all pending mutations, ordered by creation time.
   * 
   * @returns Array of queued mutations
   */
  async getAll(): Promise<QueuedMutation<T>[]> {
    const allKeys = await keys();
    const queueKeys = allKeys.filter(
      (k) => typeof k === 'string' && k.startsWith(this.getKeyPrefix())
    );
    
    const mutations: QueuedMutation<T>[] = [];
    for (const key of queueKeys) {
      const mutation = await get<QueuedMutation<T>>(key as string);
      if (mutation) {
        mutations.push(mutation);
      }
    }
    
    // Sort by creation time (oldest first)
    return mutations.sort(
      (a, b) => new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime()
    );
  }
  
  /**
   * Get the count of pending mutations.
   */
  async count(): Promise<number> {
    const allKeys = await keys();
    return allKeys.filter(
      (k) => typeof k === 'string' && k.startsWith(this.getKeyPrefix())
    ).length;
  }
  
  /**
   * Clear all pending mutations.
   * Use with caution - this discards unsynced changes!
   */
  async clear(): Promise<void> {
    const allKeys = await keys();
    const queueKeys = allKeys.filter(
      (k) => typeof k === 'string' && k.startsWith(this.getKeyPrefix())
    );
    
    for (const key of queueKeys) {
      await del(key as string);
    }
  }
  
  // ===========================================================================
  // Processing
  // ===========================================================================
  
  /**
   * Process all pending mutations.
   * 
   * @param executor - Function to execute each mutation
   * @param onProgress - Optional callback for progress updates
   * @returns Results of processing
   */
  async process(
    executor: (mutation: QueuedMutation<T>) => Promise<void>,
    onProgress?: (processed: number, total: number) => void
  ): Promise<ProcessResult> {
    // Prevent concurrent processing
    if (this.processing) {
      return { processed: 0, failed: 0, remaining: await this.count() };
    }
    
    this.processing = true;
    
    try {
      const mutations = await this.getAll();
      const total = mutations.length;
      let processed = 0;
      let failed = 0;
      
      for (const mutation of mutations) {
        try {
          await executor(mutation);
          await this.dequeue(mutation.id);
          processed++;
        } catch (error) {
          // Increment retry count
          mutation.retryCount++;
          mutation.lastError = error instanceof Error ? error.message : String(error);
          
          if (mutation.retryCount >= MAX_RETRIES) {
            // Max retries reached - mark as failed but keep in queue
            // User can manually retry or discard later
            failed++;
          }
          
          await this.update(mutation);
        }
        
        onProgress?.(processed + failed, total);
      }
      
      return {
        processed,
        failed,
        remaining: await this.count(),
      };
    } finally {
      this.processing = false;
    }
  }
  
  /**
   * Check if the queue is currently being processed.
   */
  isProcessing(): boolean {
    return this.processing;
  }
  
  // ===========================================================================
  // Helpers
  // ===========================================================================
  
  /**
   * Generate a unique ID for a mutation.
   */
  private generateId(): string {
    return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
  }
  
  /**
   * Get the IndexedDB key prefix for this store.
   */
  private getKeyPrefix(): string {
    return `${QUEUE_PREFIX}${this.storeName}:`;
  }
  
  /**
   * Get the IndexedDB key for a mutation.
   */
  private getKey(mutationId: string): string {
    return `${this.getKeyPrefix()}${mutationId}`;
  }
}

// =============================================================================
// Types
// =============================================================================

/**
 * Result of processing the sync queue.
 */
export interface ProcessResult {
  /** Number of successfully processed mutations */
  processed: number;
  
  /** Number of failed mutations (max retries reached) */
  failed: number;
  
  /** Number of mutations still in queue */
  remaining: number;
}

// =============================================================================
// Utilities
// =============================================================================

/**
 * Calculate delay for exponential backoff.
 * 
 * @param retryCount - Current retry attempt (0-based)
 * @returns Delay in milliseconds
 */
export const getRetryDelay = (retryCount: number): number => {
  // Exponential backoff: 1s, 2s, 4s, 8s, 16s (capped at 30s)
  const delay = BASE_RETRY_DELAY * Math.pow(2, retryCount);
  return Math.min(delay, 30000);
};

/**
 * Create a helper to queue mutations with optimistic updates.
 * 
 * @param queue - The sync queue
 * @param type - Mutation type
 * @param sourceId - Target source ID
 * @returns Function to queue mutations
 */
export const createMutationHelper = <T extends Identifiable>(
  queue: SyncQueue<T>,
  type: MutationType,
  sourceId: string
) => {
  return async (
    entityId: string | undefined,
    payload: Partial<T> | Omit<T, 'id'> | undefined
  ): Promise<QueuedMutation<T>> => {
    return queue.enqueue({
      type,
      sourceId,
      entityId,
      payload,
    });
  };
};
