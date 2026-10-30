import { describe, it, expect } from 'vitest';
import { isValidStellarAddress, formatAddress } from './validation';

describe('Stellar Address Validation', () => {
  const validAddress = 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT';

  it('validates correct Stellar G... StrKey address', () => {
    expect(isValidStellarAddress(validAddress)).toBe(true);
  });

  it('rejects addresses with invalid checksum', () => {
    // Corrupt one character
    const corrupted = 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXA';
    expect(isValidStellarAddress(corrupted)).toBe(false);
  });

  it('rejects non-G prefixes (e.g. S... secret key or C... contract)', () => {
    expect(isValidStellarAddress('SA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT')).toBe(
      false,
    );
    expect(isValidStellarAddress('CA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT')).toBe(
      false,
    );
  });

  it('rejects short or empty strings', () => {
    expect(isValidStellarAddress('')).toBe(false);
    expect(isValidStellarAddress('GA4ZH5')).toBe(false);
  });

  it('formats address truncation properly', () => {
    expect(formatAddress(validAddress, 4)).toBe('GA4Z...EHXT');
    expect(formatAddress('GABC', 4)).toBe('GABC');
  });
});
