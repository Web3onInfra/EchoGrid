import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as freighterApi from '@stellar/freighter-api';
import { FreighterWalletProvider } from './freighterProvider';

vi.mock('@stellar/freighter-api', () => ({
  isConnected: vi.fn(),
  requestAccess: vi.fn(),
  getAddress: vi.fn(),
  getNetworkDetails: vi.fn(),
}));

const validAddress = 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT';

describe('FreighterWalletProvider', () => {
  let provider: FreighterWalletProvider;

  beforeEach(() => {
    vi.spyOn(freighterApi, 'isConnected').mockResolvedValue({ isConnected: true });
    vi.spyOn(freighterApi, 'requestAccess').mockResolvedValue({ address: validAddress });
    vi.spyOn(freighterApi, 'getAddress').mockResolvedValue({ address: validAddress });
    vi.spyOn(freighterApi, 'getNetworkDetails').mockResolvedValue({
      network: 'TESTNET',
      networkUrl: 'https://horizon-testnet.stellar.org',
      networkPassphrase: 'Test SDF Network ; September 2015',
    });
    provider = new FreighterWalletProvider();
  });

  it('detects when extension is installed', async () => {
    vi.spyOn(freighterApi, 'isConnected').mockResolvedValue({ isConnected: true });
    expect(await provider.isAvailable()).toBe(true);
  });

  it('detects when extension is unavailable', async () => {
    vi.spyOn(freighterApi, 'isConnected').mockResolvedValue({ isConnected: false });
    expect(await provider.isAvailable()).toBe(false);
  });

  it('connects successfully on testnet', async () => {
    const addr = await provider.connect();
    expect(addr).toBe(validAddress);
  });

  it('throws descriptive error with mobile guidance when not installed', async () => {
    vi.spyOn(freighterApi, 'isConnected').mockResolvedValue({ isConnected: false });

    await expect(provider.connect()).rejects.toThrow(/Freighter extension not detected.*mobile/i);
  });

  it('rejects connection if Freighter is set to Mainnet', async () => {
    vi.spyOn(freighterApi, 'getNetworkDetails').mockResolvedValue({
      network: 'PUBLIC',
      networkUrl: 'https://horizon.stellar.org',
      networkPassphrase: 'Public Global Stellar Network ; September 2015',
    });

    await expect(provider.connect()).rejects.toThrow(/exclusively on Stellar Testnet/i);
  });
});
