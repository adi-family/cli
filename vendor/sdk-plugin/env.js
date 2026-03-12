export const env = (key) => (import.meta.env[`VITE_${key}`] ?? '').split(',').filter(Boolean);
