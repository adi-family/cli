/**
 * Eventbus registry for credentials plugin.
 */

import type {
  Credential,
  CredentialAccessLog,
  CredentialWithData,
  VerifyResult,
} from '../types';
import type { CredentialType } from './types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    'credentials:list':    { credential_type?: CredentialType; provider?: string };
    'credentials:get':     { id: string; cocoonId: string };
    'credentials:reveal':  { id: string; cocoonId: string };
    'credentials:create':  {
      cocoonId: string;
      name: string;
      credential_type: CredentialType;
      data: Record<string, unknown>;
      description?: string;
      provider?: string;
      expires_at?: string;
    };
    'credentials:update':  {
      cocoonId: string;
      id: string;
      name?: string;
      description?: string;
      data?: Record<string, unknown>;
      provider?: string;
      expires_at?: string;
    };
    'credentials:delete':  { id: string; cocoonId: string };
    'credentials:verify':  { id: string; cocoonId: string };
    'credentials:logs':    { id: string; cocoonId: string };

    'credentials:list-changed':    { credentials: Credential[] };
    'credentials:detail-changed':  { credential: Credential };
    'credentials:data-revealed':   { credential: CredentialWithData };
    'credentials:mutated':         { credential: Credential };
    'credentials:deleted':         { id: string; cocoonId: string };
    'credentials:verified':        { id: string; result: VerifyResult };
    'credentials:logs-changed':    { id: string; logs: CredentialAccessLog[] };
  }
}
