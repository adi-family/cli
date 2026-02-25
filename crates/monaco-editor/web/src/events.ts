import type {
  EditorOpenPayload,
  EditorContentPayload,
  EditorSetOptionsPayload,
  EditorSetThemePayload,
} from './types.js';

declare module '@adi-family/sdk-plugin' {
  interface EventRegistry {
    'editor:open': EditorOpenPayload;
    'editor:open:ok': { _cid: string };

    'editor:get-content': Record<string, never>;
    'editor:get-content:ok': EditorContentPayload & { _cid: string };

    'editor:set-content': EditorContentPayload;
    'editor:set-options': EditorSetOptionsPayload;
    'editor:set-theme': EditorSetThemePayload;

    'editor:changed': EditorContentPayload;
  }
}

export {};
