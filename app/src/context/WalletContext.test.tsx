import { useState } from 'react';
import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import { WalletProvider, useWallet } from './WalletContext';

const validAddress = 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT';

const TestWalletComponent = () => {
  const { isConnected, address, type, connectPaste, disconnect } = useWallet();
  const [errorMsg, setErrorMsg] = useState('');

  return (
    <div>
      <div>Connected: {isConnected ? 'yes' : 'no'}</div>
      <div>Type: {type || 'none'}</div>
      <div>Address: {address || 'none'}</div>
      <div>Error: {errorMsg}</div>
      <button onClick={() => connectPaste(validAddress)}>Connect Valid</button>
      <button onClick={() => connectPaste('INVALID').catch((err) => setErrorMsg(err.message))}>
        Connect Invalid
      </button>
      <button onClick={() => disconnect()}>Disconnect</button>
    </div>
  );
};

describe('WalletContext', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('initializes in disconnected state', () => {
    render(
      <WalletProvider>
        <TestWalletComponent />
      </WalletProvider>,
    );

    expect(screen.getByText('Connected: no')).toBeInTheDocument();
    expect(screen.getByText('Type: none')).toBeInTheDocument();
  });

  it('connects valid paste address and persists to localStorage', async () => {
    render(
      <WalletProvider>
        <TestWalletComponent />
      </WalletProvider>,
    );

    await act(async () => {
      screen.getByText('Connect Valid').click();
    });

    expect(screen.getByText('Connected: yes')).toBeInTheDocument();
    expect(screen.getByText('Type: paste')).toBeInTheDocument();
    expect(screen.getByText(`Address: ${validAddress}`)).toBeInTheDocument();
    expect(localStorage.getItem('echogrid_wallet_type')).toBe('paste');
    expect(localStorage.getItem('echogrid_wallet_address')).toBe(validAddress);
  });

  it('rejects invalid address connection', async () => {
    render(
      <WalletProvider>
        <TestWalletComponent />
      </WalletProvider>,
    );

    await act(async () => {
      screen.getByText('Connect Invalid').click();
    });

    expect(
      screen.getByText('Error: Invalid Stellar G... address or CRC16 checksum.'),
    ).toBeInTheDocument();
    expect(screen.getByText('Connected: no')).toBeInTheDocument();
  });

  it('disconnects and clears persistence', async () => {
    render(
      <WalletProvider>
        <TestWalletComponent />
      </WalletProvider>,
    );

    await act(async () => {
      screen.getByText('Connect Valid').click();
    });
    expect(screen.getByText('Connected: yes')).toBeInTheDocument();

    await act(async () => {
      screen.getByText('Disconnect').click();
    });
    expect(screen.getByText('Connected: no')).toBeInTheDocument();
    expect(localStorage.getItem('echogrid_wallet_type')).toBeNull();
  });
});
