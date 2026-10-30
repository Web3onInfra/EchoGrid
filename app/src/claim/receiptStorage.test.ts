import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  loadReceiptHistory,
  saveReceipt,
  clearReceiptHistory,
  STORAGE_KEY,
  PersistedReceipt,
  MAX_STORED_RECEIPTS,
} from './receiptStorage';

describe('receiptStorage', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.restoreAllMocks();
  });

  const mockReceipt: PersistedReceipt = {
    claimId: 'clm_test_1',
    walletAddress: 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT',
    status: 'confirmed',
    amountStroops: 10000000,
    txHash: 'tx_abc',
    explorerUrl: 'https://stellar.expert/explorer/testnet/tx/tx_abc',
    simulated: false,
    timestamp: '2026-10-30T10:00:00.000Z',
  };

  it('loads empty array when localStorage is empty or corrupt', () => {
    expect(loadReceiptHistory()).toEqual([]);

    localStorage.setItem(STORAGE_KEY, 'not-valid-json{');
    expect(loadReceiptHistory()).toEqual([]);

    localStorage.setItem(STORAGE_KEY, JSON.stringify({ not: 'an array' }));
    expect(loadReceiptHistory()).toEqual([]);
  });

  it('saves and loads receipts, prepending newer items', () => {
    saveReceipt(mockReceipt);
    const loaded = loadReceiptHistory();
    expect(loaded).toHaveLength(1);
    expect(loaded[0].claimId).toBe('clm_test_1');

    const mockReceipt2: PersistedReceipt = {
      ...mockReceipt,
      claimId: 'clm_test_2',
    };
    saveReceipt(mockReceipt2);

    const loaded2 = loadReceiptHistory();
    expect(loaded2).toHaveLength(2);
    expect(loaded2[0].claimId).toBe('clm_test_2');
    expect(loaded2[1].claimId).toBe('clm_test_1');
  });

  it('updates existing receipt in place when claimId matches', () => {
    saveReceipt(mockReceipt);

    const updatedReceipt: PersistedReceipt = {
      ...mockReceipt,
      status: 'confirmed',
      txHash: 'tx_updated',
    };
    saveReceipt(updatedReceipt);

    const loaded = loadReceiptHistory();
    expect(loaded).toHaveLength(1);
    expect(loaded[0].txHash).toBe('tx_updated');
  });

  it('caps history at MAX_STORED_RECEIPTS', () => {
    for (let i = 0; i < MAX_STORED_RECEIPTS + 10; i++) {
      saveReceipt({
        ...mockReceipt,
        claimId: `clm_${i}`,
      });
    }

    const loaded = loadReceiptHistory();
    expect(loaded).toHaveLength(MAX_STORED_RECEIPTS);
    expect(loaded[0].claimId).toBe(`clm_${MAX_STORED_RECEIPTS + 9}`);
  });

  it('handles localStorage errors gracefully without throwing', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('Access denied');
    });
    expect(() => loadReceiptHistory()).not.toThrow();
    expect(loadReceiptHistory()).toEqual([]);

    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('Quota exceeded');
    });
    expect(() => saveReceipt(mockReceipt)).not.toThrow();

    vi.spyOn(Storage.prototype, 'removeItem').mockImplementation(() => {
      throw new Error('Storage error');
    });
    expect(() => clearReceiptHistory()).not.toThrow();
  });

  it('clears receipt history', () => {
    saveReceipt(mockReceipt);
    expect(loadReceiptHistory()).toHaveLength(1);
    clearReceiptHistory();
    expect(loadReceiptHistory()).toHaveLength(0);
  });
});
