/**
 * Auto-generated protocol messages from TypeSpec.
 * DO NOT EDIT.
 */

import type { AuthOption, AuthRequirement, ConnectionInfo, DeviceInfo } from './types';

export type SignalingMessage =
  // ── auth ──
  | { type: 'auth_hello'; auth_kind: string; auth_domain: string; auth_requirement: AuthRequirement; auth_options: AuthOption[] }
  | { type: 'auth_authenticate'; access_token: string }
  | { type: 'auth_authenticate_response'; user_id: string }
  | { type: 'auth_hello_authed'; user_id: string; connection_info: ConnectionInfo }

  // ── device ──
  | { type: 'device_register'; secret: string; device_id?: string; version: string; tags?: Record<string, string> }
  | { type: 'device_register_response'; device_id: string; tags?: Record<string, string> }
  | { type: 'device_deregister'; device_id: string; reason?: string }
  | { type: 'device_deregister_response'; device_id: string }
  | { type: 'device_peer_connected'; peer_id: string }
  | { type: 'device_peer_disconnected'; peer_id: string }
  | { type: 'device_update_tags'; tags: Record<string, string> }
  | { type: 'device_update_tags_response'; device_id: string; tags: Record<string, string> }
  | { type: 'device_query_devices'; tag_filter: Record<string, string> }
  | { type: 'device_query_devices_response'; devices: DeviceInfo[] }

  // ── pairing ──
  | { type: 'pairing_create_code' }
  | { type: 'pairing_create_code_response'; code: string }
  | { type: 'pairing_use_code'; code: string }
  | { type: 'pairing_use_code_response'; peer_id: string }
  | { type: 'pairing_failed'; reason: string }

  // ── sync ──
  | { type: 'sync_data'; payload: unknown }

  // ── system ──
  | { type: 'system_error'; message: string };
