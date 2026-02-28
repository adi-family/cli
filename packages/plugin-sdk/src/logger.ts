import type { EventBus } from './types.js';

type LogData = Record<string, unknown>;

export type LogLevel = 'error' | 'warn' | 'info' | 'debug' | 'trace';

const consoleMethods: Record<LogLevel, (...args: unknown[]) => void> = {
  error: console.error,
  warn: console.warn,
  info: console.info,
  debug: console.debug,
  trace: console.trace,
};

const prefixes: Record<LogLevel, string> = {
  error: '[ERROR]',
  warn: '[WARN]',
  info: '[INFO]',
  debug: '[DEBUG]',
  trace: '[TRACE]',
};

export class Logger {
  constructor(private producer: string) {}

  error(bus: EventBus, data: LogData) {
    this.log('error', bus, data);
  }

  warn(bus: EventBus, data: LogData) {
    this.log('warn', bus, data);
  }

  info(bus: EventBus, data: LogData) {
    this.log('info', bus, data);
  }

  debug(bus: EventBus, data: LogData) {
    this.log('debug', bus, data);
  }

  trace(bus: EventBus, data: LogData) {
    this.log('trace', bus, data);
  }

  private log(level: LogLevel, bus: EventBus, data: LogData) {
    consoleMethods[level](prefixes[level], this.producer, data);
    bus.emit(`logging:${level}` as keyof LoggingEvents, data, this.producer);
  }
}

type LoggingEvents = {
  [K in `logging:${LogLevel}`]: LogData;
};

declare module './types.js' {
  interface EventRegistry {
    'logging:error': LogData;
    'logging:warn': LogData;
    'logging:info': LogData;
    'logging:debug': LogData;
    'logging:trace': LogData;
  }
}
