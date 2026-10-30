import { describe, it, expect, beforeEach, beforeAll, afterAll, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { setupServer } from 'msw/node';
import { handlers } from './api/fixtures';
import App from './App';

const server = setupServer(...handlers);

describe('App Shell and Navigation', () => {
  beforeAll(() => server.listen());
  afterEach(() => server.resetHandlers());
  afterAll(() => server.close());

  beforeEach(() => {
    localStorage.clear();
    document.documentElement.className = '';
  });

  it('renders application header with Testnet indicator', async () => {
    render(<App />);
    expect(screen.getByText('EchoGrid')).toBeInTheDocument();
    const indicator = screen.getByTestId('testnet-indicator');
    expect(indicator).toBeInTheDocument();
    expect(indicator).toHaveTextContent('Testnet');

    // Waits for bootstrap
    await waitFor(() => {
      expect(screen.queryByTestId('degraded-banner')).not.toBeInTheDocument();
    });
  });

  it('navigates through all four tabs with minimum touch targets', () => {
    render(<App />);

    // Default tab: Grid
    expect(screen.getByTestId('simulated-grid-banner')).toBeInTheDocument();

    // Tab buttons exist and have touch-target class (44px min)
    const monitorTab = screen.getByRole('tab', { name: 'Monitor' });
    expect(monitorTab).toHaveClass('touch-target');

    // Switch to Monitor
    fireEvent.click(monitorTab);
    expect(screen.getByText('Appliance Hum Detection')).toBeInTheDocument();

    // Switch to Wallet
    const walletTab = screen.getByRole('tab', { name: 'Wallet' });
    fireEvent.click(walletTab);
    expect(screen.getByText('Account & Balance')).toBeInTheDocument();

    // Switch to Receipt
    const receiptTab = screen.getByRole('tab', { name: 'Receipt' });
    fireEvent.click(receiptTab);
    expect(screen.getByText('Settlement History')).toBeInTheDocument();

    // Back to Grid
    const gridTab = screen.getByRole('tab', { name: 'Grid' });
    fireEvent.click(gridTab);
    expect(screen.getByTestId('simulated-grid-banner')).toBeInTheDocument();
  });

  it('toggles light and dark themes', () => {
    render(<App />);
    const themeBtn = screen.getByRole('button', { name: 'Toggle color theme' });
    expect(themeBtn).toHaveClass('touch-target');

    const initialIsDark = document.documentElement.classList.contains('dark');
    fireEvent.click(themeBtn);
    expect(document.documentElement.classList.contains('dark')).toBe(!initialIsDark);
  });
});
