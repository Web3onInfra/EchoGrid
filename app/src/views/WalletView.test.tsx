import { describe, it, expect, beforeEach, beforeAll, afterAll, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { setupServer } from 'msw/node';
import { handlers } from '../api/fixtures';
import * as freighterApi from '@stellar/freighter-api';
import { WalletProvider } from '../context/WalletContext';
import { OracleProvider } from '../context/OracleContext';
import { WalletView } from './WalletView';

const server = setupServer(...handlers);

vi.mock('@stellar/freighter-api', () => ({
  isConnected: vi.fn(),
  requestAccess: vi.fn(),
  getAddress: vi.fn(),
  getNetworkDetails: vi.fn(),
}));

const validAddress = 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT';

describe('WalletView', () => {
  beforeAll(() => server.listen());
  afterEach(() => {
    server.resetHandlers();
    vi.restoreAllMocks();
  });
  afterAll(() => server.close());

  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
  });

  it('shows paste input form initially and security warning', async () => {
    vi.mocked(freighterApi.isConnected).mockResolvedValue({ isConnected: false });

    render(
      <OracleProvider>
        <WalletProvider>
          <WalletView />
        </WalletProvider>
      </OracleProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('paste-address-input')).toBeInTheDocument();
    });
    expect(screen.getByTestId('demo-wallet-warning')).toHaveTextContent(/Testnet only/i);
    expect(screen.queryByTestId('connected-wallet-card')).not.toBeInTheDocument();
  });

  it('connects valid paste address and shows connected card with disconnect button', async () => {
    vi.mocked(freighterApi.isConnected).mockResolvedValue({ isConnected: false });

    render(
      <OracleProvider>
        <WalletProvider>
          <WalletView />
        </WalletProvider>
      </OracleProvider>,
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

  it('generates demo wallet, exports secret, and forgets it', async () => {
    vi.mocked(freighterApi.isConnected).mockResolvedValue({ isConnected: false });

    render(
      <OracleProvider>
        <WalletProvider>
          <WalletView />
        </WalletProvider>
      </OracleProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('generate-demo-wallet-btn')).toBeInTheDocument();
    });

    const generateBtn = screen.getByTestId('generate-demo-wallet-btn');
    fireEvent.click(generateBtn);

    await waitFor(() => {
      expect(screen.getByTestId('connected-wallet-card')).toBeInTheDocument();
    });
    expect(screen.getByText('Demo Keypair')).toBeInTheDocument();
    expect(screen.getByTestId('wallet-balance-display')).toBeInTheDocument();

    // Export secret
    const exportBtn = screen.getByTestId('export-secret-btn');
    fireEvent.click(exportBtn);

    await waitFor(() => {
      expect(screen.getByTestId('secret-key-display')).toBeInTheDocument();
    });
    expect(screen.getByTestId('copy-secret-btn')).toBeInTheDocument();

    // Forget demo wallet
    const forgetBtn = screen.getByTestId('forget-demo-btn');
    fireEvent.click(forgetBtn);

    await waitFor(() => {
      expect(screen.queryByTestId('connected-wallet-card')).not.toBeInTheDocument();
    });
  });

  it('funds demo wallet with Friendbot', async () => {
    vi.mocked(freighterApi.isConnected).mockResolvedValue({ isConnected: false });
    const originalFetch = globalThis.fetch;
    const mockFetch = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (typeof url === 'string' && url.includes('friendbot')) {
        return Promise.resolve(new Response(JSON.stringify({ successful: true }), { status: 200 }));
      }
      if (typeof url === 'string' && url.includes('horizon-testnet')) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              balances: [{ asset_type: 'native', balance: '10000.0000000' }],
            }),
            { status: 200 },
          ),
        );
      }
      return originalFetch(url, init);
    });
    vi.stubGlobal('fetch', mockFetch);

    render(
      <OracleProvider>
        <WalletProvider>
          <WalletView />
        </WalletProvider>
      </OracleProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('generate-demo-wallet-btn')).toBeInTheDocument();
    });

    const generateBtn = screen.getByTestId('generate-demo-wallet-btn');
    fireEvent.click(generateBtn);

    await waitFor(() => {
      expect(screen.getByTestId('fund-friendbot-btn')).toBeInTheDocument();
    });

    const fundBtn = screen.getByTestId('fund-friendbot-btn');
    fireEvent.click(fundBtn);

    await waitFor(() => {
      expect(screen.getByTestId('wallet-balance-display')).toHaveTextContent('10000.00 XLM');
    });
  });
});
