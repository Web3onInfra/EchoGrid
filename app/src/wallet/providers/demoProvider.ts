import { Keypair } from '@stellar/stellar-base';
import { IWalletProvider, WalletType } from '../types';
import { isValidStellarAddress } from '../validation';

const STORAGE_DEMO_PUB = 'echogrid_demo_pubkey';
const STORAGE_DEMO_SEC = 'echogrid_demo_secret';

export interface AccountBalanceInfo {
  exists: boolean;
  balance: string;
}

export class DemoWalletProvider implements IWalletProvider {
  public readonly type: WalletType = 'demo';
  public readonly name = 'Demo Testnet Wallet';

  public isAvailable(): boolean {
    return true;
  }

  public getStoredPublicKey(): string | null {
    const pub = localStorage.getItem(STORAGE_DEMO_PUB);
    return pub && isValidStellarAddress(pub) ? pub : null;
  }

  public getStoredSecretKey(): string | null {
    return localStorage.getItem(STORAGE_DEMO_SEC);
  }

  public async connect(): Promise<string> {
    const existing = this.getStoredPublicKey();
    if (existing) {
      return existing;
    }
    return this.createKeypair();
  }

  public createKeypair(): string {
    const pair = Keypair.random();
    const pub = pair.publicKey();
    const sec = pair.secret();
    localStorage.setItem(STORAGE_DEMO_PUB, pub);
    localStorage.setItem(STORAGE_DEMO_SEC, sec);
    return pub;
  }

  public forget(): void {
    localStorage.removeItem(STORAGE_DEMO_PUB);
    localStorage.removeItem(STORAGE_DEMO_SEC);
  }

  public async disconnect(): Promise<void> {
    // Retains keypair in storage for re-use unless explicitly forgotten
  }

  public async getAddress(): Promise<string | null> {
    return this.getStoredPublicKey();
  }

  public async fundWithFriendbot(
    publicKey: string,
    friendbotUrl = 'https://friendbot.stellar.org',
  ): Promise<void> {
    if (!isValidStellarAddress(publicKey)) {
      throw new Error('Invalid Stellar address for Friendbot funding.');
    }

    const res = await fetch(`${friendbotUrl}?addr=${encodeURIComponent(publicKey)}`);
    if (!res.ok) {
      let msg = `Friendbot request failed with status ${res.status}`;
      try {
        const body = await res.json();
        if (body.detail) msg = body.detail;
      } catch {
        // use default message
      }
      throw new Error(msg);
    }
  }

  public async fetchBalance(
    publicKey: string,
    horizonUrl = 'https://horizon-testnet.stellar.org',
  ): Promise<AccountBalanceInfo> {
    if (!isValidStellarAddress(publicKey)) {
      return { exists: false, balance: '0.00' };
    }

    try {
      const res = await fetch(`${horizonUrl}/accounts/${encodeURIComponent(publicKey)}`);
      if (res.status === 404) {
        return { exists: false, balance: '0.00' };
      }
      if (!res.ok) {
        return { exists: false, balance: '0.00' };
      }
      const data = await res.json();
      const nativeBalance = data.balances?.find(
        (b: { asset_type: string; balance: string }) => b.asset_type === 'native',
      );
      return {
        exists: true,
        balance: nativeBalance ? parseFloat(nativeBalance.balance).toFixed(2) : '0.00',
      };
    } catch {
      return { exists: false, balance: '0.00' };
    }
  }
}
