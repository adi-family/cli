/**
 * Auto-generated ADI service client from TypeSpec.
 * DO NOT EDIT.
 */
import type { Connection } from '@adi-family/cocoon-plugin-interface';
import type { RoomInfo } from './models.js';

const SVC = 'auth';

export const authenticate = (c: Connection, access_token: string) =>
  c.request<unknown>(SVC, 'authenticate', { access_token });

const SVC = 'device';

export const register = (c: Connection, params: { secret: string; version: string; device_id?: string; tags?: Record<string, string>; device_type?: string; device_config?: unknown; }) =>
  c.request<unknown>(SVC, 'register', params);

export const deregister = (c: Connection, params: { device_id: string; reason?: string; }) =>
  c.request<unknown>(SVC, 'deregister', params);

export const updateTags = (c: Connection, tags: Record<string, string>) =>
  c.request<unknown>(SVC, 'update_tags', { tags });

export const updateDevice = (c: Connection, params?: { tags?: Record<string, string>; device_config?: unknown; }) =>
  c.request<unknown>(SVC, 'update_device', params ?? {});

export const queryDevices = (c: Connection, tag_filter: Record<string, string>) =>
  c.request<unknown>(SVC, 'query_devices', { tag_filter });

const SVC = 'pairing';

export const createCode = (c: Connection) =>
  c.request<unknown>(SVC, 'create_code', {});

export const useCode = (c: Connection, code: string) =>
  c.request<unknown>(SVC, 'use_code', { code });

const SVC = 'hive';

export const register = (c: Connection, params: { hive_id: string; version: string; cocoon_kinds: CocoonKind[]; hive_id_signature: string; }) =>
  c.request<unknown>(SVC, 'register', params);

const SVC = 'room';

export const create = (c: Connection, params?: { room_id?: string; }) =>
  c.request<unknown>(SVC, 'create', params ?? {});

export const delete_ = (c: Connection, room_id: string) =>
  c.request<unknown>(SVC, 'delete', { room_id });

export const addActor = (c: Connection, params: { room_id: string; device_id: string; }) =>
  c.request<unknown>(SVC, 'add_actor', params);

export const removeActor = (c: Connection, params: { room_id: string; device_id: string; }) =>
  c.request<unknown>(SVC, 'remove_actor', params);

export const grantAccess = (c: Connection, params: { room_id: string; user_id: string; }) =>
  c.request<unknown>(SVC, 'grant_access', params);

export const revokeAccess = (c: Connection, params: { room_id: string; user_id: string; }) =>
  c.request<unknown>(SVC, 'revoke_access', params);

export const list = (c: Connection) =>
  c.request<unknown>(SVC, 'list', {});

export const get = (c: Connection, room_id: string) =>
  c.request<RoomInfo>(SVC, 'get', { room_id });
