type LogData = Record<string, unknown>;

export type DebugInfoProvider = () => LogData;

export type LogLevel = 'error' | 'warn' | 'info' | 'debug' | 'trace';

const consoleMethods: Record<LogLevel, (...args: unknown[]) => void> = {
  error: console.error,
  warn: console.warn,
  info: console.info,
  debug: console.debug,
  trace: console.debug,
};

const prefixes: Record<LogLevel, string> = {
  error: '\x1b[31m[ERROR]\x1b[0m',
  warn: '\x1b[33m[WARN]\x1b[0m',
  info: '\x1b[36m[INFO]\x1b[0m',
  debug: '\x1b[35m[DEBUG]\x1b[0m',
  trace: '\x1b[90m[TRACE]\x1b[0m',
};

export class Logger {
  constructor(
    private producer: string,
    private debugInfo?: DebugInfoProvider,
  ) {}

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
    const merged = this.debugInfo ? { ...this.debugInfo(), ...data } : data;
    consoleMethods[level](prefixes[level], this.producer, merged);
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
