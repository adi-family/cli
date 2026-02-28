// Unified service worker — app asset caching + plugin bundle caching.
const CACHE_NAME = 'adi-app-v1';
const PLUGIN_CACHE_PREFIX = 'adi-plugin-v1-';
const OFFLINE_URL = '/offline.html';

// --- Install: pre-cache offline fallback ---

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => cache.add(OFFLINE_URL))
  );
  self.skipWaiting();
});

// --- Activate: clean old caches, claim clients ---

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(
        keys
          .filter((k) => {
            if (k === CACHE_NAME) return false;
            if (k.startsWith('adi-app-') || k.startsWith('adi-plugin-')) return true;
            return false;
          })
          .filter((k) => {
            // Keep current-version plugin caches
            if (k.startsWith(PLUGIN_CACHE_PREFIX)) return false;
            return true;
          })
          .map((k) => caches.delete(k))
      )
    ).then(() => self.clients.claim())
  );
});

// --- Fetch: route by URL pattern ---

self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // Plugin bundles — stale-while-revalidate with client notify
  if (url.pathname.includes('/v1/plugins/')) {
    event.respondWith(pluginStaleWhileRevalidate(request));
    return;
  }

  // Only handle same-origin + Google Fonts from here
  const isSameOrigin = url.origin === self.location.origin;
  const isGoogleFontsCSS = url.hostname === 'fonts.googleapis.com';
  const isGoogleFontsFiles = url.hostname === 'fonts.gstatic.com';

  if (!isSameOrigin && !isGoogleFontsCSS && !isGoogleFontsFiles) return;

  // Hashed assets (/assets/*) — cache-first (immutable)
  if (isSameOrigin && url.pathname.startsWith('/assets/')) {
    event.respondWith(cacheFirst(request));
    return;
  }

  // Favicon — cache-first
  if (isSameOrigin && url.pathname === '/favicon.ico') {
    event.respondWith(cacheFirst(request));
    return;
  }

  // Google Fonts CSS — stale-while-revalidate
  if (isGoogleFontsCSS) {
    event.respondWith(staleWhileRevalidate(request));
    return;
  }

  // Google Fonts files (woff2) — cache-first (immutable)
  if (isGoogleFontsFiles) {
    event.respondWith(cacheFirst(request));
    return;
  }

  // Navigation requests (HTML) — network-first with offline fallback
  if (request.mode === 'navigate') {
    event.respondWith(networkFirstWithOffline(request));
    return;
  }

  // Everything else — network only (API calls, signaling, etc.)
});

// --- Caching strategies ---

async function cacheFirst(request) {
  const cached = await caches.match(request);
  if (cached) return cached;

  const response = await fetch(request);
  if (response.ok) {
    const cache = await caches.open(CACHE_NAME);
    cache.put(request, response.clone());
  }
  return response;
}

async function staleWhileRevalidate(request) {
  const cache = await caches.open(CACHE_NAME);
  const cached = await cache.match(request);

  const fetchPromise = fetch(request).then((response) => {
    if (response.ok) cache.put(request, response.clone());
    return response;
  }).catch(() => null);

  if (cached) {
    void fetchPromise;
    return cached;
  }

  const fresh = await fetchPromise;
  return fresh ?? Response.error();
}

async function networkFirstWithOffline(request) {
  try {
    const response = await fetch(request);
    if (response.ok) {
      const cache = await caches.open(CACHE_NAME);
      cache.put(request, response.clone());
    }
    return response;
  } catch {
    const cached = await caches.match(request);
    if (cached) return cached;
    return caches.match(OFFLINE_URL);
  }
}

// --- Plugin bundle caching (ported from packages/plugin-sdk/src/sw.ts) ---

function pluginCacheKey(url) {
  return PLUGIN_CACHE_PREFIX + url;
}

async function pluginStaleWhileRevalidate(request) {
  const cache = await caches.open(pluginCacheKey(request.url));
  const cached = await cache.match(request);

  const revalidate = pluginFetchAndCache(cache, request).catch(() => null);

  if (cached) {
    void revalidate;
    return cached;
  }

  const fresh = await revalidate;
  return fresh ?? new Response('Plugin bundle unavailable', { status: 503 });
}

async function pluginFetchAndCache(cache, request) {
  const response = await fetch(request.clone());
  if (!response.ok) return null;

  const cached = await cache.match(request);
  const isNew = !cached || (await pluginIsDifferent(cached, response.clone()));

  await cache.put(request, response.clone());

  if (isNew && cached) {
    await notifyClients(request.url);
  }

  return response;
}

async function pluginIsDifferent(a, b) {
  const etagA = a.headers.get('etag');
  const etagB = b.headers.get('etag');
  if (etagA && etagB) return etagA !== etagB;

  const [bodyA, bodyB] = await Promise.all([a.text(), b.text()]);
  return bodyA !== bodyB;
}

async function notifyClients(url) {
  const allClients = await self.clients.matchAll();
  for (const client of allClients) {
    client.postMessage({ type: 'plugin:bundle-updated', url });
  }
}
