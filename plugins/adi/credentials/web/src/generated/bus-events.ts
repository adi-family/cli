/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { AdiCredentialsCreateEvent, AdiCredentialsDataRevealedEvent, AdiCredentialsDeleteEvent, AdiCredentialsDeletedEvent, AdiCredentialsDetailChangedEvent, AdiCredentialsErrorEvent, AdiCredentialsGetEvent, AdiCredentialsListChangedEvent, AdiCredentialsListEvent, AdiCredentialsLogsChangedEvent, AdiCredentialsLogsEvent, AdiCredentialsMutatedEvent, AdiCredentialsRevealEvent, AdiCredentialsUpdateEvent, AdiCredentialsVerifiedEvent, AdiCredentialsVerifyEvent } from './bus-types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── adi.credentials ──
    'adi.credentials:list': AdiCredentialsListEvent;
    'adi.credentials:get': AdiCredentialsGetEvent;
    'adi.credentials:reveal': AdiCredentialsRevealEvent;
    'adi.credentials:create': AdiCredentialsCreateEvent;
    'adi.credentials:update': AdiCredentialsUpdateEvent;
    'adi.credentials:delete': AdiCredentialsDeleteEvent;
    'adi.credentials:verify': AdiCredentialsVerifyEvent;
    'adi.credentials:logs': AdiCredentialsLogsEvent;
    'adi.credentials:list-changed': AdiCredentialsListChangedEvent;
    'adi.credentials:detail-changed': AdiCredentialsDetailChangedEvent;
    'adi.credentials:data-revealed': AdiCredentialsDataRevealedEvent;
    'adi.credentials:mutated': AdiCredentialsMutatedEvent;
    'adi.credentials:deleted': AdiCredentialsDeletedEvent;
    'adi.credentials:verified': AdiCredentialsVerifiedEvent;
    'adi.credentials:logs-changed': AdiCredentialsLogsChangedEvent;
    'adi.credentials:error': AdiCredentialsErrorEvent;
  }
}
