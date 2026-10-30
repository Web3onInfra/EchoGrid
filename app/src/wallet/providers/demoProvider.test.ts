import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { DemoWalletProvider } from './demoProvider';
import { isValidStellarAddress } from '../validation';

describe('DemoWalletProvider', () => {
  let provider: DemoWalletProvider;

  beforeEach(() => {
    localStorage.clear();
    provider = new DemoWalletProvider();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('generates a valid Stellar keypair and persists keys', () => {
    const pub = provider.createKeypair();
    expect(isValidStellarAddress(pub)).toBe(true);
    expect(provider.getStoredPublicKey()).toBe(pub);

    const secret = provider.getStoredSecretKey();
    expect(secret).toBeDefined();
    expect(secret?.startsWith('S')).toBe(true);
  });

  it('forgets stored keypair on forget()', () => {
    provider.createKeypair();
    provider.forget();
    expect(provider.getStoredPublicKey()).toBeNull();
    expect(provider.getStoredSecretKey()).toBeNull();
  });

  it('funds account via Friendbot endpoint', async () => {
    const pub = provider.createKeypair();
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ successful: true }),
    });
    vi.stubGlobal('fetch', mockFetch);

    await provider.fundWithFriendbot(pub, 'https://friendbot.test.org');
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('https://friendbot.test.org?addr='),
    );
  });

  it('fetches balance from Horizon endpoint', async () => {
    const pub = provider.createKeypair();
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        balances: [{ asset_type: 'native', balance: '10000.0000000' }],
      }),
    });
    vi.stubGlobal('fetch', mockFetch);

    const info = await provider.fetchBalance(pub, 'https://horizon.test.org');
    expect(info.exists).toBe(true);
    expect(info.balance).toBe('10000.00');
  });

  it('returns 0 balance for 404 unfunded accounts', async () => {
    const pub = provider.createKeypair();
    const mockFetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
    });
    vi.stubGlobal('fetch', mockFetch);

    const info = await provider.fetchBalance(pub, 'https://horizon.test.org');
    expect(info.exists).toBe(false);
    expect(info.balance).toBe('0.00');
  });
});
