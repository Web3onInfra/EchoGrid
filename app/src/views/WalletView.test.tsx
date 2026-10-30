import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { WalletProvider } from '../context/WalletContext';
import { WalletView } from './WalletView';

const validAddress = 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT';

describe('WalletView', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('shows paste input form initially', () => {
    render(
      <WalletProvider>
        <WalletView />
      </WalletProvider>,
    );

    expect(screen.getByTestId('paste-address-input')).toBeInTheDocument();
    expect(screen.queryByTestId('connected-wallet-card')).not.toBeInTheDocument();
  });

  it('displays error on invalid address submission', () => {
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

  it('connects valid address and shows connected card with disconnect button', async () => {
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
});
