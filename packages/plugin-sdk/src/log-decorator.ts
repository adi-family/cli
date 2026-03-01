import type { Logger } from './logger.js';

/**
 * Method decorator that emits a trace log on entry.
 *
 * Requires the class to have a `log: Logger` property (private is fine — accessed at runtime).
 *
 * @example
 * class MyService {
 *   private readonly log = new Logger('my-service');
 *
 *   @trace('connecting to server')
 *   connect() { ... }
 * }
 * // On call: [TRACE] my-service { msg: 'connecting to server' }
 */
export function trace(msg: string) {
  return function (
    _target: unknown,
    propertyKey: string,
    descriptor: PropertyDescriptor,
  ): void {
    const original = descriptor.value as ((...args: unknown[]) => unknown) | undefined;
    if (!original) return;

    descriptor.value = function (this: { log: Logger }, ...args: unknown[]) {
      this.log.trace({ msg, method: propertyKey });
      return original.apply(this, args);
    };
  };
}
