/**
 * Auto-generated protocol messages from TypeSpec.
 * DO NOT EDIT.
 */

import type { AuthOption, AuthRequirement, CocoonKind, ConnectionInfo, DeviceInfo, RoomInfo } from './types';

export type SignalingMessage =
  // ── auth ──
  | { type: 'auth_hello'; auth_kind: string; auth_domain: string; auth_requirement: AuthRequirement; auth_options: AuthOption[] }
  | { type: 'auth_authenticate'; access_token: string }
  | { type: 'auth_authenticate_response'; user_id: string }
  | { type: 'auth_hello_authed'; user_id: string; connection_info: ConnectionInfo; devices: DeviceInfo[] }

  // ── device ──
  | { type: 'device_register'; secret: string; device_id?: string; version: string; tags?: Record<string, string>; device_type?: string; device_config?: unknown }
  | { type: 'device_register_response'; device_id: string; tags?: Record<string, string> }
  | { type: 'device_deregister'; device_id: string; reason?: string }
  | { type: 'device_deregister_response'; device_id: string }
  | { type: 'device_peer_connected'; peer_id: string }
  | { type: 'device_peer_disconnected'; peer_id: string }
  | { type: 'device_update_tags'; tags: Record<string, string> }
  | { type: 'device_update_tags_response'; device_id: string; tags: Record<string, string> }
  | { type: 'device_update_device'; tags?: Record<string, string>; device_config?: unknown }
  | { type: 'device_update_device_response'; device_id: string; tags: Record<string, string>; device_config?: unknown }
  | { type: 'device_query_devices'; tag_filter: Record<string, string> }
  | { type: 'device_query_devices_response'; devices: DeviceInfo[] }
  | { type: 'device_device_list_updated'; devices: DeviceInfo[] }

  // ── pairing ──
  | { type: 'pairing_create_code' }
  | { type: 'pairing_create_code_response'; code: string }
  | { type: 'pairing_use_code'; code: string }
  | { type: 'pairing_use_code_response'; peer_id: string }
  | { type: 'pairing_failed'; reason: string }

  // ── sync ──
  | { type: 'sync_data'; payload: unknown }

  // ── hive ──
  | { type: 'hive_register'; hive_id: string; version: string; cocoon_kinds: CocoonKind[]; hive_id_signature: string }
  | { type: 'hive_register_response'; hive_id: string }
  | { type: 'hive_spawn_cocoon'; request_id: string; setup_token: string; name?: string; kind: string }
  | { type: 'hive_terminate_cocoon'; request_id: string; container_id: string }
  | { type: 'hive_spawn_cocoon_result'; request_id: string; success: boolean; device_id?: string; container_id?: string; error?: string }
  | { type: 'hive_terminate_cocoon_result'; request_id: string; success: boolean; error?: string }

  // ── room ──
  | { type: 'room_create'; room_id?: string }
  | { type: 'room_create_response'; room_id: string }
  | { type: 'room_delete'; room_id: string }
  | { type: 'room_delete_response'; room_id: string }
  | { type: 'room_add_actor'; room_id: string; device_id: string }
  | { type: 'room_add_actor_response'; room_id: string; device_id: string }
  | { type: 'room_remove_actor'; room_id: string; device_id: string }
  | { type: 'room_remove_actor_response'; room_id: string; device_id: string }
  | { type: 'room_grant_access'; room_id: string; user_id: string }
  | { type: 'room_grant_access_response'; room_id: string; user_id: string }
  | { type: 'room_revoke_access'; room_id: string; user_id: string }
  | { type: 'room_revoke_access_response'; room_id: string; user_id: string }
  | { type: 'room_list' }
  | { type: 'room_list_response'; rooms: RoomInfo[] }
  | { type: 'room_get'; room_id: string }
  | { type: 'room_get_response'; room_id: string; owner_user_id: string; granted_users: string[]; actors: DeviceInfo[] }
  | { type: 'room_send'; room_id: string; to?: string; payload: unknown }
  | { type: 'room_actor_joined'; room_id: string; device_id: string }
  | { type: 'room_actor_left'; room_id: string; device_id: string }
  | { type: 'room_updated'; room: RoomInfo }

  // ── system ──
  | { type: 'system_error'; message: string };
