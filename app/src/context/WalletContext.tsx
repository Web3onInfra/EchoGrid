import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import { WalletType } from '../wallet/types';
import { PasteWalletProvider } from '../wallet/providers/pasteProvider';
import { FreighterWalletProvider } from '../wallet/providers/freighterProvider';
import { isValidStellarAddress } from '../wallet/validation';

export interface WalletContextType {
  type: WalletType | null;
  address: string | null;
  isConnected: boolean;
  connectPaste: (address: string) => Promise<void>;
  connectFreighter: () => Promise<void>;
  disconnect: () => Promise<void>;
}

const WalletContext = createContext<WalletContextType>({
  type: null,
  address: null,
  isConnected: false,
  connectPaste: async () => {},
  connectFreighter: async () => {},
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

  useEffect(() => {
    if (type && address) {
      localStorage.setItem(STORAGE_KEY_TYPE, type);
      localStorage.setItem(STORAGE_KEY_ADDR, address);
    } else {
      localStorage.removeItem(STORAGE_KEY_TYPE);
      localStorage.removeItem(STORAGE_KEY_ADDR);
    }
  }, [type, address]);

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
        connectPaste,
        connectFreighter,
        disconnect,
      }}
    >
      {children}
    </WalletContext.Provider>
  );
};

// eslint-disable-next-line react-refresh/only-export-components
export const useWallet = () => useContext(WalletContext);
