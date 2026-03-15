export interface RegistryPlugin {
  id: string;
  name: string;
  description: string;
  latestVersion: string;
  downloads: number;
  author: string;
  tags: string[];
  pluginTypes: string[];
}

export interface CocoonInstallStatus {
  cocoonId: string;
  cocoonName: string;
  installed: boolean;
  installedVersion?: string;
  installing: boolean;
  error?: string;
}

export interface CocoonDevice {
  deviceId: string;
  signalingUrl: string;
  name?: string;
  online: boolean;
}

export type WebInstallStatus =
  | { kind: 'loading' }
  | { kind: 'available' }
  | { kind: 'installing' }
  | { kind: 'installed' }
  | { kind: 'error'; message: string };

export interface PluginItem {
  plugin: RegistryPlugin;
  webStatus: WebInstallStatus;
  cocoonStatuses: CocoonInstallStatus[];
}

export type PluginFilter = 'web' | 'installed';
export type View = 'list' | 'detail';
