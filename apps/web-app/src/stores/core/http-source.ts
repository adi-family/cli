/**
 * HTTP Source adapter implementation.
 * 
 * Wraps an HTTP API endpoint as a Source<T> for the multi-source store.
 * Each HTTP source represents one backend (local, cloud-prod, cloud-dev, etc.).
 */

import { HttpClient, ApiError } from '../../api/base-client';
import type { Source, SourceConfig, Identifiable } from './types';

// =============================================================================
// HTTP Source Implementation
// =============================================================================

/**
 * Generic HTTP source that implements the Source interface.
 * 
 * Usage:
 * ```ts
 * const cloudSource = new HttpSource<Credential>({
 *   id: 'cloud-prod',
 *   name: 'Cloud (Production)',
 *   baseUrl: '/api/credentials',
 * });
 * 
 * const items = await cloudSource.list();
 * ```
 * 
 * @template T - Entity type (must extend Identifiable)
 */
export class HttpSource<T extends Identifiable> implements Source<T> {
  readonly id: string;
  readonly name: string;
  readonly readOnly: boolean;
  
  private readonly client: HttpClient;
  private readonly priority: number;
  
  constructor(config: SourceConfig) {
    this.id = config.id;
    this.name = config.name;
    this.readOnly = config.readOnly ?? false;
    this.priority = config.priority ?? 0;
    
    this.client = new HttpClient({
      baseUrl: config.baseUrl,
      headers: config.headers,
    });
  }
  
  /**
   * Fetch all items from the source.
   * 
   * @returns Array of entities
   * @throws ApiError if the request fails
   */
  async list(): Promise<T[]> {
    return this.client.get<T[]>('');
  }
  
  /**
   * Fetch a single item by ID.
   * 
   * @param id - Entity ID
   * @returns The entity
   * @throws ApiError if not found or request fails
   */
  async get(id: string): Promise<T> {
    return this.client.get<T>(`/${id}`);
  }
  
  /**
   * Create a new item.
   * 
   * @param item - Entity data (without id)
   * @returns Created entity (with id)
   * @throws ApiError if request fails
   * @throws Error if source is read-only
   */
  async create(item: Omit<T, 'id'>): Promise<T> {
    this.assertWritable('create');
    return this.client.post<T>('', item);
  }
  
  /**
   * Update an existing item.
   * 
   * @param id - Entity ID
   * @param item - Partial entity data to update
   * @returns Updated entity
   * @throws ApiError if not found or request fails
   * @throws Error if source is read-only
   */
  async update(id: string, item: Partial<T>): Promise<T> {
    this.assertWritable('update');
    return this.client.put<T>(`/${id}`, item);
  }
  
  /**
   * Delete an item.
   * 
   * @param id - Entity ID
   * @throws ApiError if not found or request fails
   * @throws Error if source is read-only
   */
  async delete(id: string): Promise<void> {
    this.assertWritable('delete');
    await this.client.delete(`/${id}`);
  }
  
  /**
   * Check if the source is reachable.
   * Performs a lightweight request to verify connectivity.
   * 
   * @returns true if source is healthy
   */
  async healthCheck(): Promise<boolean> {
    try {
      // Use list with a limit of 1 if supported, otherwise just list
      // Most APIs should return quickly even without pagination
      await this.client.get<T[]>('', { timeout: 5000 });
      return true;
    } catch (error) {
      // Network errors mean the source is unreachable
      if (error instanceof ApiError && error.isNetworkError) {
        return false;
      }
      // Auth errors mean the source is reachable but we can't access it
      // Still consider it "healthy" from a connectivity standpoint
      if (error instanceof ApiError && error.isAuthError) {
        return true;
      }
      // Other errors (500, etc.) - source is reachable but having issues
      return true;
    }
  }
  
  /**
   * Get the priority of this source for conflict resolution.
   */
  getPriority(): number {
    return this.priority;
  }
  
  /**
   * Assert that the source is writable.
   * @throws Error if source is read-only
   */
  private assertWritable(operation: string): void {
    if (this.readOnly) {
      throw new Error(
        `Cannot ${operation}: source "${this.name}" is read-only`
      );
    }
  }
}

// =============================================================================
// Factory Function
// =============================================================================

/**
 * Create an HTTP source from configuration.
 * 
 * @param config - Source configuration
 * @returns Configured HttpSource instance
 */
export const createHttpSource = <T extends Identifiable>(
  config: SourceConfig
): HttpSource<T> => {
  return new HttpSource<T>(config);
};
