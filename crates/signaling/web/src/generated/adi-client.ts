/**
 * Auto-generated ADI service client from TypeSpec.
 * DO NOT EDIT.
 */
import type { Connection } from '@adi-family/cocoon-plugin-interface';
import type { CocoonKind } from './models';

const SVC_AUTH = 'auth';

export const authenticate = (c: Connection, access_token: string) =>
  c.request<unknown>(SVC_AUTH, 'authenticate', { access_token });

const SVC_DEVICE = 'device';

export const registerDevice = (c: Connection, params: { secret: string; version: string; device_id?: string; tags?: Record<string, string>; device_type?: string; device_config?: unknown; }) =>
  c.request<unknown>(SVC_DEVICE, 'register', params);

export const deregister = (c: Connection, params: { device_id: string; reason?: string; }) =>
  c.request<unknown>(SVC_DEVICE, 'deregister', params);

export const updateTags = (c: Connection, tags: Record<string, string>) =>
  c.request<unknown>(SVC_DEVICE, 'update_tags', { tags });

export const updateDevice = (c: Connection, params?: { tags?: Record<string, string>; device_config?: unknown; }) =>
  c.request<unknown>(SVC_DEVICE, 'update_device', params ?? {});

export const queryDevices = (c: Connection, tag_filter: Record<string, string>) =>
  c.request<unknown>(SVC_DEVICE, 'query_devices', { tag_filter });

const SVC_PAIRING = 'pairing';

export const createCode = (c: Connection) =>
  c.request<unknown>(SVC_PAIRING, 'create_code', {});

export const useCode = (c: Connection, code: string) =>
  c.request<unknown>(SVC_PAIRING, 'use_code', { code });

const SVC_HIVE = 'hive';

export const registerHive = (c: Connection, params: { hive_id: string; version: string; cocoon_kinds: CocoonKind[]; hive_id_signature: string; }) =>
  c.request<unknown>(SVC_HIVE, 'register', params);
