/**
 * DocSigner Service Worker
 * Provides offline support and caching for the signing page
 */

const CACHE_NAME = 'docsign-v1';
const STATIC_ASSETS = [
    '/sign.html',
    '/sign.js',
    '/pkg/docsign_wasm.js',
    '/pkg/docsign_wasm_bg.wasm',
];

// External assets to cache
const EXTERNAL_ASSETS = [
    'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.min.js',
    'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js',
    'https://fonts.googleapis.com/css2?family=Allura&family=Dancing+Script&family=Great+Vibes&family=Pacifico&family=Sacramento&display=swap',
];

/**
 * Install event - cache static assets
 */
self.addEventListener('install', (event) => {
    console.log('[SW] Installing service worker...');

    event.waitUntil(
        caches.open(CACHE_NAME).then((cache) => {
            console.log('[SW] Caching static assets');
            // Cache static assets with error handling
            return Promise.allSettled([
                ...STATIC_ASSETS.map(url =>
                    cache.add(url).catch(err => console.warn(`[SW] Failed to cache ${url}:`, err))
                ),
                ...EXTERNAL_ASSETS.map(url =>
                    cache.add(url).catch(err => console.warn(`[SW] Failed to cache ${url}:`, err))
                )
            ]);
        }).then(() => {
            console.log('[SW] Install complete');
            return self.skipWaiting();
        })
    );
});

/**
 * Activate event - clean up old caches
 */
self.addEventListener('activate', (event) => {
    console.log('[SW] Activating service worker...');

    event.waitUntil(
        caches.keys().then((cacheNames) => {
            return Promise.all(
                cacheNames
                    .filter(name => name !== CACHE_NAME)
                    .map(name => {
                        console.log(`[SW] Deleting old cache: ${name}`);
                        return caches.delete(name);
                    })
            );
        }).then(() => {
            console.log('[SW] Activation complete');
            return self.clients.claim();
        })
    );
});

/**
 * Fetch event - serve from cache, fallback to network
 */
self.addEventListener('fetch', (event) => {
    const { request } = event;
    const url = new URL(request.url);

    // Skip non-GET requests
    if (request.method !== 'GET') {
        return;
    }

    // Skip cross-origin API requests (let them fail naturally when offline)
    if (!url.origin.includes('localhost') &&
        !url.origin.includes('cdnjs.cloudflare.com') &&
        !url.origin.includes('fonts.googleapis.com') &&
        !url.origin.includes('fonts.gstatic.com')) {
        return;
    }

    event.respondWith(
        caches.match(request).then((cachedResponse) => {
            // Return cached response if available
            if (cachedResponse) {
                console.log(`[SW] Serving from cache: ${url.pathname}`);
                return cachedResponse;
            }

            // Fetch from network
            return fetch(request).then((networkResponse) => {
                // Don't cache non-successful responses
                if (!networkResponse || networkResponse.status !== 200) {
                    return networkResponse;
                }

                // Clone the response for caching
                const responseToCache = networkResponse.clone();

                // Cache successful responses
                caches.open(CACHE_NAME).then((cache) => {
                    cache.put(request, responseToCache);
                    console.log(`[SW] Cached: ${url.pathname}`);
                });

                return networkResponse;
            }).catch((err) => {
                console.error(`[SW] Fetch failed for ${url.pathname}:`, err);

                // Return offline fallback for HTML pages
                if (request.headers.get('accept')?.includes('text/html')) {
                    return caches.match('/sign.html');
                }

                throw err;
            });
        })
    );
});

/**
 * Message event - handle sync requests
 */
self.addEventListener('message', (event) => {
    if (event.data.type === 'SYNC_QUEUE') {
        console.log('[SW] Syncing offline queue...');
        syncOfflineQueue();
    }
});

/**
 * Sync offline queue when back online
 */
async function syncOfflineQueue() {
    // Get queue from clients
    const clients = await self.clients.matchAll();

    for (const client of clients) {
        client.postMessage({ type: 'REQUEST_QUEUE' });
    }
}

/**
 * Background sync (if supported)
 */
self.addEventListener('sync', (event) => {
    if (event.tag === 'sync-signatures') {
        console.log('[SW] Background sync triggered');
        event.waitUntil(syncOfflineQueue());
    }
});
