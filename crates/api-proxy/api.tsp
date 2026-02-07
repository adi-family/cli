/**
 * API Proxy HTTP API
 *
 * LLM API proxy with BYOK and platform key modes, usage tracking, and scripting.
 */

import "@typespec/http";

using TypeSpec.Http;

namespace ApiProxy;

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
  providerType: ProviderType;
  baseUrl?: string;
  isActive: boolean;
  createdAt: utcDateTime;
  updatedAt: utcDateTime;
}

model CreateKeyRequest {
  name: string;
  providerType: ProviderType;
  apiKey: string;
  baseUrl?: string;
}

model CreateKeyResponse {
  key: UpstreamApiKeySummary;
}

model UpdateKeyRequest {
  name?: string;
  apiKey?: string;
  baseUrl?: string;
  isActive?: boolean;
}

model VerifyKeyResponse {
  valid: boolean;
  models?: string[];
  error?: string;
}

// -- Models: Platform Keys --

model PlatformKeySummary {
  id: uuid;
  providerType: ProviderType;
  baseUrl?: string;
  isActive: boolean;
  createdAt: utcDateTime;
  updatedAt: utcDateTime;
}

model UpsertPlatformKeyRequest {
  providerType: ProviderType;
  apiKey: string;
  baseUrl?: string;
}

model UpdatePlatformKeyRequest {
  isActive?: boolean;
}

// -- Models: Proxy Tokens --

model ProxyTokenSummary {
  id: uuid;
  name: string;
  tokenPrefix: string;
  keyMode: KeyMode;
  upstreamKeyId?: uuid;
  platformProvider?: ProviderType;
  allowedModels?: string[];
  blockedModels?: string[];
  logRequests: boolean;
  logResponses: boolean;
  isActive: boolean;
  expiresAt?: utcDateTime;
  createdAt: utcDateTime;
}

model CreateTokenRequest {
  name: string;
  keyMode: KeyMode;
  upstreamKeyId?: uuid;
  platformProvider?: ProviderType;
  requestScript?: string;
  responseScript?: string;
  allowedModels?: string[];
  blockedModels?: string[];
  logRequests?: boolean;
  logResponses?: boolean;
  expiresAt?: utcDateTime;
}

model CreateTokenResponse {
  token: ProxyTokenSummary;
  secret: string;
}

model UpdateTokenRequest {
  name?: string;
  requestScript?: string;
  responseScript?: string;
  allowedModels?: string[];
  blockedModels?: string[];
  logRequests?: boolean;
  logResponses?: boolean;
  isActive?: boolean;
  expiresAt?: utcDateTime;
}

model RotateTokenResponse {
  token: ProxyTokenSummary;
  secret: string;
}

// -- Models: Providers --

model AllowedModelInfo {
  modelId: string;
  displayName?: string;
}

model ProviderSummary {
  providerType: ProviderType;
  isAvailable: boolean;
  allowedModels: AllowedModelInfo[];
}

model ListProvidersResponse {
  providers: ProviderSummary[];
}

// -- Models: Usage --

model UsageQuery {
  @query proxyTokenId?: uuid;
  @query from?: string;
  @query to?: string;
  @query limit?: int64;
  @query offset?: int64;
}

model ProxyUsageLog {
  id: uuid;
  proxyTokenId: uuid;
  userId: uuid;
  requestId: string;
  upstreamRequestId?: string;
  requestedModel?: string;
  actualModel?: string;
  providerType: ProviderType;
  keyMode: KeyMode;
  inputTokens?: int32;
  outputTokens?: int32;
  totalTokens?: int32;
  reportedCostUsd?: string;
  endpoint: string;
  isStreaming: boolean;
  latencyMs?: int32;
  ttftMs?: int32;
  status: RequestStatus;
  statusCode?: int16;
  errorType?: string;
  errorMessage?: string;
  requestBody?: Record<unknown>;
  responseBody?: Record<unknown>;
  createdAt: utcDateTime;
}

model UsageResponse {
  logs: ProxyUsageLog[];
  total: int64;
}

model ExportQuery {
  @query proxyTokenId?: uuid;
  @query from?: string;
  @query to?: string;
  @query format?: string;
}

model DeletedResponse {
  deleted: boolean;
}

// -- Models: v1 Proxy --

