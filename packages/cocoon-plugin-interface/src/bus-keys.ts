import type { Connection, DeviceInfo } from '@adi/signaling-web-plugin/bus';

export enum CocoonBusKey {
  ConnectionAdded = 'adi.cocoon:connection-added',
  ConnectionRemoved = 'adi.cocoon:connection-removed',
}

export interface CocoonConnectionAddedEvent {
  id: string;
  connection: Connection;
}

export interface CocoonConnectionRemovedEvent {
  id: string;
}

export type { DeviceInfo };
