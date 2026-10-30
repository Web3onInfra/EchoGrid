import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
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
} from '../../scripts/smoke-test.js';

describe('Live Smoke Test Tooling', () => {
  const originalFetch = global.fetch;

  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    global.fetch = originalFetch;
  });

  describe('Oracle Health and CORS Checks', () => {
    it('passes when health responds with status ok', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        json: async () => ({ status: 'ok', service: 'echogrid-oracle' }),
      } as Response);

      await expect(checkOracleHealth('http://localhost:8080')).resolves.toBeUndefined();
    });

    it('throws when health status is not ok', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        json: async () => ({ status: 'degraded' }),
      } as Response);

      await expect(checkOracleHealth('http://localhost:8080')).rejects.toThrow(
        'Unexpected status: degraded',
      );
    });

    it('passes when CORS header matches target origin', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'access-control-allow-origin': 'https://app.echogrid.org' }),
      } as unknown as Response);

      await expect(
        checkOracleCors('http://localhost:8080', 'https://app.echogrid.org'),
      ).resolves.toBeUndefined();
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
        json: async () => ({ rpc_healthy: true }),
      } as Response);

      await expect(checkOracleReady('http://localhost:8080')).resolves.toBeUndefined();
    });

    it('throws when ready reports rpc_healthy false', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        json: async () => ({ rpc_healthy: false }),
      } as Response);

      await expect(checkOracleReady('http://localhost:8080')).rejects.toThrow('RPC unhealthy');
    });

    it('validates config testnet parameters', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        json: async () => ({ network: 'testnet', bounty_stroops: 10_000_000 }),
      } as Response);

      await expect(checkOracleConfig('http://localhost:8080')).resolves.toBeUndefined();
    });

    it('rejects invalid or non-testnet config', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
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
        json: async () => ({
          name: 'EchoGrid PWA',
          short_name: 'EchoGrid',
          icons: [{ src: '/icon.svg', sizes: 'any' }],
        }),
      } as Response);

      await expect(checkPwaManifest('http://localhost:4173')).resolves.toBeUndefined();
    });

    it('throws when webmanifest is missing icons array', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        json: async () => ({
          name: 'EchoGrid PWA',
          short_name: 'EchoGrid',
          icons: [],
        }),
      } as Response);

      await expect(checkPwaManifest('http://localhost:4173')).rejects.toThrow(
        'Manifest missing icons array',
      );
    });

    it('verifies all required icon sizes return HTTP 200', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
      } as Response);

      await expect(checkPwaIcons('http://localhost:4173')).resolves.toBeUndefined();
      expect(global.fetch).toHaveBeenCalledTimes(3);
    });

    it('verifies service worker returns HTTP 200', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
      } as Response);

      await expect(checkPwaServiceWorker('http://localhost:4173')).resolves.toBeUndefined();
    });

    it('verifies PWA security headers when present', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({
          'permissions-policy': 'microphone=(self), screen-wake-lock=(self)',
          'content-security-policy': "default-src 'self'; connect-src 'self'",
        }),
      } as unknown as Response);

      await expect(checkPwaHeaders('https://app.echogrid.org')).resolves.toBeUndefined();
    });
  });

  describe('runSmokeTests Coordinator', () => {
    it('runs orchestrator with skip flags', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => '<div id="root">EchoGrid</div>',
        json: async () => ({
          name: 'EchoGrid',
          short_name: 'EchoGrid',
          icons: [{ src: '/icon.svg' }],
        }),
        headers: new Headers(),
      } as unknown as Response);

      const result = await runSmokeTests({
        targetOrigin: 'http://localhost:4173',
        skipOracle: true,
      });

      expect(result.success).toBe(true);
      expect(result.passed).toBe(5);
    });
  });
});
