import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { BandMeters } from './BandMeters';

describe('BandMeters', () => {
  it('renders all three mains band meters with energy values', () => {
    const energies = [43.5, 41.2, 38.0];
    render(<BandMeters bandEnergiesDb={energies} />);

    expect(screen.getByTestId('band-meters-container')).toBeInTheDocument();
    expect(screen.getByText('50 Hz (Mains)')).toBeInTheDocument();
    expect(screen.getByText('60 Hz (Mains)')).toBeInTheDocument();
    expect(screen.getByText('120 Hz (Harmonic)')).toBeInTheDocument();

    const meters = screen.getAllByRole('meter');
    expect(meters.length).toBe(3);
    expect(meters[0]).toHaveAttribute('aria-valuenow', '44');
    expect(meters[1]).toHaveAttribute('aria-valuenow', '41');
    expect(meters[2]).toHaveAttribute('aria-valuenow', '38');
  });

  it('clamps extreme values within minDb and maxDb', () => {
    const energies = [-120.0, 100.0];
    render(
      <BandMeters bandEnergiesDb={energies} labels={['Low', 'High']} minDb={-60} maxDb={60} />,
    );

    const meters = screen.getAllByRole('meter');
    expect(meters[0]).toHaveAttribute('aria-valuemin', '-60');
    expect(meters[0]).toHaveAttribute('aria-valuemax', '60');
  });
});
