import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import { WalletType } from '../wallet/types';
import { PasteWalletProvider } from '../wallet/providers/pasteProvider';
import { FreighterWalletProvider } from '../wallet/providers/freighterProvider';
import { DemoWalletProvider } from '../wallet/providers/demoProvider';
import { isValidStellarAddress } from '../wallet/validation';

export interface WalletContextType {
  type: WalletType | null;
  address: string | null;
  isConnected: boolean;
  balance: string;
  accountExists: boolean;
  isFunding: boolean;
  connectPaste: (address: string) => Promise<void>;
  connectFreighter: () => Promise<void>;
  createDemoWallet: () => Promise<string>;
  fundDemoWallet: () => Promise<void>;
  forgetDemoWallet: () => Promise<void>;
  getDemoSecretKey: () => string | null;
  refreshBalance: () => Promise<void>;
  disconnect: () => Promise<void>;
}

const WalletContext = createContext<WalletContextType>({
  type: null,
  address: null,
  isConnected: false,
  balance: '0.00',
  accountExists: false,
  isFunding: false,
  connectPaste: async () => {},
  connectFreighter: async () => {},
  createDemoWallet: async () => '',
  fundDemoWallet: async () => {},
  forgetDemoWallet: async () => {},
  getDemoSecretKey: () => null,
  refreshBalance: async () => {},
  disconnect: async () => {},
});

const STORAGE_KEY_TYPE = 'echogrid_wallet_type';
const STORAGE_KEY_ADDR = 'echogrid_wallet_address';

export const WalletProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [type, setType] = useState<WalletType | null>(() => {
    return (localStorage.getItem(STORAGE_KEY_TYPE) as WalletType) || null;
  });
  const [address, setAddress] = useState<string | null>(() => {
    const saved = localStorage.getItem(STORAGE_KEY_ADDR);
    return saved && isValidStellarAddress(saved) ? saved : null;
  });
  const [balance, setBalance] = useState<string>('0.00');
  const [accountExists, setAccountExists] = useState<boolean>(false);
  const [isFunding, setIsFunding] = useState<boolean>(false);

  const refreshBalance = useCallback(async () => {
    if (!address) {
      setBalance('0.00');
      setAccountExists(false);
      return;
    }
    const demoProvider = new DemoWalletProvider();
    const info = await demoProvider.fetchBalance(address);
    setBalance(info.balance);
    setAccountExists(info.exists);
  }, [address]);

  useEffect(() => {
    if (type && address) {
      localStorage.setItem(STORAGE_KEY_TYPE, type);
      localStorage.setItem(STORAGE_KEY_ADDR, address);
      refreshBalance();
    } else {
      localStorage.removeItem(STORAGE_KEY_TYPE);
      localStorage.removeItem(STORAGE_KEY_ADDR);
      setBalance('0.00');
      setAccountExists(false);
    }
  }, [type, address, refreshBalance]);

  const connectPaste = useCallback(async (newAddress: string) => {
    const provider = new PasteWalletProvider();
    const validated = await provider.connect(newAddress);
    setType('paste');
    setAddress(validated);
  }, []);

  const connectFreighter = useCallback(async () => {
    const provider = new FreighterWalletProvider();
    const validated = await provider.connect();
    setType('freighter');
    setAddress(validated);
  }, []);

  const createDemoWallet = useCallback(async () => {
    const provider = new DemoWalletProvider();
    const pub = provider.createKeypair();
    setType('demo');
    setAddress(pub);
    return pub;
  }, []);

  const fundDemoWallet = useCallback(async () => {
    if (!address) throw new Error('No wallet address to fund');
    setIsFunding(true);
    try {
      const provider = new DemoWalletProvider();
      await provider.fundWithFriendbot(address);
      await refreshBalance();
    } finally {
      setIsFunding(false);
    }
  }, [address, refreshBalance]);

  const forgetDemoWallet = useCallback(async () => {
    const provider = new DemoWalletProvider();
    provider.forget();
    setType(null);
    setAddress(null);
  }, []);

  const getDemoSecretKey = useCallback(() => {
    const provider = new DemoWalletProvider();
    return provider.getStoredSecretKey();
  }, []);

  const disconnect = useCallback(async () => {
    setType(null);
    setAddress(null);
  }, []);

  return (
    <WalletContext.Provider
      value={{
        type,
        address,
        isConnected: Boolean(address && type),
        balance,
        accountExists,
        isFunding,
        connectPaste,
        connectFreighter,
        createDemoWallet,
        fundDemoWallet,
        forgetDemoWallet,
        getDemoSecretKey,
        refreshBalance,
        disconnect,
      }}
    >
      {children}
    </WalletContext.Provider>
  );
};

// eslint-disable-next-line react-refresh/only-export-components
export const useWallet = () => useContext(WalletContext);
