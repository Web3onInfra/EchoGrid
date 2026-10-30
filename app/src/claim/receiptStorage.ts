export interface PersistedReceipt {
  claimId: string;
  walletAddress: string;
  status: 'confirmed' | 'pending' | 'failed';
  amountStroops: number;
  txHash: string | null;
  explorerUrl: string | null;
  simulated: boolean;
  error?: string | null;
  timestamp: string;
}

export const STORAGE_KEY = 'echogrid_receipt_history';
export const MAX_STORED_RECEIPTS = 50;

/**
 * Loads saved receipt history from localStorage wrapped in try/catch.
 * Falls back to an empty list on read errors or corrupt JSON.
 */
export function loadReceiptHistory(): PersistedReceipt[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) {
      return parsed;
    }
    return [];
  } catch {
    return [];
  }
}

/**
 * Saves a new or updated receipt into localStorage, prepending to list,
 * deduplicating by claimId, and capping at MAX_STORED_RECEIPTS.
 */
export function saveReceipt(receipt: PersistedReceipt): void {
  try {
    const current = loadReceiptHistory();
    // Replace if exists, otherwise prepend
    const existingIndex = current.findIndex((r) => r.claimId === receipt.claimId);
    let updated: PersistedReceipt[];

    if (existingIndex >= 0) {
      updated = [...current];
      updated[existingIndex] = receipt;
    } else {
      updated = [receipt, ...current].slice(0, MAX_STORED_RECEIPTS);
    }

    localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
  } catch {
    // Gracefully ignore quota or access errors
  }
}

/**
 * Clears receipt history from localStorage wrapped in try/catch.
 */
export function clearReceiptHistory(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    // Gracefully ignore
  }
}
