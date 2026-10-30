import { IWalletProvider, WalletType } from '../types';
import { isValidStellarAddress } from '../validation';

export class PasteWalletProvider implements IWalletProvider {
  public readonly type: WalletType = 'paste';
  public readonly name = 'Read-Only Address';
  private currentAddress: string | null = null;

  constructor(initialAddress?: string | null) {
    if (initialAddress && isValidStellarAddress(initialAddress)) {
      this.currentAddress = initialAddress.trim();
    }
  }

  public isAvailable(): boolean {
    return true;
  }

  public async connect(addressInput?: unknown): Promise<string> {
    if (typeof addressInput !== 'string') {
      throw new Error('A valid Stellar address string is required.');
    }
    const clean = addressInput.trim();
    if (!isValidStellarAddress(clean)) {
      throw new Error('Invalid Stellar G... address or CRC16 checksum.');
    }
    this.currentAddress = clean;
    return this.currentAddress;
  }

  public async disconnect(): Promise<void> {
    this.currentAddress = null;
  }

  public async getAddress(): Promise<string | null> {
    return this.currentAddress;
  }
}
