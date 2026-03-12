import type { DeviceInfo } from '@adi-family/plugin-signaling';

export interface Connection {
  readonly id: string;
  plugins: string[];
  request<T>(plugin: string, method: string, params?: unknown): Promise<T>;
  stream<T>(plugin: string, method: string, params?: unknown): AsyncGenerator<T>;
  httpProxy(plugin: string, path: string, init?: RequestInit): Promise<Response>;
  httpDirect(url: string, init?: RequestInit): Promise<Response>;
  refreshPlugins(): Promise<string[]>;
  installPlugin(pluginId: string, opts?: { registry?: string; version?: string }): Promise<unknown>;
  dispose(): void;
}

export interface ConnectionSettings {
  autoinstallPlugins?: boolean;
}

export enum CocoonBusKey {
  ConnectionAdded = 'adi.cocoon:connection-added',
  ConnectionRemoved = 'adi.cocoon:connection-removed',
  SettingsChanged = 'adi.cocoon:settings-changed',
}

export interface CocoonConnectionAddedEvent {
  id: string;
  connection: Connection;
}

export interface CocoonConnectionRemovedEvent {
  id: string;
}

export interface CocoonSettingsChangedEvent {
  id: string;
  settings: ConnectionSettings;
}

export type { DeviceInfo };
