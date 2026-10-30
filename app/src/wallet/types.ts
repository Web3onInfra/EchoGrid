export type WalletType = 'paste' | 'freighter' | 'demo';

export interface WalletState {
  type: WalletType | null;
  address: string | null;
  isConnected: boolean;
}

export interface IWalletProvider {
  type: WalletType;
  name: string;
  isAvailable(): boolean;
  connect(params?: unknown): Promise<string>;
  disconnect(): Promise<void>;
  getAddress(): Promise<string | null>;
}