model ModelData {
  id: string;
  object: string;
  ownedBy: string;
  contextLength?: int32;
}

model ModelsResponse {
  object: string;
  data: ModelData[];
}

// -- Interfaces --

@route("/api/proxy/keys")
interface KeysService {
  @post
  createKey(@body body: CreateKeyRequest): {
    @statusCode statusCode: 201;
    @body body: CreateKeyResponse;
  };

  @get
  listKeys(): {
    @statusCode statusCode: 200;
    @body body: UpstreamApiKeySummary[];
  };

  @get
  @route("/{id}")
  getKey(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: UpstreamApiKeySummary;
  };

  @patch
  @route("/{id}")
  updateKey(@path id: uuid, @body body: UpdateKeyRequest): {
    @statusCode statusCode: 200;
    @body body: UpstreamApiKeySummary;
  };

  @delete
  @route("/{id}")
  deleteKey(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: DeletedResponse;
  };

  @post
  @route("/{id}/verify")
  verifyKey(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: VerifyKeyResponse;
  };
}

@route("/api/proxy/platform-keys")
interface PlatformKeysService {
  @get
  listPlatformKeys(): {
    @statusCode statusCode: 200;
    @body body: PlatformKeySummary[];
  };

  @post
  upsertPlatformKey(@body body: UpsertPlatformKeyRequest): {
    @statusCode statusCode: 200;
    @body body: PlatformKeySummary;
  };

  @patch
  @route("/{id}")
  updatePlatformKey(@path id: uuid, @body body: UpdatePlatformKeyRequest): {
    @statusCode statusCode: 200;
    @body body: PlatformKeySummary;
  };

  @delete
  @route("/{id}")
  deletePlatformKey(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: DeletedResponse;
  };

  @post
  @route("/{providerType}/verify")
  verifyPlatformKey(@path providerType: string): {
    @statusCode statusCode: 200;
    @body body: VerifyKeyResponse;
  };
}

@route("/api/proxy/tokens")
interface TokensService {
  @post
  createToken(@body body: CreateTokenRequest): {
    @statusCode statusCode: 201;
    @body body: CreateTokenResponse;
  };

  @get
  listTokens(): {
    @statusCode statusCode: 200;
    @body body: ProxyTokenSummary[];
  };

  @get
  @route("/{id}")
  getToken(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: ProxyTokenSummary;
  };

  @patch
  @route("/{id}")
  updateToken(@path id: uuid, @body body: UpdateTokenRequest): {
    @statusCode statusCode: 200;
    @body body: ProxyTokenSummary;
  };

  @delete
  @route("/{id}")
  deleteToken(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: DeletedResponse;
  };

  @post
  @route("/{id}/rotate")
  rotateToken(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: RotateTokenResponse;
  };
}

@route("/api/proxy/providers")
interface ProvidersService {
  @get
  listProviders(): {
    @statusCode statusCode: 200;
    @body body: ListProvidersResponse;
  };
}

@route("/api/proxy/usage")
interface UsageService {
  @get
  queryUsage(...UsageQuery): {
    @statusCode statusCode: 200;
    @body body: UsageResponse;
  };

  @get
  @route("/summary")
  usageSummary(...UsageQuery): {
    @statusCode statusCode: 200;
    @body body: Record<unknown>;
  };

  @get
  @route("/export")
  exportUsage(...ExportQuery): {
    @statusCode statusCode: 200;
    @body body: string;
  };
}

@route("/v1")
interface ProxyService {
  @post
  @route("/chat/completions")
  chatCompletions(@body body: Record<unknown>): {
    @statusCode statusCode: 200;
    @body body: Record<unknown>;
  };

  @post
  @route("/completions")
  completions(@body body: Record<unknown>): {
    @statusCode statusCode: 200;
    @body body: Record<unknown>;
  };

  @post
  @route("/embeddings")
  embeddings(@body body: Record<unknown>): {
    @statusCode statusCode: 200;
    @body body: Record<unknown>;
  };

  @post
  @route("/messages")
  messages(@body body: Record<unknown>): {
    @statusCode statusCode: 200;
    @body body: Record<unknown>;
  };

  @get
  @route("/models")
  listModels(): {
    @statusCode statusCode: 200;
    @body body: ModelsResponse;
  };
}
