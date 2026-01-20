/**
 * Auto-generated API client from TypeSpec.
 * DO NOT EDIT.
 */

import type { Credential, CredentialWithData, CreateCredential, UpdateCredential, CredentialAccessLog, VerifyResult, DeleteResult } from './models';
import { CredentialType } from './enums';


export class ApiError extends Error {
  constructor(
    public statusCode: number,
    public code: string,
    message: string
  ) {
    super(message);
  }
}

export interface ClientConfig {
  baseUrl: string;
  accessToken?: string;
  fetch?: typeof fetch;
}

export class BaseClient {
  private baseUrl: string;
  private accessToken?: string;
  private fetchFn: typeof fetch;

  constructor(config: ClientConfig) {
    this.baseUrl = config.baseUrl.replace(/\/$/, '');
    this.accessToken = config.accessToken;
    this.fetchFn = config.fetch ?? fetch;
  }

  setAccessToken(token: string) {
    this.accessToken = token;
  }

  protected async request<T>(
    method: string,
    path: string,
    options: { body?: unknown; query?: Record<string, unknown> } = {}
  ): Promise<T> {
    const url = new URL(path, this.baseUrl);
    if (options.query) {
      for (const [k, v] of Object.entries(options.query)) {
        if (v !== undefined) url.searchParams.set(k, String(v));
      }
    }

    const headers: Record<string, string> = { 'Content-Type': 'application/json' };
    if (this.accessToken) {
      headers['Authorization'] = `Bearer ${this.accessToken}`;
    }

    const resp = await this.fetchFn(url.toString(), {
      method,
      headers,
      body: options.body ? JSON.stringify(options.body) : undefined,
    });

    if (!resp.ok) {
      const err = await resp.json().catch(() => ({}));
      throw new ApiError(resp.status, err.code ?? 'ERROR', err.message ?? resp.statusText);
    }

    if (resp.status === 204) return undefined as T;
    return resp.json();
  }
}


export class CredentialsServiceClient extends BaseClient {

  async list(credentialType?: CredentialType, provider?: string): Promise<Credential[]> {
    const path = `/credentials`;
    return this.request('GET', path, { query: { credentialType, provider } });
  }

  async create(body: CreateCredential): Promise<Credential> {
    const path = `/credentials`;
    return this.request('POST', path, { body: body });
  }

  async get(id: string): Promise<Credential> {
    const path = `/credentials/${id}`;
    return this.request('GET', path);
  }

  async update(id: string, body: UpdateCredential): Promise<Credential> {
    const path = `/credentials/${id}`;
    return this.request('PUT', path, { body: body });
  }

  async delete(id: string): Promise<DeleteResult> {
    const path = `/credentials/${id}`;
    return this.request('DELETE', path);
  }

  async getWithData(id: string): Promise<CredentialWithData> {
    const path = `/credentials/${id}/data`;
    return this.request('GET', path);
  }

  async getAccessLogs(id: string): Promise<CredentialAccessLog[]> {
    const path = `/credentials/${id}/logs`;
    return this.request('GET', path);
  }

  async verify(id: string): Promise<VerifyResult> {
    const path = `/credentials/${id}/verify`;
    return this.request('GET', path);
  }
}

export class Client extends BaseClient {
  readonly credentialsService: CredentialsServiceClient;

  constructor(config: ClientConfig) {
    super(config);
    this.credentialsService = new CredentialsServiceClient(config);
  }
}
