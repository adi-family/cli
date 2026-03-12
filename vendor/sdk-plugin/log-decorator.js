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
export function trace(msg) {
    return function (_target, propertyKey, descriptor) {
        const original = descriptor.value;
        if (!original)
            return;
        descriptor.value = function (...args) {
            this.log.trace({ msg, method: propertyKey });
            return original.apply(this, args);
        };
    };
}
