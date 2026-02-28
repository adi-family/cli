const DEFAULT_REGISTRIES = (
  (import.meta.env.VITE_DEFAULT_REGISTRY_URLS as string) ?? ''
)
  .split(',')
  .filter(Boolean);

const DEFAULT_SIGNALING_SERVERS = (
  (import.meta.env.VITE_SIGNALING_URL as string) ?? ''
)
  .split(',')
  .filter(Boolean);

export { DEFAULT_REGISTRIES, DEFAULT_SIGNALING_SERVERS };
