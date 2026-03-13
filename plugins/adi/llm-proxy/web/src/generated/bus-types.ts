/**
 * Auto-generated eventbus types from TypeSpec.
 * DO NOT EDIT.
 */

import type { ProviderSummary, ProxyTokenSummaryWithCocoon, UpstreamApiKeySummaryWithCocoon, UsageLogEntry } from './models';

import { KeyMode, ProviderType } from './enums';

export interface AdiLlmProxyListKeysEvent {
  cocoonId: string;
}

export interface AdiLlmProxyListTokensEvent {
  cocoonId: string;
}

export interface AdiLlmProxyCreateKeyEvent {
  cocoonId: string;
  name: string;
  provider_type: ProviderType;
  api_key: string;
  base_url?: string;
}

export interface AdiLlmProxyDeleteKeyEvent {
  cocoonId: string;
  id: string;
}

export interface AdiLlmProxyVerifyKeyEvent {
  cocoonId: string;
  id: string;
}

export interface AdiLlmProxyCreateTokenEvent {
  cocoonId: string;
  name: string;
  key_mode: KeyMode;
  upstream_key_id?: string;
  platform_provider?: ProviderType;
}

export interface AdiLlmProxyDeleteTokenEvent {
  cocoonId: string;
  id: string;
}

export interface AdiLlmProxyRotateTokenEvent {
  cocoonId: string;
  id: string;
}

export interface AdiLlmProxyQueryUsageEvent {
  cocoonId: string;
  proxy_token_id?: string;
  from?: string;
  to?: string;
}

export interface AdiLlmProxyListProvidersEvent {
  cocoonId: string;
}

export interface AdiLlmProxyKeysChangedEvent {
  keys: UpstreamApiKeySummaryWithCocoon[];
}

export interface AdiLlmProxyTokensChangedEvent {
  tokens: ProxyTokenSummaryWithCocoon[];
}

export interface AdiLlmProxyTokenCreatedEvent {
  token: ProxyTokenSummaryWithCocoon;
  secret: string;
}

export interface AdiLlmProxyTokenRotatedEvent {
  token: ProxyTokenSummaryWithCocoon;
  secret: string;
}

export interface AdiLlmProxyKeyVerifiedEvent {
  id: string;
  valid: boolean;
  cocoonId: string;
  models?: string[];
  error?: string;
}

export interface AdiLlmProxyProvidersChangedEvent {
  providers: ProviderSummary[];
  cocoonId: string;
}

export interface AdiLlmProxyUsageLoadedEvent {
  logs: UsageLogEntry[];
  total: number;
  cocoonId: string;
}

export interface AdiLlmProxyKeyDeletedEvent {
  id: string;
  cocoonId: string;
}

export interface AdiLlmProxyTokenDeletedEvent {
  id: string;
  cocoonId: string;
}

export interface AdiLlmProxyErrorEvent {
  message: string;
  event: string;
}

export enum AdiLlmProxyBusKey {
  ListKeys = 'adi.llm-proxy:list-keys',
  ListTokens = 'adi.llm-proxy:list-tokens',
  CreateKey = 'adi.llm-proxy:create-key',
  DeleteKey = 'adi.llm-proxy:delete-key',
  VerifyKey = 'adi.llm-proxy:verify-key',
  CreateToken = 'adi.llm-proxy:create-token',
  DeleteToken = 'adi.llm-proxy:delete-token',
  RotateToken = 'adi.llm-proxy:rotate-token',
  QueryUsage = 'adi.llm-proxy:query-usage',
  ListProviders = 'adi.llm-proxy:list-providers',
  KeysChanged = 'adi.llm-proxy:keys-changed',
  TokensChanged = 'adi.llm-proxy:tokens-changed',
  TokenCreated = 'adi.llm-proxy:token-created',
  TokenRotated = 'adi.llm-proxy:token-rotated',
  KeyVerified = 'adi.llm-proxy:key-verified',
  ProvidersChanged = 'adi.llm-proxy:providers-changed',
  UsageLoaded = 'adi.llm-proxy:usage-loaded',
  KeyDeleted = 'adi.llm-proxy:key-deleted',
  TokenDeleted = 'adi.llm-proxy:token-deleted',
  Error = 'adi.llm-proxy:error',
}
