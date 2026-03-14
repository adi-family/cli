/**
 * ADI Embed Proxy - AdiService Management API
 *
 * Key/token/usage management via @channel (WebRTC).
 * Actual /v1/embeddings proxy forwarding is in the HTTP server (api-http.tsp).
 */

@format("uuid")
scalar uuid extends string;

// -- Enums --

enum ProviderType {
  openai,
  cohere,
  google,
  custom,
}

enum KeyMode {
  byok,
  platform,
}

enum RequestStatus {
  success,
  error,
  upstreamError: "upstream_error",
}

// -- Models: Upstream API Keys --

model UpstreamApiKeySummary {
  id: uuid;
  name: string;
  provider_type: ProviderType;
  base_url?: string;
  is_active: boolean;
  created_at: utcDateTime;
  updated_at: utcDateTime;
}

// -- Models: Platform Keys --

model PlatformKeySummary {
  id: uuid;
  provider_type: ProviderType;
  base_url?: string;
  is_active: boolean;
  created_at: utcDateTime;
  updated_at: utcDateTime;
}

// -- Models: Proxy Tokens --

model ProxyTokenSummary {
  id: uuid;
  name: string;
  token_prefix: string;
  key_mode: KeyMode;
  upstream_key_id?: uuid;
  platform_provider?: ProviderType;
  allowed_models?: string[];
  blocked_models?: string[];
  log_requests: boolean;
  log_responses: boolean;
  is_active: boolean;
  expires_at?: utcDateTime;
  created_at: utcDateTime;
}

model CreateTokenResponse {
  token: ProxyTokenSummary;
  secret: string;
}

model RotateTokenResponse {
  token: ProxyTokenSummary;
  secret: string;
}

// -- Models: Verification --

model VerifyKeyResponse {
  valid: boolean;
  models?: string[];
  error?: string;
}

// -- Models: Providers --

model AllowedModelInfo {
  model_id: string;
  display_name?: string;
}

model ProviderSummary {
  provider_type: ProviderType;
  is_available: boolean;
  allowed_models: AllowedModelInfo[];
}

// -- Models: Usage --

model UsageLogEntry {
  id: uuid;
  proxy_token_id: uuid;
  user_id: uuid;
  request_id: string;
  upstream_request_id?: string;
  requested_model?: string;
  actual_model?: string;
  provider_type: ProviderType;
  key_mode: KeyMode;
  input_tokens?: int32;
  total_tokens?: int32;
  dimensions?: int32;
  input_count?: int32;
  reported_cost_usd?: string;
  endpoint: string;
  latency_ms?: int32;
  status: RequestStatus;
  status_code?: int16;
  error_type?: string;
  error_message?: string;
  created_at: utcDateTime;
}

model UsageResponse {
  logs: UsageLogEntry[];
  total: int64;
}

// -- Models: Common --

model DeletedResponse {
  deleted: boolean;
}

// -- Channel: AdiService management API --

@channel("adi.embed-proxy")
interface EmbedProxyService {
  // Upstream key management (user's own keys)
  @request listKeys(): UpstreamApiKeySummary[];
  @request getKey(id: uuid): UpstreamApiKeySummary;
  @request createKey(name: string, provider_type: ProviderType, api_key: string, base_url?: string): UpstreamApiKeySummary;
  @request updateKey(id: uuid, name?: string, api_key?: string, base_url?: string, is_active?: boolean): UpstreamApiKeySummary;
  @request deleteKey(id: uuid): DeletedResponse;
  @request verifyKey(id: uuid): VerifyKeyResponse;

  // Platform key management (admin)
  @request listPlatformKeys(): PlatformKeySummary[];
  @request upsertPlatformKey(provider_type: ProviderType, api_key: string, base_url?: string): PlatformKeySummary;
  @request updatePlatformKey(id: uuid, is_active?: boolean): PlatformKeySummary;
  @request deletePlatformKey(id: uuid): DeletedResponse;

  // Token management
  @request listTokens(): ProxyTokenSummary[];
  @request getToken(id: uuid): ProxyTokenSummary;
  @request createToken(
    name: string,
    key_mode: KeyMode,
    upstream_key_id?: uuid,
    platform_provider?: ProviderType,
    allowed_models?: string[],
    blocked_models?: string[],
    log_requests?: boolean,
    log_responses?: boolean,
    expires_at?: utcDateTime,
  ): CreateTokenResponse;
  @request updateToken(
    id: uuid,
    name?: string,
    allowed_models?: string[],
    blocked_models?: string[],
    log_requests?: boolean,
    log_responses?: boolean,
    is_active?: boolean,
    expires_at?: utcDateTime,
  ): ProxyTokenSummary;
  @request deleteToken(id: uuid): DeletedResponse;
  @request rotateToken(id: uuid): RotateTokenResponse;

  // Providers
  @request listProviders(): ProviderSummary[];

  // Usage
  @request queryUsage(
    proxy_token_id?: uuid,
    from?: string,
    to?: string,
    limit?: int64,
    offset?: int64,
  ): UsageResponse;
}
