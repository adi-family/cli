/**
 * Auto-generated models from TypeSpec.
 * DO NOT EDIT.
 */

import { ProviderType, KeyMode, RequestStatus } from './enums';

export interface UpstreamApiKeySummary {
  id: string;
  name: string;
  provider_type: ProviderType;
  base_url?: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface PlatformKeySummary {
  id: string;
  provider_type: ProviderType;
  base_url?: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface ProxyTokenSummary {
  id: string;
  name: string;
  token_prefix: string;
  key_mode: KeyMode;
  upstream_key_id?: string;
  platform_provider?: ProviderType;
  allowed_models?: string[];
  blocked_models?: string[];
  log_requests: boolean;
  log_responses: boolean;
  is_active: boolean;
  expires_at?: string;
  created_at: string;
}

export interface CreateTokenResponse {
  token: ProxyTokenSummary;
  secret: string;
}

export interface RotateTokenResponse {
  token: ProxyTokenSummary;
  secret: string;
}

export interface VerifyKeyResponse {
  valid: boolean;
  models?: string[];
  error?: string;
}

export interface AllowedModelInfo {
  model_id: string;
  display_name?: string;
}

export interface ProviderSummary {
  provider_type: ProviderType;
  is_available: boolean;
  allowed_models: AllowedModelInfo[];
}

export interface UsageLogEntry {
  id: string;
  proxy_token_id: string;
  user_id: string;
  request_id: string;
  upstream_request_id?: string;
  requested_model?: string;
  actual_model?: string;
  provider_type: ProviderType;
  key_mode: KeyMode;
  input_tokens?: number;
  total_tokens?: number;
  dimensions?: number;
  input_count?: number;
  reported_cost_usd?: string;
  endpoint: string;
  latency_ms?: number;
  status: RequestStatus;
  status_code?: number;
  error_type?: string;
  error_message?: string;
  created_at: string;
}

export interface UsageResponse {
  logs: UsageLogEntry[];
  total: number;
}

export interface DeletedResponse {
  deleted: boolean;
}
