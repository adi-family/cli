import type { Connection } from '@adi-family/cocoon-plugin-interface';
import type {
  Credential,
  CredentialAccessLog,
  CredentialType,
  CredentialWithData,
  DeleteResult,
  VerifyResult,
} from './types.js';

const SVC = 'adi.credentials';

export const listCredentials = (c: Connection, params?: { credential_type?: CredentialType; provider?: string }) =>
  c.request<Credential[]>(SVC, 'list', params ?? {});

export const getCredential = (c: Connection, id: string) =>
  c.request<Credential>(SVC, 'get', { id });

export const getCredentialWithData = (c: Connection, id: string) =>
  c.request<CredentialWithData>(SVC, 'getWithData', { id });

export const createCredential = (c: Connection, params: {
  name: string;
  credential_type: CredentialType;
  data: Record<string, unknown>;
  description?: string;
  metadata?: Record<string, unknown>;
  provider?: string;
  expires_at?: string;
}) => c.request<Credential>(SVC, 'create', params);

export const updateCredential = (c: Connection, params: {
  id: string;
  name?: string;
  description?: string;
  data?: Record<string, unknown>;
  metadata?: Record<string, unknown>;
  provider?: string;
  expires_at?: string;
}) => c.request<Credential>(SVC, 'update', params);

export const deleteCredential = (c: Connection, id: string) =>
  c.request<DeleteResult>(SVC, 'delete', { id });

export const verifyCredential = (c: Connection, id: string) =>
  c.request<VerifyResult>(SVC, 'verify', { id });

export const getAccessLogs = (c: Connection, id: string) =>
  c.request<CredentialAccessLog[]>(SVC, 'accessLogs', { id });
