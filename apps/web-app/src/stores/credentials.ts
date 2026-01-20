/**
 * Credentials store - Multi-source reactive store for credentials.
 * 
 * Supports multiple HTTP backends (local, cloud, etc.) with automatic
 * merging and offline-first sync.
 * 
 * Usage in Lit components:
 * ```ts
 * import { StoreController } from '@nanostores/lit';
 * import { credentialsStore } from '../stores/credentials';
 * 
 * class MyComponent extends LitElement {
 *   private credentials = new StoreController(this, credentialsStore.$items);
 *   private loading = new StoreController(this, credentialsStore.$loading);
 *   
 *   render() {
 *     if (this.loading.value) return html`<loading-spinner></loading-spinner>`;
 *     return html`
 *       ${this.credentials.value.map(item => html`
 *         <credential-card
 *           .credential=${item.data}
 *           .source=${item._meta.source}
 *         ></credential-card>
 *       `)}
 *     `;
 *   }
 * }
 * ```
 */

import { createMultiSourceStore } from './core';
import type { Credential } from '../services/generated/credentials/typescript';

// =============================================================================
// Store Configuration
// =============================================================================

/**
 * Default source configurations.
 * These can be modified at runtime via addSource/removeSource.
 */
const DEFAULT_SOURCES = [
  {
    id: 'cloud',
    name: 'Cloud',
    baseUrl: '/api/credentials/credentials',
    priority: 10,
  },
];

// =============================================================================
// Credentials Store Instance
// =============================================================================

/**
 * The main credentials store instance.
 * 
 * Features:
 * - Multiple HTTP sources (add via credentialsStore.addSource())
 * - Unified view with source tagging
 * - Offline-first with IndexedDB queue
 * - Reactive via nanostores
 * 
 * Reactive atoms:
 * - $items: All credentials from all sources
 * - $loading: Loading state
 * - $error: Current error
 * - $sources: Status of all connected sources
 * - $pendingCount: Number of pending offline mutations
 * - $online: Online/offline status
 */
export const credentialsStore = createMultiSourceStore<Credential>({
  name: 'credentials',
  sources: DEFAULT_SOURCES,
});

// =============================================================================
// Convenience Functions
// =============================================================================

/**
 * Add a new credentials source.
 * 
 * @example
 * ```ts
 * addCredentialsSource({
 *   id: 'local',
 *   name: 'Local Storage',
 *   baseUrl: 'http://localhost:8032/credentials',
 *   priority: 5,
 * });
 * ```
 */
export const addCredentialsSource = credentialsStore.addSource;

/**
 * Remove a credentials source.
 */
export const removeCredentialsSource = credentialsStore.removeSource;

/**
 * Refresh credentials from all sources.
 */
export const refreshCredentials = credentialsStore.refresh;

/**
 * Create a new credential.
 * 
 * @param credential - Credential data (without id)
 * @param targetSource - Source to create in (default: 'cloud')
 */
export const createCredential = (
  credential: Omit<Credential, 'id' | 'createdAt' | 'updatedAt'>,
  targetSource = 'cloud'
) => credentialsStore.create(credential as Omit<Credential, 'id'>, targetSource);

/**
 * Update a credential.
 * 
 * @param id - Credential ID
 * @param changes - Partial credential data
 */
export const updateCredential = credentialsStore.update;

/**
 * Delete a credential.
 * 
 * @param id - Credential ID
 */
export const deleteCredential = credentialsStore.delete;

// =============================================================================
// Re-exports for convenience
// =============================================================================

export type { Credential } from '../services/generated/credentials/typescript';
export type { StoreItem, SourceStatus } from './core';
