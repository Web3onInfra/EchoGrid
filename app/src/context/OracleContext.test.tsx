import { describe, it, expect, beforeAll, afterAll, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { setupServer } from 'msw/node';
import { http, HttpResponse } from 'msw';
import { handlers, mockConfig, mockReady } from '../api/fixtures';
import { OracleProvider, useOracle } from './OracleContext';
import { DegradedStateBanner } from '../components/DegradedStateBanner';

const server = setupServer(...handlers);

const TestComponent = () => {
  const { config, loading, isDegraded } = useOracle();
  if (loading) return <div>Loading...</div>;
  return (
    <div>
      <div>Network: {config?.network}</div>
      <div>Degraded: {isDegraded ? 'yes' : 'no'}</div>
      <DegradedStateBanner />
    </div>
  );
};

describe('OracleContext and DegradedStateBanner', () => {
  beforeAll(() => server.listen());
  afterEach(() => server.resetHandlers());
  afterAll(() => server.close());

  it('bootstraps successfully when healthy', async () => {
    render(
      <OracleProvider>
        <TestComponent />
      </OracleProvider>,
    );

    await waitFor(() => {
      expect(screen.getByText('Network: testnet')).toBeInTheDocument();
    });
    expect(screen.getByText('Degraded: no')).toBeInTheDocument();
    expect(screen.queryByTestId('degraded-banner')).not.toBeInTheDocument();
  });

  it('shows degraded banner when oracle is unreachable', async () => {
    server.use(
      http.get('*/api/v1/config', () => {
        return new HttpResponse(null, { status: 503 });
      }),
    );

    render(
      <OracleProvider>
        <TestComponent />
      </OracleProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('degraded-banner')).toBeInTheDocument();
    });
    expect(screen.getByText('Oracle Unavailable')).toBeInTheDocument();
  });

  it('shows degraded banner when contract is paused', async () => {
    server.use(
      http.get('*/api/v1/ready', () => {
        return HttpResponse.json({ ...mockReady, contract_paused: true });
      }),
    );

    render(
      <OracleProvider>
        <TestComponent />
      </OracleProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('degraded-banner')).toBeInTheDocument();
    });
    expect(screen.getByText('Contract Paused')).toBeInTheDocument();
  });

  it('shows degraded banner when network is not testnet', async () => {
    server.use(
      http.get('*/api/v1/config', () => {
        return HttpResponse.json({ ...mockConfig, network: 'mainnet' });
      }),
    );

    render(
      <OracleProvider>
        <TestComponent />
      </OracleProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('degraded-banner')).toBeInTheDocument();
    });
    expect(screen.getByText('Unsupported Network')).toBeInTheDocument();
  });
});
