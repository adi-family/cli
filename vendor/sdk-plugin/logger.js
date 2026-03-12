const consoleMethods = {
    error: console.error,
    warn: console.warn,
    info: console.info,
    debug: console.debug,
    trace: console.debug,
};
const prefixes = {
    error: '\x1b[31m[ERROR]\x1b[0m',
    warn: '\x1b[33m[WARN]\x1b[0m',
    info: '\x1b[36m[INFO]\x1b[0m',
    debug: '\x1b[35m[DEBUG]\x1b[0m',
    trace: '\x1b[90m[TRACE]\x1b[0m',
};
export class Logger {
    producer;
    debugInfo;
    constructor(producer, debugInfo) {
        this.producer = producer;
        this.debugInfo = debugInfo;
    }
    error(data) {
        this.log('error', data);
    }
    warn(data) {
        this.log('warn', data);
    }
    info(data) {
        this.log('info', data);
    }
    debug(data) {
        this.log('debug', data);
    }
    trace(data) {
        this.log('trace', data);
    }
    log(level, data) {
        const merged = this.debugInfo ? { ...this.debugInfo(), ...data } : data;
        consoleMethods[level](prefixes[level], this.producer, merged);
    }
}
