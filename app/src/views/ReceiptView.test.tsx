import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ReceiptView } from './ReceiptView';
import { saveReceipt, clearReceiptHistory, PersistedReceipt } from '../claim/receiptStorage';

describe('ReceiptView', () => {
  beforeEach(() => {
    localStorage.clear();
    clearReceiptHistory();
    vi.restoreAllMocks();
  });

  const confirmedReceipt: PersistedReceipt = {
    claimId: 'clm_confirmed_123',
    walletAddress: 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT',
    status: 'confirmed',
    amountStroops: 10000000,
    txHash: 'tx_hash_real_testnet',
    explorerUrl: 'https://stellar.expert/explorer/testnet/tx/tx_hash_real_testnet',
    simulated: false,
    timestamp: '2026-10-30T10:00:00.000Z',
  };

  const simulatedReceipt: PersistedReceipt = {
    claimId: 'clm_sim_456',
    walletAddress: 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT',
    status: 'confirmed',
    amountStroops: 10000000,
    txHash: 'SIM_TX_abcdef123456',
    explorerUrl: null,
    simulated: true,
    timestamp: '2026-10-30T10:05:00.000Z',
  };

  it('renders empty receipt state when no history exists', () => {
    render(<ReceiptView />);
    expect(screen.getByTestId('empty-receipt-card')).toBeInTheDocument();
    expect(screen.getByText(/No previous claims recorded/i)).toBeInTheDocument();
  });

  it('renders confirmed on-chain receipt with StellarExpert explorer link', () => {
    saveReceipt(confirmedReceipt);
    render(<ReceiptView initialReceipt={confirmedReceipt} />);

    expect(screen.getByTestId('receipt-card')).toBeInTheDocument();
    expect(screen.getByTestId('confirmed-badge')).toHaveTextContent(/Settled On-Chain/i);
    expect(screen.getByTestId('bounty-amount')).toHaveTextContent('1.0000000 XLM');

    const link = screen.getByTestId('explorer-link');
    expect(link).toHaveAttribute('href', confirmedReceipt.explorerUrl);
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', 'noopener noreferrer');
  });

  it('renders distinct "Simulated, no on-chain payout" variant', () => {
    saveReceipt(simulatedReceipt);
    render(<ReceiptView initialReceipt={simulatedReceipt} />);

    expect(screen.getByTestId('simulated-badge')).toHaveTextContent(
      /Simulated Payout \(Zero On-Chain XLM\)/i,
    );
    expect(screen.getAllByText(/\(Simulated\)/i).length).toBeGreaterThan(0);
    // Real explorer link should not be rendered for simulated claims
    expect(screen.queryByTestId('explorer-link')).not.toBeInTheDocument();
  });

  it('renders pending and failed receipt states correctly', () => {
    const pendingReceipt: PersistedReceipt = {
      ...confirmedReceipt,
      claimId: 'clm_pending',
      status: 'pending',
      txHash: null,
    };
    const { unmount } = render(<ReceiptView initialReceipt={pendingReceipt} />);
    expect(screen.getByTestId('pending-badge')).toHaveTextContent(/Settlement Pending/i);
    unmount();

    const failedReceipt: PersistedReceipt = {
      ...confirmedReceipt,
      claimId: 'clm_failed',
      status: 'failed',
      error: 'Smart contract transaction reverted',
      txHash: null,
    };
    render(<ReceiptView initialReceipt={failedReceipt} />);
    expect(screen.getByTestId('failed-badge')).toHaveTextContent(/Settlement Failed/i);
    expect(screen.getByText(/Smart contract transaction reverted/i)).toBeInTheDocument();
  });

  it('supports copying claim ID and tx hash to clipboard', async () => {
    saveReceipt(confirmedReceipt);
    const writeTextSpy = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, {
      clipboard: { writeText: writeTextSpy },
    });

    render(<ReceiptView initialReceipt={confirmedReceipt} />);

    fireEvent.click(screen.getByTestId('copy-claim-id-btn'));
    expect(writeTextSpy).toHaveBeenCalledWith(confirmedReceipt.claimId);

    fireEvent.click(screen.getByTestId('copy-tx-btn'));
    expect(writeTextSpy).toHaveBeenCalledWith(confirmedReceipt.txHash);
  });

  it('shares receipt summary via navigator.share or clipboard fallback', async () => {
    saveReceipt(confirmedReceipt);
    const writeTextSpy = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, {
      clipboard: { writeText: writeTextSpy },
      share: undefined,
    });

    render(<ReceiptView initialReceipt={confirmedReceipt} />);
    fireEvent.click(screen.getByTestId('share-btn'));

    await waitFor(() => {
      expect(screen.getByText(/Copied share summary to clipboard/i)).toBeInTheDocument();
    });
  });

  it('switches active receipt when tapping an item in the history list and clears history', () => {
    saveReceipt(confirmedReceipt);
    saveReceipt(simulatedReceipt);

    render(<ReceiptView />);

    // Initially loads simulatedReceipt (most recent)
    expect(screen.getByTestId('simulated-badge')).toBeInTheDocument();

    // Click older confirmed item
    fireEvent.click(screen.getByTestId(`history-item-${confirmedReceipt.claimId}`));
    expect(screen.getByTestId('confirmed-badge')).toBeInTheDocument();

    // Clear history
    fireEvent.click(screen.getByTestId('clear-history-btn'));
    expect(screen.getByTestId('empty-receipt-card')).toBeInTheDocument();
    expect(screen.getByText(/No previous claims recorded/i)).toBeInTheDocument();
  });
});
