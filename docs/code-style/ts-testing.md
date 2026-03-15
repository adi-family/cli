# TypeScript Testing

## Runner

Use **bun test** (`bun:test`). No vitest, jest, or mocha.

```bash
bun test                          # run all tests in cwd
bun test --watch                  # watch mode
bun run test                      # via package.json script
bun run --filter '*' test         # workspace-wide from root
```

## File Convention

- Co-locate tests: `foo.ts` → `foo.test.ts` in the same directory
- Exclude from build via `tsconfig.build.json` (not `tsconfig.json`)
- Exclude from npm publish via `files` in `package.json`

## Imports

```ts
import { describe, it, expect, mock, spyOn, jest, beforeEach, afterEach } from 'bun:test';
```

Import only what you use. Common imports:

| Import | Purpose |
|--------|---------|
| `mock()` | Create a mock function |
| `spyOn(obj, 'method')` | Spy on existing method |
| `jest.useFakeTimers()` | Control time |
| `jest.restoreAllMocks()` | Clean up spies in `afterEach` |

## Structure

```ts
describe('ModuleName — feature group', () => {
  it('does specific thing', () => {
    // arrange
    const bus = EventBus.init();
    const handler = mock();

    // act
    bus.on('event', handler, 'test');
    bus.emit('event', { value: 42 }, 'test');

    // assert
    expect(handler).toHaveBeenCalledWith({ value: 42 });
  });
});
```

### Rules

- **One behavior per test.** Name describes the expected behavior, not the method.
- **Flat describe blocks.** Use `describe('Module — topic')` not deeply nested describes.
- **No shared mutable state.** Create fresh instances in each test. Use `beforeEach` only for reset functions like `_resetRegistry()`.
- **No test helpers in separate files** unless shared across multiple test files.

## Factory Functions

Extract object creation into local factory functions at the top of the test file:

```ts
function makeCtx(opts?: ConstructorParameters<typeof AppContext>[1]) {
  return new AppContext(EventBus.init(), opts);
}

function makePlugin(id: string, version = '1.0.0', opts: PluginOpts = {}): AdiPlugin {
  const { deps = [], requires = [], onRegister } = opts;
  class P extends AdiPlugin {
    readonly id = id;
    readonly version = version;
    readonly dependencies = deps;
    readonly requires = requires;
    onRegister = onRegister;
  }
  return new P();
}
```

Keep factory functions simple — default parameters for the common case, explicit overrides for edge cases.

## Mocking

### Mock functions

```ts
const handler = mock();
handler();
expect(handler).toHaveBeenCalledTimes(1);
```

### Spying on globals

```ts
spyOn(globalThis, 'fetch').mockImplementation(async () => new Response(''));
spyOn(URL, 'createObjectURL').mockImplementation(() => 'blob:...');
```

Always restore in `afterEach`:

```ts
afterEach(() => {
  jest.restoreAllMocks();
});
```

### Fake timers

```ts
it('drops events older than TTL', () => {
  jest.useFakeTimers();
  // ... test ...
  jest.advanceTimersByTime(31_000);
  // ... assert ...
  jest.useRealTimers();
});
```

Always call `jest.useRealTimers()` at the end to avoid leaking into other tests.

## Assertions

Prefer specific matchers:

```ts
expect(handler).toHaveBeenCalledWith({ value: 42 });    // exact payload
expect(handler).toHaveBeenCalledTimes(1);                // call count
expect(handler).not.toHaveBeenCalled();                  // never called
expect(list).toEqual(['a', 'b']);                         // deep equality
expect(list).toContain('a');                              // includes element
expect(() => fn()).toThrow('message substring');          // error message
expect(result).toBe(same);                               // reference equality
expect(obj).toEqual(expect.objectContaining({ key: 1 }));// partial match
expect(arr).toEqual(expect.arrayContaining(['a']));      // subset match
```

### Async

```ts
it('loads plugins', async () => {
  await loadPlugins(bus, [], { timeout: 1000 });
  expect(handler).toHaveBeenCalled();
});
```

## Type Augmentation in Tests

When testing typed event systems, augment the type registry in the test file:

```ts
declare module './types.js' {
  interface EventRegistry {
    'test:ping': { value: number };
  }
}
```

This keeps test-only types out of production code.

## Anti-Patterns

| Don't | Do |
|-------|-----|
| Share state between tests | Fresh instances per test |
| `setTimeout` in tests | `jest.useFakeTimers()` + `advanceTimersByTime` |
| Test implementation details | Test observable behavior |
| Giant test with many assertions | One behavior per test |
| `any` casts to silence types | Proper type setup or `as never` for intentional misuse |
| `console.log` debugging | Remove before commit |
| Snapshot tests | Explicit assertions |
