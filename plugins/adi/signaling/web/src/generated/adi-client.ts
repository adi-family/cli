/**
 * Auto-generated ADI service client from TypeSpec.
 * DO NOT EDIT.
 */
import type { Connection } from '@adi-family/cocoon-plugin-interface';

const SVC_AUTH = 'auth';

export const authAuthenticate = (c: Connection, access_token: string) =>
  c.request<unknown>(SVC_AUTH, 'authenticate', { access_token });

const SVC_DEVICE = 'device';

export const deviceRegister = (c: Connection, params: { secret: string; version: string; device_id?: string; tags?: Record<string, string>; device_type?: string; device_config?: unknown; }) =>
  c.request<unknown>(SVC_DEVICE, 'register', params);

export const deviceDeregister = (c: Connection, params: { device_id: string; reason?: string; }) =>
  c.request<unknown>(SVC_DEVICE, 'deregister', params);

export const deviceUpdateTags = (c: Connection, tags: Record<string, string>) =>
  c.request<unknown>(SVC_DEVICE, 'update_tags', { tags });

export const deviceUpdateDevice = (c: Connection, params?: { tags?: Record<string, string>; device_config?: unknown; }) =>
  c.request<unknown>(SVC_DEVICE, 'update_device', params ?? {});

export const deviceQueryDevices = (c: Connection, tag_filter: Record<string, string>) =>
  c.request<unknown>(SVC_DEVICE, 'query_devices', { tag_filter });

const SVC_PAIRING = 'pairing';

export const pairingCreateCode = (c: Connection) =>
  c.request<unknown>(SVC_PAIRING, 'create_code', {});

export const pairingUseCode = (c: Connection, code: string) =>
  c.request<unknown>(SVC_PAIRING, 'use_code', { code });

const SVC_HIVE = 'hive';

export const hiveRegister = (c: Connection, params: { hive_id: string; version: string; cocoon_kinds: CocoonKind[]; hive_id_signature: string; }) =>
  c.request<unknown>(SVC_HIVE, 'register', params);

const SVC_ROOM = 'room';

export const roomCreate = (c: Connection, params?: { room_id?: string; }) =>
  c.request<unknown>(SVC_ROOM, 'create', params ?? {});

export const roomDelete_ = (c: Connection, room_id: string) =>
  c.request<unknown>(SVC_ROOM, 'delete', { room_id });

export const roomAddActor = (c: Connection, params: { room_id: string; device_id: string; }) =>
  c.request<unknown>(SVC_ROOM, 'add_actor', params);

export const roomRemoveActor = (c: Connection, params: { room_id: string; device_id: string; }) =>
  c.request<unknown>(SVC_ROOM, 'remove_actor', params);

export const roomGrantAccess = (c: Connection, params: { room_id: string; user_id: string; }) =>
  c.request<unknown>(SVC_ROOM, 'grant_access', params);

export const roomRevokeAccess = (c: Connection, params: { room_id: string; user_id: string; }) =>
  c.request<unknown>(SVC_ROOM, 'revoke_access', params);

export const roomList = (c: Connection) =>
  c.request<unknown>(SVC_ROOM, 'list', {});

export const roomGet = (c: Connection, room_id: string) =>
  c.request<RoomInfo>(SVC_ROOM, 'get', { room_id });
