/**
 * ADI LLM Proxy - AdiService Management API
 *
 * Key/token/usage management via @channel (WebRTC).
 * Actual /v1/* proxy forwarding is in the HTTP server (api-http.tsp).
 */

@format("uuid")
scalar uuid extends string;

// -- Enums --

enum ProviderType {
  openai,
  anthropic,
  openrouter,
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
  output_tokens?: int32;
  total_tokens?: int32;
  reported_cost_usd?: string;
  endpoint: string;
  is_streaming: boolean;
  latency_ms?: int32;
  ttft_ms?: int32;
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

@channel("adi.llm-proxy")
interface LlmProxyService {
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
    request_script?: string,
    response_script?: string,
    allowed_models?: string[],
    blocked_models?: string[],
    log_requests?: boolean,
    log_responses?: boolean,
    expires_at?: utcDateTime,
  ): CreateTokenResponse;
  @request updateToken(
    id: uuid,
    name?: string,
    request_script?: string,
    response_script?: string,
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

  // LLM completion (proxied via AdiService protocol)
  @request complete(
    proxy_token: string,
    endpoint: string,
    body: unknown,
  ): unknown;
}

// -- Bus: UI event bus --

model UpstreamApiKeySummaryWithCocoon {
  ...UpstreamApiKeySummary;
  cocoonId: string;
}

model ProxyTokenSummaryWithCocoon {
  ...ProxyTokenSummary;
  cocoonId: string;
}

@bus("adi.llm-proxy")
interface LlmProxyBus {
  // Commands (UI → plugin)
  @event listKeys(cocoonId: string): void;
  @event listTokens(cocoonId: string): void;
  @event createKey(cocoonId: string, name: string, provider_type: ProviderType, api_key: string, base_url?: string): void;
  @event deleteKey(cocoonId: string, id: uuid): void;
  @event verifyKey(cocoonId: string, id: uuid): void;
  @event createToken(cocoonId: string, name: string, key_mode: KeyMode, upstream_key_id?: uuid, platform_provider?: ProviderType): void;
  @event deleteToken(cocoonId: string, id: uuid): void;
  @event rotateToken(cocoonId: string, id: uuid): void;
  @event queryUsage(cocoonId: string, proxy_token_id?: uuid, from?: string, to?: string): void;
  @event listProviders(cocoonId: string): void;

  // Responses (plugin → UI)
  @event keysChanged(keys: UpstreamApiKeySummaryWithCocoon[]): void;
  @event tokensChanged(tokens: ProxyTokenSummaryWithCocoon[]): void;
  @event tokenCreated(token: ProxyTokenSummaryWithCocoon, secret: string): void;
  @event tokenRotated(token: ProxyTokenSummaryWithCocoon, secret: string): void;
  @event keyVerified(id: uuid, valid: boolean, cocoonId: string, models?: string[], error?: string): void;
  @event providersChanged(providers: ProviderSummary[], cocoonId: string): void;
  @event usageLoaded(logs: UsageLogEntry[], total: int64, cocoonId: string): void;
  @event keyDeleted(id: uuid, cocoonId: string): void;
  @event tokenDeleted(id: uuid, cocoonId: string): void;
  @event error(message: string, event: string): void;
}
