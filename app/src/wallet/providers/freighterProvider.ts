import { isConnected, requestAccess, getAddress, getNetworkDetails } from '@stellar/freighter-api';
import { IWalletProvider, WalletType } from '../types';
import { isValidStellarAddress } from '../validation';

export class FreighterWalletProvider implements IWalletProvider {
  public readonly type: WalletType = 'freighter';
  public readonly name = 'Freighter Wallet';

  public async isAvailable(): Promise<boolean> {
    try {
      const res = await isConnected();
      return Boolean(res && res.isConnected);
    } catch {
      return false;
    }
  }

  public async connect(): Promise<string> {
    const available = await this.isAvailable();
    if (!available) {
      throw new Error(
        'Freighter extension not detected. If on mobile, please use the Demo Wallet or paste your address. On desktop, install Freighter from freighter.app.',
      );
    }

    // 1. Request user approval
    const accessRes = await requestAccess();
    if (accessRes.error) {
      throw new Error(`Freighter access request denied: ${accessRes.error}`);
    }

    const addrRes = accessRes.address ? accessRes : await getAddress();
    if (addrRes.error || !addrRes.address) {
      throw new Error(`Failed to retrieve account address from Freighter: ${addrRes.error}`);
    }

    const addr = addrRes.address.trim();
    if (!isValidStellarAddress(addr)) {
      throw new Error(`Freighter returned an invalid Stellar public key: ${addr}`);
    }

    // 2. Strict Testnet network validation
    const netDetails = await getNetworkDetails();
    if (netDetails.error) {
      throw new Error(`Failed to verify Freighter network: ${netDetails.error}`);
    }

    const netName = (netDetails.network || '').toUpperCase();
    const passphrase = netDetails.networkPassphrase || '';

    const isTestnet =
      netName === 'TESTNET' ||
      passphrase.includes('Test SDF Network') ||
      (netDetails.networkUrl || '').includes('testnet');

    if (!isTestnet) {
      throw new Error(
        `Freighter is connected to ${netName || 'Mainnet'}. EchoGrid operates exclusively on Stellar Testnet. Please switch your Freighter network to Testnet.`,
      );
    }

    return addr;
  }

  public async disconnect(): Promise<void> {
    // Freighter does not expose programmatic logout; app discards state
  }

  public async getAddress(): Promise<string | null> {
    try {
      const res = await getAddress();
      return res.address && isValidStellarAddress(res.address) ? res.address : null;
    } catch {
      return null;
    }
  }
}
