import type {
  EditorOpenPayload,
  EditorContentPayload,
  EditorSetOptionsPayload,
  EditorSetThemePayload,
} from './types.js';

declare module '@adi-family/sdk-plugin' {
  interface EventRegistry {
    'editor:open': EditorOpenPayload;
    'editor:get-content': Record<string, never>;
    'editor:set-content': EditorContentPayload;
    'editor:set-options': EditorSetOptionsPayload;
    'editor:set-theme': EditorSetThemePayload;
    'editor:changed': EditorContentPayload;
  }
}

export {};
