export interface RuntimeCacheRule {
  urlPattern: RegExp | string | ((options: { url: URL }) => boolean);
  handler: 'NetworkOnly' | 'StaleWhileRevalidate' | 'CacheFirst' | 'NetworkFirst';
  options?: {
    cacheName?: string;
    expiration?: {
      maxEntries?: number;
      maxAgeSeconds?: number;
    };
  };
}

export const ORACLE_ENDPOINTS_REGEX = /\/api\/v1\/(challenge|proof|claims|ready|config|health)/;
export const STATIC_ASSETS_REGEX = /\.(?:js|css|html|png|svg|ico|wasm)$/;

export const workboxRuntimeCaching: RuntimeCacheRule[] = [
  {
    // CRITICAL SECURITY RULE: Oracle API endpoints must NEVER be cached by ServiceWorker
    urlPattern: ORACLE_ENDPOINTS_REGEX,
    handler: 'NetworkOnly',
    options: {
      cacheName: 'oracle-network-only',
    },
  },
  {
    // Static assets cached with StaleWhileRevalidate
    urlPattern: STATIC_ASSETS_REGEX,
    handler: 'StaleWhileRevalidate',
    options: {
      cacheName: 'echogrid-static-assets',
    },
  },
];
