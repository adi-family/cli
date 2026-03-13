/**
 * Auto-generated ADI service client from TypeSpec.
 * DO NOT EDIT.
 */
import type { Connection } from '@adi-family/cocoon-plugin-interface';
import type { CreateTokenResponse, DeletedResponse, PlatformKeySummary, ProviderSummary, ProxyTokenSummary, RotateTokenResponse, UpstreamApiKeySummary, UsageResponse, VerifyKeyResponse } from './models.js';
import { KeyMode, ProviderType } from './enums.js';

const SVC = 'adi.llm-proxy';

export const listKeys = (c: Connection) =>
  c.request<UpstreamApiKeySummary[]>(SVC, 'list_keys', {});

export const getKey = (c: Connection, id: string) =>
  c.request<UpstreamApiKeySummary>(SVC, 'get_key', { id });

export const createKey = (c: Connection, params: { name: string; provider_type: ProviderType; api_key: string; base_url?: string; }) =>
  c.request<UpstreamApiKeySummary>(SVC, 'create_key', params);

export const updateKey = (c: Connection, params: { id: string; name?: string; api_key?: string; base_url?: string; is_active?: boolean; }) =>
  c.request<UpstreamApiKeySummary>(SVC, 'update_key', params);

export const deleteKey = (c: Connection, id: string) =>
  c.request<DeletedResponse>(SVC, 'delete_key', { id });

export const verifyKey = (c: Connection, id: string) =>
  c.request<VerifyKeyResponse>(SVC, 'verify_key', { id });

export const listPlatformKeys = (c: Connection) =>
  c.request<PlatformKeySummary[]>(SVC, 'list_platform_keys', {});

export const upsertPlatformKey = (c: Connection, params: { provider_type: ProviderType; api_key: string; base_url?: string; }) =>
  c.request<PlatformKeySummary>(SVC, 'upsert_platform_key', params);

export const updatePlatformKey = (c: Connection, params: { id: string; is_active?: boolean; }) =>
  c.request<PlatformKeySummary>(SVC, 'update_platform_key', params);

export const deletePlatformKey = (c: Connection, id: string) =>
  c.request<DeletedResponse>(SVC, 'delete_platform_key', { id });

export const listTokens = (c: Connection) =>
  c.request<ProxyTokenSummary[]>(SVC, 'list_tokens', {});

export const getToken = (c: Connection, id: string) =>
  c.request<ProxyTokenSummary>(SVC, 'get_token', { id });

export const createToken = (c: Connection, params: { name: string; key_mode: KeyMode; upstream_key_id?: string; platform_provider?: ProviderType; request_script?: string; response_script?: string; allowed_models?: string[]; blocked_models?: string[]; log_requests?: boolean; log_responses?: boolean; expires_at?: string; }) =>
  c.request<CreateTokenResponse>(SVC, 'create_token', params);

export const updateToken = (c: Connection, params: { id: string; name?: string; request_script?: string; response_script?: string; allowed_models?: string[]; blocked_models?: string[]; log_requests?: boolean; log_responses?: boolean; is_active?: boolean; expires_at?: string; }) =>
  c.request<ProxyTokenSummary>(SVC, 'update_token', params);

export const deleteToken = (c: Connection, id: string) =>
  c.request<DeletedResponse>(SVC, 'delete_token', { id });

export const rotateToken = (c: Connection, id: string) =>
  c.request<RotateTokenResponse>(SVC, 'rotate_token', { id });

export const listProviders = (c: Connection) =>
  c.request<ProviderSummary[]>(SVC, 'list_providers', {});

export const queryUsage = (c: Connection, params?: { proxy_token_id?: string; from?: string; to?: string; limit?: number; offset?: number; }) =>
  c.request<UsageResponse>(SVC, 'query_usage', params ?? {});
