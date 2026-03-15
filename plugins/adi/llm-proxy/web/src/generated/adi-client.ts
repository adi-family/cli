/**
 * Auto-generated ADI service client from TypeSpec.
 * DO NOT EDIT.
 */
import type { Connection } from '@adi-family/cocoon-plugin-interface';
import type { CreateTokenResponse, DeletedResponse, PlatformKeySummary, ProviderSummary, ProxyTokenSummary, RotateTokenResponse, UpstreamApiKeySummary, UsageResponse, VerifyKeyResponse } from './models.js';
import { KeyMode, ProviderType } from './enums.js';

const SVC_ADI_LLM_PROXY = 'adi.llm-proxy';

export const listKeys = (c: Connection) =>
  c.request<UpstreamApiKeySummary[]>(SVC_ADI_LLM_PROXY, 'list_keys', {});

export const getKey = (c: Connection, id: string) =>
  c.request<UpstreamApiKeySummary>(SVC_ADI_LLM_PROXY, 'get_key', { id });

export const createKey = (c: Connection, params: { name: string; provider_type: ProviderType; api_key: string; base_url?: string; }) =>
  c.request<UpstreamApiKeySummary>(SVC_ADI_LLM_PROXY, 'create_key', params);

export const updateKey = (c: Connection, params: { id: string; name?: string; api_key?: string; base_url?: string; is_active?: boolean; }) =>
  c.request<UpstreamApiKeySummary>(SVC_ADI_LLM_PROXY, 'update_key', params);

export const deleteKey = (c: Connection, id: string) =>
  c.request<DeletedResponse>(SVC_ADI_LLM_PROXY, 'delete_key', { id });

export const verifyKey = (c: Connection, id: string) =>
  c.request<VerifyKeyResponse>(SVC_ADI_LLM_PROXY, 'verify_key', { id });

export const listPlatformKeys = (c: Connection) =>
  c.request<PlatformKeySummary[]>(SVC_ADI_LLM_PROXY, 'list_platform_keys', {});

export const upsertPlatformKey = (c: Connection, params: { provider_type: ProviderType; api_key: string; base_url?: string; }) =>
  c.request<PlatformKeySummary>(SVC_ADI_LLM_PROXY, 'upsert_platform_key', params);

export const updatePlatformKey = (c: Connection, params: { id: string; is_active?: boolean; }) =>
  c.request<PlatformKeySummary>(SVC_ADI_LLM_PROXY, 'update_platform_key', params);

export const deletePlatformKey = (c: Connection, id: string) =>
  c.request<DeletedResponse>(SVC_ADI_LLM_PROXY, 'delete_platform_key', { id });

export const listTokens = (c: Connection) =>
  c.request<ProxyTokenSummary[]>(SVC_ADI_LLM_PROXY, 'list_tokens', {});

export const getToken = (c: Connection, id: string) =>
  c.request<ProxyTokenSummary>(SVC_ADI_LLM_PROXY, 'get_token', { id });

export const createToken = (c: Connection, params: { name: string; key_mode: KeyMode; upstream_key_id?: string; platform_provider?: ProviderType; request_script?: string; response_script?: string; allowed_models?: string[]; blocked_models?: string[]; log_requests?: boolean; log_responses?: boolean; expires_at?: string; }) =>
  c.request<CreateTokenResponse>(SVC_ADI_LLM_PROXY, 'create_token', params);

export const updateToken = (c: Connection, params: { id: string; name?: string; request_script?: string; response_script?: string; allowed_models?: string[]; blocked_models?: string[]; log_requests?: boolean; log_responses?: boolean; is_active?: boolean; expires_at?: string; }) =>
  c.request<ProxyTokenSummary>(SVC_ADI_LLM_PROXY, 'update_token', params);

export const deleteToken = (c: Connection, id: string) =>
  c.request<DeletedResponse>(SVC_ADI_LLM_PROXY, 'delete_token', { id });

export const rotateToken = (c: Connection, id: string) =>
  c.request<RotateTokenResponse>(SVC_ADI_LLM_PROXY, 'rotate_token', { id });

export const listProviders = (c: Connection) =>
  c.request<ProviderSummary[]>(SVC_ADI_LLM_PROXY, 'list_providers', {});

export const queryUsage = (c: Connection, params?: { proxy_token_id?: string; from?: string; to?: string; limit?: number; offset?: number; }) =>
  c.request<UsageResponse>(SVC_ADI_LLM_PROXY, 'query_usage', params ?? {});

export const complete = (c: Connection, params: { proxy_token: string; endpoint: string; body: unknown; }) =>
  c.request<unknown>(SVC_ADI_LLM_PROXY, 'complete', params);
