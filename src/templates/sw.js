// Service Worker for ESP32-S3 Dashboard PWA
const CACHE_NAME = 'esp32-dashboard-v2';
const STATIC_CACHE = 'esp32-static-v2';
const DYNAMIC_CACHE = 'esp32-dynamic-v2';

// Files to cache immediately
const STATIC_ASSETS = [
    '/',
    '/logs',
    '/manifest.json',
    // Intentionally exclude '/dashboard' to avoid serving stale HTML
];

// Install event - cache static assets
self.addEventListener('install', (event) => {
    console.log('[SW] Installing service worker...');
    event.waitUntil(
        caches.open(STATIC_CACHE).then((cache) => {
            console.log('[SW] Caching static assets');
            return cache.addAll(STATIC_ASSETS);
        })
    );
    self.skipWaiting();
});

// Activate event - clean up old caches
self.addEventListener('activate', (event) => {
    console.log('[SW] Activating service worker...');
    event.waitUntil(
        caches.keys().then((cacheNames) => {
            return Promise.all(
                cacheNames
                    .filter((name) => name !== STATIC_CACHE && name !== DYNAMIC_CACHE)
                    .map((name) => {
                        console.log('[SW] Deleting old cache:', name);
                        return caches.delete(name);
                    })
            );
        })
    );
    self.clients.claim();
});

// Fetch event - serve from cache, fallback to network
self.addEventListener('fetch', (event) => {
    const { request } = event;
    const url = new URL(request.url);

    // Skip caching for SSE endpoints
    if (url.pathname === '/api/events') {
        return;
    }

    // Skip caching for API endpoints that need fresh data
    if (url.pathname.startsWith('/api/') && 
        !url.pathname.startsWith('/api/logs/recent')) {
        event.respondWith(
            fetch(request).catch(() => {
                // Return a offline response for API calls
                return new Response(
                    JSON.stringify({ error: 'offline' }),
                    {
                        status: 503,
                        headers: { 'Content-Type': 'application/json' }
                    }
                );
            })
        );
        return;
    }

    // Network-first for HTML documents to avoid stale dashboard
    if (request.destination === 'document') {
        event.respondWith(
            fetch(request, { cache: 'no-store' }).then((networkResponse) => {
                if (networkResponse && networkResponse.status === 200) {
                    const responseToCache = networkResponse.clone();
                    caches.open(DYNAMIC_CACHE).then((cache) => {
                        cache.put(request, responseToCache);
                    });
                }
                return networkResponse;
            }).catch(() => {
                return caches.match(request).then((cached) => {
                    if (cached) return cached;
                    // Ultimate fallback - offline page
                    return new Response(
                            `<!DOCTYPE html>
                            <html>
                            <head>
                                <title>ESP32 Dashboard - Offline</title>
                                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                                <style>
                                    body {
                                        background: #0a0a0a;
                                        color: #f9fafb;
                                        font-family: system-ui, sans-serif;
                                        display: flex;
                                        align-items: center;
                                        justify-content: center;
                                        height: 100vh;
                                        margin: 0;
                                        text-align: center;
                                    }
                                    .container {
                                        padding: 2rem;
                                    }
                                    h1 {
                                        color: #ef4444;
                                        margin-bottom: 1rem;
                                    }
                                    p {
                                        color: #9ca3af;
                                        margin-bottom: 2rem;
                                    }
                                    button {
                                        background: #3b82f6;
                                        color: white;
                                        border: none;
                                        padding: 0.75rem 1.5rem;
                                        border-radius: 8px;
                                        font-size: 1rem;
                                        cursor: pointer;
                                    }
                                    button:hover {
                                        background: #2563eb;
                                    }
                                </style>
                            </head>
                            <body>
                                <div class="container">
                                    <h1>📵 Offline</h1>
                                    <p>Unable to connect to ESP32 device</p>
                                    <button onclick="location.reload()">Retry</button>
                                </div>
                            </body>
                            </html>`,
                            {
                                status: 200,
                                headers: { 'Content-Type': 'text/html' }
                            }
                        );
                });
            })
        );
        return;
    }

    // Cache-first strategy for non-document static assets
    event.respondWith(
        caches.match(request).then((cachedResponse) => {
            if (cachedResponse) {
                // Update cache in background
                fetch(request).then((networkResponse) => {
                    if (networkResponse && networkResponse.status === 200) {
                        caches.open(DYNAMIC_CACHE).then((cache) => {
                            cache.put(request, networkResponse.clone());
                        });
                    }
                }).catch(() => {});
                return cachedResponse;
            }
            return fetch(request).then((networkResponse) => {
                if (networkResponse && networkResponse.status === 200) {
                    caches.open(DYNAMIC_CACHE).then((cache) => {
                        cache.put(request, networkResponse.clone());
                    });
                }
                return networkResponse;
            }).catch(() => new Response('Network error', { status: 408 }));
        })
    );
});

// Background sync for deferred actions
self.addEventListener('sync', (event) => {
    console.log('[SW] Background sync:', event.tag);
    
    if (event.tag === 'sync-metrics') {
        event.waitUntil(
            // Attempt to sync any cached metrics or logs
            fetch('/api/metrics')
                .then(response => response.json())
                .then(data => {
                    // Cache latest metrics
                    return caches.open(DYNAMIC_CACHE).then(cache => {
                        return cache.put(
                            new Request('/api/metrics'),
                            new Response(JSON.stringify(data), {
                                headers: { 'Content-Type': 'application/json' }
                            })
                        );
                    });
                })
                .catch(err => console.error('[SW] Sync failed:', err))
        );
    }
});

// Push notifications (for future alerts)
self.addEventListener('push', (event) => {
    const options = {
        body: event.data ? event.data.text() : 'ESP32 Alert',
        icon: '/icon-192.png',
        badge: '/icon-192.png',
        vibrate: [100, 50, 100],
        data: {
            dateOfArrival: Date.now(),
            primaryKey: 1
        }
    };

    event.waitUntil(
        self.registration.showNotification('ESP32 Dashboard', options)
    );
});

// Notification click handler
self.addEventListener('notificationclick', (event) => {
    console.log('[SW] Notification clicked');
    event.notification.close();

    event.waitUntil(
        clients.openWindow('/dashboard')
    );
});