/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { AdiLlmProxyCreateKeyEvent, AdiLlmProxyCreateTokenEvent, AdiLlmProxyDeleteKeyEvent, AdiLlmProxyDeleteTokenEvent, AdiLlmProxyErrorEvent, AdiLlmProxyKeyDeletedEvent, AdiLlmProxyKeyVerifiedEvent, AdiLlmProxyKeysChangedEvent, AdiLlmProxyListKeysEvent, AdiLlmProxyListProvidersEvent, AdiLlmProxyListTokensEvent, AdiLlmProxyProvidersChangedEvent, AdiLlmProxyQueryUsageEvent, AdiLlmProxyRotateTokenEvent, AdiLlmProxyTokenCreatedEvent, AdiLlmProxyTokenDeletedEvent, AdiLlmProxyTokenRotatedEvent, AdiLlmProxyTokensChangedEvent, AdiLlmProxyUsageLoadedEvent, AdiLlmProxyVerifyKeyEvent } from './bus-types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── adi.llm-proxy ──
    'adi.llm-proxy:list-keys': AdiLlmProxyListKeysEvent;
    'adi.llm-proxy:list-tokens': AdiLlmProxyListTokensEvent;
    'adi.llm-proxy:create-key': AdiLlmProxyCreateKeyEvent;
    'adi.llm-proxy:delete-key': AdiLlmProxyDeleteKeyEvent;
    'adi.llm-proxy:verify-key': AdiLlmProxyVerifyKeyEvent;
    'adi.llm-proxy:create-token': AdiLlmProxyCreateTokenEvent;
    'adi.llm-proxy:delete-token': AdiLlmProxyDeleteTokenEvent;
    'adi.llm-proxy:rotate-token': AdiLlmProxyRotateTokenEvent;
    'adi.llm-proxy:query-usage': AdiLlmProxyQueryUsageEvent;
    'adi.llm-proxy:list-providers': AdiLlmProxyListProvidersEvent;
    'adi.llm-proxy:keys-changed': AdiLlmProxyKeysChangedEvent;
    'adi.llm-proxy:tokens-changed': AdiLlmProxyTokensChangedEvent;
    'adi.llm-proxy:token-created': AdiLlmProxyTokenCreatedEvent;
    'adi.llm-proxy:token-rotated': AdiLlmProxyTokenRotatedEvent;
    'adi.llm-proxy:key-verified': AdiLlmProxyKeyVerifiedEvent;
    'adi.llm-proxy:providers-changed': AdiLlmProxyProvidersChangedEvent;
    'adi.llm-proxy:usage-loaded': AdiLlmProxyUsageLoadedEvent;
    'adi.llm-proxy:key-deleted': AdiLlmProxyKeyDeletedEvent;
    'adi.llm-proxy:token-deleted': AdiLlmProxyTokenDeletedEvent;
    'adi.llm-proxy:error': AdiLlmProxyErrorEvent;
  }
}
