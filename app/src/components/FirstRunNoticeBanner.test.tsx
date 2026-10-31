import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { axe } from 'vitest-axe';
import * as matchers from 'vitest-axe/matchers';
import { FirstRunNoticeBanner } from './FirstRunNoticeBanner';
import { CANONICAL_TRUST_STATEMENT } from './AboutModal';

expect.extend(matchers);

declare module 'vitest' {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-explicit-any
  interface Assertion<T = any> {
    toHaveNoViolations(): void;
  }
}

describe('FirstRunNoticeBanner Component', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('renders notice with canonical statement on first visit', () => {
    render(<FirstRunNoticeBanner onOpenAbout={vi.fn()} />);

    expect(screen.getByTestId('first-run-notice')).toBeInTheDocument();
    const statement = screen.getByTestId('first-run-trust-statement');
    expect(statement.textContent?.trim()).toBe(CANONICAL_TRUST_STATEMENT);
  });

  it('dismisses banner and persists state to localStorage on button click', () => {
    render(<FirstRunNoticeBanner onOpenAbout={vi.fn()} />);

    const dismissBtn = screen.getByRole('button', { name: /Understood/i });
    fireEvent.click(dismissBtn);

    expect(screen.queryByTestId('first-run-notice')).not.toBeInTheDocument();
    expect(localStorage.getItem('echogrid_first_run_dismissed')).toBe('true');
  });

  it('triggers onOpenAbout callback when clicked', () => {
    const onOpenAbout = vi.fn();
    render(<FirstRunNoticeBanner onOpenAbout={onOpenAbout} />);

    const linkBtn = screen.getByRole('button', { name: /How it works & Architecture/i });
    fireEvent.click(linkBtn);

    expect(onOpenAbout).toHaveBeenCalledTimes(1);
  });

  it('has no accessibility violations (axe-core)', async () => {
    const { container } = render(<FirstRunNoticeBanner onOpenAbout={vi.fn()} />);
    const results = await axe(container);
    expect(results).toHaveNoViolations();
    expect(results.violations).toHaveLength(0);
  });
});
