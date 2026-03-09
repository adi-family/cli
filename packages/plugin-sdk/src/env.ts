export const env = (key: string): string[] =>
  ((import.meta.env as Record<string, string | undefined>)[`VITE_${key}`] ?? '').split(',').filter(Boolean);
