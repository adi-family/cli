declare global {
  interface ImportMeta {
    readonly env: Record<string, string | undefined>;
  }
}

export const env = (key: string): string[] =>
  (import.meta.env[`VITE_${key}`] ?? '').split(',').filter(Boolean);
