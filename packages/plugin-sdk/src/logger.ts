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

  error(data: LogData) {
    this.log('error', data);
  }

  warn(data: LogData) {
    this.log('warn', data);
  }

  info(data: LogData) {
    this.log('info', data);
  }

  debug(data: LogData) {
    this.log('debug', data);
  }

  trace(data: LogData) {
    this.log('trace', data);
  }

  private log(level: LogLevel, data: LogData) {
    consoleMethods[level](prefixes[level], this.producer, data);
  }
}

declare module './types.js' {
  interface EventRegistry {
    'logging:error': LogData;
    'logging:warn': LogData;
    'logging:info': LogData;
    'logging:debug': LogData;
    'logging:trace': LogData;
  }
}
