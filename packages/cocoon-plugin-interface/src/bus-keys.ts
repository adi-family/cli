import type { Connection, DeviceInfo } from '@adi-family/plugin-signaling';

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
