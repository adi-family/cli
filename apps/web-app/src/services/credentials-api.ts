// Credentials API service - Re-exports from generated TypeSpec code
//
// Generated types and client are from ./generated/credentials/typescript/
// Manual utilities (labels, icons) are from ./credentials-utils.ts

// Re-export all generated types and client
export * from './generated/credentials/typescript';

// Re-export utility functions
export { CREDENTIAL_TYPE_LABELS, CREDENTIAL_TYPE_ICONS } from './credentials-utils';

// Import for creating the configured client
import { CredentialsServiceClient } from './generated/credentials/typescript';

// Base path for credentials API - nginx routes /api/credentials/* to the service
const API_BASE = "/api/credentials";

// Create a pre-configured client instance for convenience
// Note: The generated client expects a full baseUrl, so we work around that
class ConfiguredCredentialsClient extends CredentialsServiceClient {
  constructor() {
    // Use a dummy config since we override the request method
    super({ baseUrl: '' });
  }

  protected async request<T>(
    method: string,
    path: string,
    options: { body?: unknown; query?: Record<string, unknown> } = {}
  ): Promise<T> {
    const url = new URL(path, window.location.origin);
    // Prepend API_BASE to the path (path already includes /credentials from generated client)
    url.pathname = API_BASE + path;
    
    if (options.query) {
      for (const [k, v] of Object.entries(options.query)) {
        if (v !== undefined) url.searchParams.set(k, String(v));
      }
    }

    const headers: Record<string, string> = { "Content-Type": "application/json" };

    const resp = await fetch(url.toString(), {
      method,
      headers,
      credentials: "include", // Include cookies for auth
      body: options.body ? JSON.stringify(options.body) : undefined,
    });

    if (!resp.ok) {
      const err = await resp.json().catch(() => ({ message: "Request failed" }));
      throw new Error(err.message || `HTTP ${resp.status}`);
    }

    if (resp.status === 204) return undefined as T;
    return resp.json();
  }
}

// Export a pre-configured client instance
export const credentialsApi = new ConfiguredCredentialsClient();
