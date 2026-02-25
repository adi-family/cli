export interface EditorOptions {
  language?: string;
  theme?: string;
  readOnly?: boolean;
  minimap?: boolean;
  lineNumbers?: 'on' | 'off' | 'relative';
  wordWrap?: 'on' | 'off' | 'wordWrapColumn' | 'bounded';
  fontSize?: number;
  tabSize?: number;
}

export interface EditorOpenPayload {
  content: string;
  options?: EditorOptions;
}

export interface EditorContentPayload {
  content: string;
}

export interface EditorSetOptionsPayload {
  options: EditorOptions;
}

export interface EditorSetThemePayload {
  theme: string;
}
