import { describe, it, expect, beforeAll, afterAll, afterEach } from 'vitest';
import { setupServer } from 'msw/node';
import { http, HttpResponse } from 'msw';
import { OracleClient } from './oracleClient';
import { handlers, mockConfig } from './fixtures';
import { OracleError } from './errors';

const server = setupServer(...handlers);

describe('OracleClient', () => {
  const client = new OracleClient({ baseUrl: 'http://localhost:8080' });

  beforeAll(() => server.listen());
  afterEach(() => server.resetHandlers());
  afterAll(() => server.close());

  it('fetches /config successfully', async () => {
    const config = await client.getConfig();
    expect(config.network).toBe('testnet');
    expect(config.bounty_stroops).toBe(mockConfig.bounty_stroops);
  });

  it('fetches /ready successfully', async () => {
    const ready = await client.getReady();
    expect(ready.rpc_healthy).toBe(true);
    expect(ready.contract_paused).toBe(false);
  });

  it('fetches challenge for valid wallet', async () => {
    const challenge = await client.getChallenge(
      'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT',
    );
    expect(challenge.challenge_id).toBeDefined();
    expect(challenge.wallet_address).toBe(
      'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT',
    );
  });

  it('throws mapped OracleError on API error status', async () => {
    server.use(
      http.get('*/api/v1/challenge', () => {
        return HttpResponse.json(
          { code: 'CHALLENGE_RATE_LIMIT_EXCEEDED', message: 'Too many requests', status: 429 },
          { status: 429 },
        );
      }),
    );

    await expect(client.getChallenge('GABC')).rejects.toThrow(OracleError);
    try {
      await client.getChallenge('GABC');
    } catch (err: unknown) {
      const oracleErr = err as OracleError;
      expect(oracleErr.code).toBe('CHALLENGE_RATE_LIMIT_EXCEEDED');
      expect(oracleErr.userMessage).toContain('Too many challenge requests');
    }
  });

  it('handles request timeouts gracefully', async () => {
    const shortTimeoutClient = new OracleClient({
      baseUrl: 'http://localhost:8080',
      defaultTimeoutMs: 20,
    });

    server.use(
      http.get('*/api/v1/health', async () => {
        await new Promise((resolve) => setTimeout(resolve, 100));
        return HttpResponse.json({ status: 'ok' });
      }),
    );

    await expect(shortTimeoutClient.getHealth()).rejects.toThrow(OracleError);
    try {
      await shortTimeoutClient.getHealth();
    } catch (err: unknown) {
      const oracleErr = err as OracleError;
      expect(oracleErr.code).toBe('NETWORK_TIMEOUT');
    }
  });
});
