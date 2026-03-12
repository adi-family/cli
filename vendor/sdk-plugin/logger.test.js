// src/logger.test.ts
import { describe, it, expect } from 'bun:test';
import { Logger } from './logger.js';
describe('Logger', () => {
    it('creates logger with producer name', () => {
        const log = new Logger('my-producer');
        expect(log).toBeDefined();
    });
    it('all log levels are callable without throwing', () => {
        const log = new Logger('test');
        expect(() => log.error({ msg: 'e' })).not.toThrow();
        expect(() => log.warn({ msg: 'w' })).not.toThrow();
        expect(() => log.info({ msg: 'i' })).not.toThrow();
        expect(() => log.debug({ msg: 'd' })).not.toThrow();
        expect(() => log.trace({ msg: 't' })).not.toThrow();
    });
    it('accepts debugInfo provider without throwing', () => {
        const log = new Logger('ctx', () => ({ extra: true }));
        expect(() => log.info({ msg: 'hello' })).not.toThrow();
    });
});
