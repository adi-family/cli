/**
 * Auto-generated ADI service client from TypeSpec.
 * DO NOT EDIT.
 */
import type { Connection } from '@adi-family/cocoon-plugin-interface';

const SVC_SILK = 'silk';

export const silkCreateSession = (c: Connection, params?: { cwd?: string; env?: Record<string, string>; shell?: string; }) =>
  c.request<unknown>(SVC_SILK, 'create_session', params ?? {});

const SVC_PLUGIN = 'plugin';

export const pluginInstallPlugin = (c: Connection, params: { request_id: string; plugin_id: string; registry?: string; version?: string; }) =>
  c.request<PluginInstallResult>(SVC_PLUGIN, 'install_plugin', params);
