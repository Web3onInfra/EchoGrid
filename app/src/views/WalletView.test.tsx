import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import * as freighterApi from '@stellar/freighter-api';
import { WalletProvider } from '../context/WalletContext';
import { WalletView } from './WalletView';

vi.mock('@stellar/freighter-api', () => ({
  isConnected: vi.fn(),
  requestAccess: vi.fn(),
  getAddress: vi.fn(),
  getNetworkDetails: vi.fn(),
}));

const validAddress = 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT';

describe('WalletView', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
  });

  it('shows paste input form initially', () => {
    vi.mocked(freighterApi.isConnected).mockResolvedValue({ isConnected: false });

    render(
      <WalletProvider>
        <WalletView />
      </WalletProvider>,
    );

    expect(screen.getByTestId('paste-address-input')).toBeInTheDocument();
    expect(screen.queryByTestId('connected-wallet-card')).not.toBeInTheDocument();
  });

  it('displays error on invalid address submission', () => {
    vi.mocked(freighterApi.isConnected).mockResolvedValue({ isConnected: false });

    render(
      <WalletProvider>
        <WalletView />
      </WalletProvider>,
    );

    const input = screen.getByTestId('paste-address-input');
    const submitBtn = screen.getByTestId('paste-submit-btn');

    fireEvent.change(input, { target: { value: 'G_INVALID_ADDRESS' } });
    fireEvent.click(submitBtn);

    expect(screen.getByTestId('paste-error-message')).toBeInTheDocument();
  });

  it('connects valid paste address and shows connected card with disconnect button', async () => {
    vi.mocked(freighterApi.isConnected).mockResolvedValue({ isConnected: false });

    render(
      <WalletProvider>
        <WalletView />
      </WalletProvider>,
    );

    const input = screen.getByTestId('paste-address-input');
    const submitBtn = screen.getByTestId('paste-submit-btn');

    fireEvent.change(input, { target: { value: validAddress } });
    fireEvent.click(submitBtn);

    await waitFor(() => {
      expect(screen.getByTestId('connected-wallet-card')).toBeInTheDocument();
    });

    expect(screen.getByTestId('connected-address')).toHaveTextContent(/GA4Z.*EHXT/);

    // Disconnect
    const disconnectBtn = screen.getByTestId('disconnect-wallet-btn');
    fireEvent.click(disconnectBtn);

    await waitFor(() => {
      expect(screen.queryByTestId('connected-wallet-card')).not.toBeInTheDocument();
    });
  });

  it('connects via Freighter when available and on testnet', async () => {
    vi.mocked(freighterApi.isConnected).mockResolvedValue({ isConnected: true });
    vi.mocked(freighterApi.requestAccess).mockResolvedValue({ address: validAddress });
    vi.mocked(freighterApi.getAddress).mockResolvedValue({ address: validAddress });
    vi.mocked(freighterApi.getNetworkDetails).mockResolvedValue({
      network: 'TESTNET',
      networkUrl: 'https://horizon-testnet.stellar.org',
      networkPassphrase: 'Test SDF Network ; September 2015',
    });

    render(
      <WalletProvider>
        <WalletView />
      </WalletProvider>,
    );

    const freighterBtn = screen.getByTestId('connect-freighter-btn');
    fireEvent.click(freighterBtn);

    await waitFor(() => {
      expect(screen.getByTestId('connected-wallet-card')).toBeInTheDocument();
    });
    expect(screen.getByText('Freighter')).toBeInTheDocument();
  });

  it('displays guidance and error when Freighter is unavailable', async () => {
    vi.mocked(freighterApi.isConnected).mockResolvedValue({ isConnected: false });

    render(
      <WalletProvider>
        <WalletView />
      </WalletProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('freighter-unavailable-notice')).toBeInTheDocument();
    });

    const freighterBtn = screen.getByTestId('connect-freighter-btn');
    fireEvent.click(freighterBtn);

    await waitFor(() => {
      expect(screen.getByTestId('freighter-error-message')).toBeInTheDocument();
    });
    expect(screen.getByTestId('freighter-error-message')).toHaveTextContent(
      /Freighter extension not detected/i,
    );
  });
});
