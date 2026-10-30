import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { axe } from 'vitest-axe';
import * as matchers from 'vitest-axe/matchers';
import { GridView } from './views/GridView';
import { MonitorView } from './views/MonitorView';
import { WalletView } from './views/WalletView';
import { ReceiptView } from './views/ReceiptView';
import { AppContent } from './App';
import { ThemeProvider } from './context/ThemeContext';
import { OracleProvider } from './context/OracleContext';
import { WalletProvider } from './context/WalletContext';

expect.extend(matchers);

declare module 'vitest' {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-explicit-any
  interface Assertion<T = any> {
    toHaveNoViolations(): void;
  }
}

const renderWithProviders = (ui: React.ReactElement) => {
  return render(
    <ThemeProvider>
      <OracleProvider>
        <WalletProvider>{ui}</WalletProvider>
      </OracleProvider>
    </ThemeProvider>,
  );
};

describe('Accessibility Audit (axe-core WCAG AA)', () => {
  it('GridView has no accessibility violations', async () => {
    const { container } = renderWithProviders(<GridView />);
    const results = await axe(container);
    expect(results).toHaveNoViolations();
    expect(results.violations).toHaveLength(0);
  });

  it('MonitorView has no accessibility violations', async () => {
    const { container } = renderWithProviders(<MonitorView />);
    const results = await axe(container);
    expect(results).toHaveNoViolations();
    expect(results.violations).toHaveLength(0);
  });

  it('WalletView has no accessibility violations', async () => {
    const { container } = renderWithProviders(<WalletView />);
    const results = await axe(container);
    expect(results).toHaveNoViolations();
    expect(results.violations).toHaveLength(0);
  });

  it('ReceiptView has no accessibility violations', async () => {
    const { container } = renderWithProviders(<ReceiptView />);
    const results = await axe(container);
    expect(results).toHaveNoViolations();
    expect(results.violations).toHaveLength(0);
  });

  it('App Shell and TabBar have no accessibility violations', async () => {
    const { container } = renderWithProviders(<AppContent />);
    const results = await axe(container);
    expect(results).toHaveNoViolations();
    expect(results.violations).toHaveLength(0);
  });
});
