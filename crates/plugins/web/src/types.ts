/** A plugin entry from the registry index. */
export interface RegistryPlugin {
  id: string;
  name: string;
  description: string;
  latestVersion: string;
  author: string;
  downloads: number;
  tags: string[];
  pluginTypes: string[];
}

/** Install status on a specific cocoon. */
export interface CocoonInstallStatus {
  cocoonId: string;
  cocoonName: string;
  installed: boolean;
  installedVersion?: string;
  installing: boolean;
  error?: string;
}

/** Combined view model for a plugin with install state. */
export interface PluginItem {
  plugin: RegistryPlugin;
  webInstalled: boolean;
  webInstalling: boolean;
  cocoonStatuses: CocoonInstallStatus[];
}

export type PluginFilter = 'all' | 'web' | 'cocoon' | 'installed';
export type View = 'list' | 'detail';

export interface CocoonDevice {
  deviceId: string;
  signalingUrl: string;
  name?: string;
  online: boolean;
}
