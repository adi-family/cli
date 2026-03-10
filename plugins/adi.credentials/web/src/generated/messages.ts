/**
 * Auto-generated protocol messages from TypeSpec.
 * DO NOT EDIT.
 */

import type { CredentialType } from './types';

export type SignalingMessage =
  // ── credentials ──
  | { type: 'credentials_list'; credential_type?: CredentialType; provider?: string }
  | { type: 'credentials_get'; id: string }
  | { type: 'credentials_get_response'; id: string; name: string; description?: string; credential_type: CredentialType; metadata: Record<string, unknown>; provider?: string; expires_at?: string; created_at: string; updated_at: string; last_used_at?: string }
  | { type: 'credentials_get_with_data'; id: string }
  | { type: 'credentials_get_with_data_response'; data: Record<string, unknown> }
  | { type: 'credentials_create'; name: string; credential_type: CredentialType; data: Record<string, unknown>; description?: string; metadata?: Record<string, unknown>; provider?: string; expires_at?: string }
  | { type: 'credentials_create_response'; id: string; name: string; description?: string; credential_type: CredentialType; metadata: Record<string, unknown>; provider?: string; expires_at?: string; created_at: string; updated_at: string; last_used_at?: string }
  | { type: 'credentials_update'; id: string; name?: string; description?: string; data?: Record<string, unknown>; metadata?: Record<string, unknown>; provider?: string; expires_at?: string }
  | { type: 'credentials_update_response'; id: string; name: string; description?: string; credential_type: CredentialType; metadata: Record<string, unknown>; provider?: string; expires_at?: string; created_at: string; updated_at: string; last_used_at?: string }
  | { type: 'credentials_delete'; id: string }
  | { type: 'credentials_delete_response'; deleted: boolean }
  | { type: 'credentials_verify'; id: string }
  | { type: 'credentials_verify_response'; valid: boolean; is_expired: boolean; expires_at?: string }
  | { type: 'credentials_access_logs'; id: string };
