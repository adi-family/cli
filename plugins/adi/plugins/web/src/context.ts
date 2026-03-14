import type { EventBus } from '@adi-family/sdk-plugin';

let _bus: EventBus | undefined;

export const setBus = (bus: EventBus): void => { _bus = bus; };

export const getBus = (): EventBus => {
  if (!_bus) throw new Error('Plugins plugin: bus not initialized');
  return _bus;
};
