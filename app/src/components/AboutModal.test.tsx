import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { axe } from 'vitest-axe';
import * as matchers from 'vitest-axe/matchers';
import { AboutModal, CANONICAL_TRUST_STATEMENT } from './AboutModal';

expect.extend(matchers);

declare module 'vitest' {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-explicit-any
  interface Assertion<T = any> {
    toHaveNoViolations(): void;
  }
}

describe('AboutModal Component', () => {
  it('does not render when isOpen is false', () => {
    const { container } = render(<AboutModal isOpen={false} onClose={vi.fn()} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('renders canonical trust statement and links when isOpen is true', () => {
    render(<AboutModal isOpen={true} onClose={vi.fn()} />);

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('About EchoGrid')).toBeInTheDocument();

    const statement = screen.getByTestId('canonical-trust-statement');
    expect(statement).toBeInTheDocument();
    expect(statement.textContent?.trim()).toBe(CANONICAL_TRUST_STATEMENT);

    const readmeLink = screen.getByRole('link', { name: /Read Full Documentation/i });
    expect(readmeLink).toHaveAttribute('href', 'https://github.com/Web3onInfra/EchoGrid#readme');

    const contractLink = screen.getByRole('link', { name: /View Escrow Contract/i });
    expect(contractLink).toHaveAttribute(
      'href',
      'https://stellar.expert/explorer/testnet/contract/CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF',
    );
  });

  it('calls onClose when close button or Escape key is pressed', () => {
    const onClose = vi.fn();
    render(<AboutModal isOpen={true} onClose={onClose} />);

    const closeBtn = screen.getByRole('button', { name: /Close dialog/i });
    fireEvent.click(closeBtn);
    expect(onClose).toHaveBeenCalledTimes(1);

    fireEvent.keyDown(window, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it('has no accessibility violations (axe-core)', async () => {
    const { container } = render(<AboutModal isOpen={true} onClose={vi.fn()} />);
    const results = await axe(container);
    expect(results).toHaveNoViolations();
    expect(results.violations).toHaveLength(0);
  });
});
