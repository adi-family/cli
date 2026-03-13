/**
 * Auto-generated ADI service client from TypeSpec.
 * DO NOT EDIT.
 */
import type { Connection } from '@adi-family/cocoon-plugin-interface';

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
