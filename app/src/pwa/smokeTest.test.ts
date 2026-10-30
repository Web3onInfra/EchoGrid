import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  normalizeUrl,
  checkOracleHealth,
  checkOracleCors,
  checkOracleReady,
  checkOracleConfig,
  checkOracleChallenge,
  checkPwaIndex,
  checkPwaManifest,
  checkPwaIcons,
  checkPwaServiceWorker,
  checkPwaHeaders,
  runSmokeTests,
  parseCliArgs,
} from '../../scripts/smoke-test.js';

describe('Live Smoke Test Tooling', () => {
  const originalFetch = global.fetch;

  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    global.fetch = originalFetch;
  });

  describe('normalizeUrl helper', () => {
    it('prepends default protocol if missing and strips trailing slashes', () => {
      expect(normalizeUrl('localhost:5173')).toBe('http://localhost:5173');
      expect(normalizeUrl('https://echogrid-app.pages.dev/')).toBe(
        'https://echogrid-app.pages.dev',
      );
      expect(normalizeUrl('')).toBe('');
    });
  });

  describe('parseCliArgs helper', () => {
    it('parses flags and positional arguments correctly', () => {
      const args1 = parseCliArgs(['http://localhost:4173', '--skip-oracle']);
      expect(args1.skipOracle).toBe(true);
      expect(args1.skipPwa).toBe(false);
      expect(args1.targetOrigin).toBe('http://localhost:4173');

      const args2 = parseCliArgs(['--skip-pwa', 'http://localhost:8080']);
      expect(args2.skipPwa).toBe(true);
      expect(args2.oracleUrl).toBe('http://localhost:8080');

      const args3 = parseCliArgs(['--help']);
      expect(args3.showHelp).toBe(true);
    });
  });

  describe('Oracle Health and CORS Checks', () => {
    it('passes when health responds with status ok', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'application/json' }),
        json: async () => ({ status: 'ok', service: 'echogrid-oracle' }),
      } as Response);

      await expect(checkOracleHealth('http://localhost:8080')).resolves.toBeUndefined();
    });

    it('throws when health status is not ok', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'application/json' }),
        json: async () => ({ status: 'degraded' }),
      } as Response);

      await expect(checkOracleHealth('http://localhost:8080')).rejects.toThrow(
        'Unexpected status: degraded',
      );
    });

    it('passes when CORS header matches target origin for GET and OPTIONS', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'access-control-allow-origin': 'https://app.echogrid.org' }),
      } as unknown as Response);

      await expect(
        checkOracleCors('http://localhost:8080', 'https://app.echogrid.org'),
      ).resolves.toBeUndefined();
      expect(global.fetch).toHaveBeenCalledTimes(2); // Simple GET + Preflight OPTIONS
    });

    it('passes when CORS header is wildcard (*)', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'access-control-allow-origin': '*' }),
      } as unknown as Response);

      await expect(
        checkOracleCors('http://localhost:8080', 'http://localhost:5173'),
      ).resolves.toBeUndefined();
    });

    it('throws when CORS header is missing or disallowed', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers(),
      } as unknown as Response);

      await expect(checkOracleCors('http://localhost:8080', 'https://evil.com')).rejects.toThrow(
        'Oracle rejected Origin "https://evil.com"',
      );
    });
  });

  describe('Oracle Config and Challenge Checks', () => {
    it('passes when ready check reports healthy RPC', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'application/json' }),
        json: async () => ({ rpc_healthy: true }),
      } as Response);

      await expect(checkOracleReady('http://localhost:8080')).resolves.toBeUndefined();
    });

    it('throws when ready reports rpc_healthy false', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'application/json' }),
        json: async () => ({ rpc_healthy: false }),
      } as Response);

      await expect(checkOracleReady('http://localhost:8080')).rejects.toThrow('RPC unhealthy');
    });

    it('validates config testnet parameters', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'application/json' }),
        json: async () => ({ network: 'testnet', bounty_stroops: 10_000_000 }),
      } as Response);

      await expect(checkOracleConfig('http://localhost:8080')).resolves.toBeUndefined();
    });

    it('rejects invalid or non-testnet config', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'application/json' }),
        json: async () => ({ network: 'mainnet', bounty_stroops: 10_000_000 }),
      } as Response);

      await expect(checkOracleConfig('http://localhost:8080')).rejects.toThrow(
        'Expected testnet, got: mainnet',
      );
    });

    it('validates challenge issuance with nonce and timestamp', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'application/json' }),
        json: async () => ({ challenge_id: 'ch_123', expires_at: 1730000000 }),
      } as Response);

      await expect(checkOracleChallenge('http://localhost:8080')).resolves.toBeUndefined();
    });
  });

  describe('PWA Asset and Header Checks', () => {
    it('validates PWA index root containing root div', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'text/html' }),
        text: async () => '<html><body><div id="root"></div><title>EchoGrid</title></body></html>',
      } as Response);

      await expect(checkPwaIndex('http://localhost:4173')).resolves.toBeUndefined();
    });

    it('throws when index returns 404', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 404,
      } as Response);

      await expect(checkPwaIndex('http://localhost:4173')).rejects.toThrow('HTTP 404');
    });

    it('validates webmanifest structure', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'application/manifest+json' }),
        json: async () => ({
          name: 'EchoGrid PWA',
          short_name: 'EchoGrid',
          start_url: '/',
          display: 'standalone',
          icons: [{ src: '/icon.svg', sizes: 'any' }],
        }),
      } as Response);

      await expect(checkPwaManifest('http://localhost:4173')).resolves.toBeUndefined();
    });

    it('detects SPA HTML rewrite fallback on manifest fetch', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'text/html; charset=utf-8' }),
      } as Response);

      await expect(checkPwaManifest('http://localhost:4173')).rejects.toThrow(
        'Manifest returned HTML',
      );
    });

    it('verifies all required icon sizes return HTTP 200 and image mime type', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'image/png' }),
      } as Response);

      await expect(checkPwaIcons('http://localhost:4173')).resolves.toBeUndefined();
      expect(global.fetch).toHaveBeenCalledTimes(4); // icon.svg + 192 + 512 + 512-maskable
    });

    it('detects SPA rewrite fallback on missing icon', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'text/html; charset=utf-8' }),
      } as Response);

      await expect(checkPwaIcons('http://localhost:4173')).rejects.toThrow(
        'returned HTML (SPA rewrite fallback)',
      );
    });

    it('verifies service worker returns HTTP 200 and valid worker code', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'application/javascript' }),
        text: async () => 'self.addEventListener("fetch", () => {}); importScripts("workbox.js");',
      } as Response);

      await expect(checkPwaServiceWorker('http://localhost:4173')).resolves.toBeUndefined();
    });

    it('detects SPA rewrite fallback on missing service worker', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'text/html' }),
      } as Response);

      await expect(checkPwaServiceWorker('http://localhost:4173')).rejects.toThrow(
        'returned HTML (SPA rewrite fallback)',
      );
    });

    it('verifies PWA security headers on deployed HTTPS target', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({
          'permissions-policy': 'microphone=(self), screen-wake-lock=(self)',
          'content-security-policy': "default-src 'self'; connect-src 'self'",
          'x-frame-options': 'DENY',
          'x-content-type-options': 'nosniff',
        }),
      } as unknown as Response);

      await expect(checkPwaHeaders('https://app.echogrid.org')).resolves.toBeUndefined();
    });

    it('fails when deployed HTTPS target is missing Permissions-Policy', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers(),
      } as unknown as Response);

      await expect(checkPwaHeaders('https://app.echogrid.org')).rejects.toThrow(
        'Deployed PWA missing Permissions-Policy',
      );
    });
  });

  describe('runSmokeTests Coordinator', () => {
    it('runs orchestrator with skip flags', async () => {
      global.fetch = vi.fn().mockImplementation(async (url: string) => {
        if (url.endsWith('/manifest.webmanifest')) {
          return {
            ok: true,
            status: 200,
            headers: new Headers({ 'content-type': 'application/manifest+json' }),
            json: async () => ({
              name: 'EchoGrid',
              short_name: 'EchoGrid',
              start_url: '/',
              display: 'standalone',
              icons: [{ src: '/icon.svg' }],
            }),
          };
        }
        if (url.endsWith('/sw.js')) {
          return {
            ok: true,
            status: 200,
            headers: new Headers({ 'content-type': 'application/javascript' }),
            text: async () =>
              'importScripts("workbox.js"); self.addEventListener("install", () => {});',
          };
        }
        if (url.includes('.png') || url.includes('.svg')) {
          return {
            ok: true,
            status: 200,
            headers: new Headers({ 'content-type': 'image/png' }),
          };
        }
        return {
          ok: true,
          status: 200,
          headers: new Headers({ 'content-type': 'text/html' }),
          text: async () => '<div id="root">EchoGrid</div>',
        };
      });

      const result = await runSmokeTests({
        targetOrigin: 'http://localhost:4173',
        skipOracle: true,
      });

      expect(result.success).toBe(true);
      expect(result.passed).toBe(5);
    });
  });
});
