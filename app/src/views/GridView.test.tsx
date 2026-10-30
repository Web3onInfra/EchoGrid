import { describe, it, expect, beforeAll, afterAll, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { setupServer } from 'msw/node';
import { handlers } from '../api/fixtures';
import { OracleProvider } from '../context/OracleContext';
import { GridView } from './GridView';

const server = setupServer(...handlers);

describe('GridView', () => {
  beforeAll(() => server.listen());
  afterEach(() => server.resetHandlers());
  afterAll(() => server.close());

  it('renders simulated banner and substation tags', async () => {
    render(
      <OracleProvider>
        <GridView />
      </OracleProvider>,
    );

    expect(screen.getByTestId('simulated-grid-banner')).toBeInTheDocument();
    expect(screen.getByTestId('simulated-substation')).toHaveTextContent(/Simulated/i);
    expect(screen.getByText(/Frequency/i)).toHaveTextContent(/Simulated/i);
  });

  it('switches between simulated scenarios and updates frequency', async () => {
    render(
      <OracleProvider>
        <GridView />
      </OracleProvider>,
    );

    // Initial default scenario: strain
    expect(screen.getByTestId('grid-frequency-value')).toHaveTextContent('49.82 Hz');

    // Switch to normal
    const normalBtn = screen.getByTestId('scenario-btn-normal');
    expect(normalBtn).toHaveClass('touch-target');
    fireEvent.click(normalBtn);
    expect(screen.getByTestId('grid-frequency-value')).toHaveTextContent('50.02 Hz');

    // Switch to emergency
    const emergencyBtn = screen.getByTestId('scenario-btn-emergency');
    fireEvent.click(emergencyBtn);
    expect(screen.getByTestId('grid-frequency-value')).toHaveTextContent('49.51 Hz');
  });

  it('displays verified live bounty from Oracle config', async () => {
    render(
      <OracleProvider>
        <GridView />
      </OracleProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('live-bounty-value')).toHaveTextContent('1.0 XLM');
    });
    expect(screen.getByText('10 XLM')).toBeInTheDocument();
  });
});
