/**
 * Auto-generated ADI service client from TypeSpec.
 * DO NOT EDIT.
 */
import type { Connection } from '@adi-family/cocoon-plugin-interface';
import type { Credential, CredentialAccessLog, CredentialWithData, VerifyResult } from './models.js';
import { CredentialType } from './enums.js';

const SVC_ADI_CREDENTIALS = 'adi.credentials';

export const list = (c: Connection, params?: { credential_type?: CredentialType; provider?: string; }) =>
  c.request<Credential[]>(SVC_ADI_CREDENTIALS, 'list', params ?? {});

export const get = (c: Connection, id: string) =>
  c.request<Credential>(SVC_ADI_CREDENTIALS, 'get', { id });

export const getWithData = (c: Connection, id: string) =>
  c.request<CredentialWithData>(SVC_ADI_CREDENTIALS, 'get_with_data', { id });

export const create = (c: Connection, params: { name: string; credential_type: CredentialType; data: Record<string, unknown>; description?: string; metadata?: Record<string, unknown>; provider?: string; expires_at?: string; }) =>
  c.request<Credential>(SVC_ADI_CREDENTIALS, 'create', params);

export const update = (c: Connection, params: { id: string; name?: string; description?: string; data?: Record<string, unknown>; metadata?: Record<string, unknown>; provider?: string; expires_at?: string; }) =>
  c.request<Credential>(SVC_ADI_CREDENTIALS, 'update', params);

export const delete_ = (c: Connection, id: string) =>
  c.request<DeleteResult>(SVC_ADI_CREDENTIALS, 'delete', { id });

export const verify = (c: Connection, id: string) =>
  c.request<VerifyResult>(SVC_ADI_CREDENTIALS, 'verify', { id });

export const accessLogs = (c: Connection, id: string) =>
  c.request<CredentialAccessLog[]>(SVC_ADI_CREDENTIALS, 'access_logs', { id });
