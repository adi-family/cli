// Service worker for plugin bundle caching (stale-while-revalidate).
// Register via registerPluginSW() before calling loadPlugins().

const CACHE_VERSION = 'v1';
const CACHE_PREFIX = 'adi-plugin-';

function cacheKey(url: string): string {
  return `${CACHE_PREFIX}${CACHE_VERSION}-${url}`;
}

function isPluginBundle(url: string): boolean {
  return url.includes('/v1/plugins/');
}

self.addEventListener('activate', (event) => {
  (event as ExtendableEvent).waitUntil(
    caches.keys().then((keys) =>
      Promise.all(
        keys
          .filter((k) => k.startsWith(CACHE_PREFIX) && !k.startsWith(`${CACHE_PREFIX}${CACHE_VERSION}-`))
          .map((k) => caches.delete(k))
      )
    )
  );
});

self.addEventListener('fetch', (event: Event) => {
  const fetchEvent = event as FetchEvent;
  const url = fetchEvent.request.url;

  if (!isPluginBundle(url)) return;

  fetchEvent.respondWith(staleWhileRevalidate(fetchEvent.request));
});

async function staleWhileRevalidate(request: Request): Promise<Response> {
  const cache = await caches.open(cacheKey(request.url));
  const cached = await cache.match(request);
  const revalidate = fetchAndCache(cache, request).catch(() => null);

  if (cached) {
    void revalidate;
    return cached;
  }

  const fresh = await revalidate;
  return fresh ?? new Response('Plugin bundle unavailable', { status: 503 });
}

async function fetchAndCache(
  cache: Cache,
  request: Request
): Promise<Response | null> {
  const response = await fetch(request.clone());
  if (!response.ok) return null;

  const cached = await cache.match(request);

  // Detect if content changed (ETag first, body comparison as fallback).
  const isNew = !cached || (await isDifferent(cached, response.clone()));

  await cache.put(request, response.clone());

  if (isNew && cached) {
    void notifyClients(request.url);
  }

  return response;
}

async function isDifferent(a: Response, b: Response): Promise<boolean> {
  const etagA = a.headers.get('etag');
  const etagB = b.headers.get('etag');
  if (etagA && etagB) return etagA !== etagB;
  const [bodyA, bodyB] = await Promise.all([a.text(), b.text()]);
  return bodyA !== bodyB;
}

async function notifyClients(url: string): Promise<void> {
  const clients = await (
    self as unknown as ServiceWorkerGlobalScope
  ).clients.matchAll();
  for (const client of clients) {
    client.postMessage({ type: 'plugin:bundle-updated', url });
  }
}
