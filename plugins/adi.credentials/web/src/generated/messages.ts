/**
 * Auto-generated protocol messages from TypeSpec.
 * DO NOT EDIT.
 */

import type { CredentialType } from './types';

export type SignalingMessage =
  // ── adi.credentials ──
  | { type: 'adi.credentials_list'; credential_type?: CredentialType; provider?: string }
  | { type: 'adi.credentials_get'; id: string }
  | { type: 'adi.credentials_get_response'; id: string; name: string; description?: string; credential_type: CredentialType; metadata: Record<string, unknown>; provider?: string; expires_at?: string; created_at: string; updated_at: string; last_used_at?: string }
  | { type: 'adi.credentials_get_with_data'; id: string }
  | { type: 'adi.credentials_get_with_data_response'; data: Record<string, unknown> }
  | { type: 'adi.credentials_create'; name: string; credential_type: CredentialType; data: Record<string, unknown>; description?: string; metadata?: Record<string, unknown>; provider?: string; expires_at?: string }
  | { type: 'adi.credentials_create_response'; id: string; name: string; description?: string; credential_type: CredentialType; metadata: Record<string, unknown>; provider?: string; expires_at?: string; created_at: string; updated_at: string; last_used_at?: string }
  | { type: 'adi.credentials_update'; id: string; name?: string; description?: string; data?: Record<string, unknown>; metadata?: Record<string, unknown>; provider?: string; expires_at?: string }
  | { type: 'adi.credentials_update_response'; id: string; name: string; description?: string; credential_type: CredentialType; metadata: Record<string, unknown>; provider?: string; expires_at?: string; created_at: string; updated_at: string; last_used_at?: string }
  | { type: 'adi.credentials_delete'; id: string }
  | { type: 'adi.credentials_delete_response'; deleted: boolean }
  | { type: 'adi.credentials_verify'; id: string }
  | { type: 'adi.credentials_verify_response'; valid: boolean; is_expired: boolean; expires_at?: string }
  | { type: 'adi.credentials_access_logs'; id: string };
