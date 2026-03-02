import type { EventBus } from '@adi-family/sdk-plugin';
import type { Connection } from './types.js';

let _bus: EventBus | undefined;

export const connections = new Map<string, Connection>();

export const setBus = (bus: EventBus): void => { _bus = bus; };

export const getBus = (): EventBus => {
  if (!_bus) throw new Error('Knowledgebase plugin: bus not initialized');
  return _bus;
};
