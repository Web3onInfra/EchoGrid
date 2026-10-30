import React, { useState, useEffect } from 'react';
import { PersistedReceipt, loadReceiptHistory, clearReceiptHistory } from '../claim/receiptStorage';

export interface ReceiptViewProps {
  initialReceipt?: PersistedReceipt | null;
}

export const ReceiptView: React.FC<ReceiptViewProps> = ({ initialReceipt }) => {
  const [history, setHistory] = useState<PersistedReceipt[]>([]);
  const [activeReceipt, setActiveReceipt] = useState<PersistedReceipt | null>(
    initialReceipt || null,
  );
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const [shareFeedback, setShareFeedback] = useState<string | null>(null);

  useEffect(() => {
    const list = loadReceiptHistory();
    setHistory(list);
    if (!initialReceipt && list.length > 0) {
      setActiveReceipt(list[0]);
    }
  }, [initialReceipt]);

  const copyToClipboard = async (text: string, key: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedKey(key);
      setTimeout(() => setCopiedKey(null), 2000);
    } catch {
      // ignore
    }
  };

  const handleShare = async (receipt: PersistedReceipt) => {
    const shareText = receipt.simulated
      ? `EchoGrid Simulated Claim (${receipt.claimId}): ${(receipt.amountStroops / 1e7).toFixed(2)} XLM (Simulated)`
      : `EchoGrid Settlement Confirmed: ${(receipt.amountStroops / 1e7).toFixed(2)} XLM paid on Stellar Testnet! Tx: ${receipt.txHash}`;

    if (navigator.share) {
      try {
        await navigator.share({
          title: 'EchoGrid Bounty Receipt',
          text: shareText,
          url: receipt.explorerUrl || window.location.href,
        });
        return;
      } catch {
        // Fall back to copy
      }
    }

    await copyToClipboard(shareText, 'share');
    setShareFeedback('Copied share summary to clipboard!');
    setTimeout(() => setShareFeedback(null), 2500);
  };

  const handleClear = () => {
    clearReceiptHistory();
    setHistory([]);
    setActiveReceipt(null);
  };

  const formatStroops = (stroops: number) => {
    return (stroops / 1e7).toFixed(7);
  };

  return (
    <div className="space-y-4">
      {/* View Header */}
      <div className="p-4 rounded-2xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 shadow-sm">
        <span className="text-[11px] font-bold uppercase tracking-wider text-emerald-600 dark:text-emerald-400">
          Stellar Settlement
        </span>
        <h2 className="text-xl font-extrabold text-slate-900 dark:text-white mt-0.5">
          Claim Receipt &amp; History
        </h2>
        <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">
          Cryptographic confirmation of Proof-of-Power-Down bounties settled on-chain.
        </p>
      </div>

      {/* Active Receipt Detail Card */}
      {activeReceipt ? (
        <div
          data-testid="receipt-card"
          className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-4 shadow-sm space-y-4"
        >
          {/* Status Banner */}
          {activeReceipt.simulated ? (
            <div
              data-testid="simulated-badge"
              className="p-3 rounded-xl bg-purple-500/10 border border-purple-500/20 text-purple-700 dark:text-purple-300 space-y-1"
            >
              <div className="flex items-center gap-2 font-bold text-xs uppercase tracking-wide">
                <span className="w-2 h-2 rounded-full bg-purple-500 animate-pulse" />
                <span>Simulated Payout (Zero On-Chain XLM)</span>
              </div>
              <p className="text-[11px] text-purple-600/90 dark:text-purple-400/90">
                Synthetic simulator mode verification. Payout recorded without consuming live
                testnet Lumens.
              </p>
            </div>
          ) : activeReceipt.status === 'confirmed' ? (
            <div
              data-testid="confirmed-badge"
              className="p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/20 text-emerald-800 dark:text-emerald-300 flex items-center gap-2 font-bold text-xs"
            >
              <span className="w-2 h-2 rounded-full bg-emerald-500" />
              <span>Settled On-Chain (Stellar Testnet)</span>
            </div>
          ) : activeReceipt.status === 'pending' ? (
            <div
              data-testid="pending-badge"
              className="p-3 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-800 dark:text-amber-300 flex items-center gap-2 font-bold text-xs animate-pulse"
            >
              <span className="w-2 h-2 rounded-full bg-amber-500" />
              <span>Settlement Pending (Polling Oracle...)</span>
            </div>
          ) : (
            <div
              data-testid="failed-badge"
              className="p-3 rounded-xl bg-rose-500/10 border border-rose-500/20 text-rose-800 dark:text-rose-300 space-y-1"
            >
              <div className="flex items-center gap-2 font-bold text-xs">
                <span className="w-2 h-2 rounded-full bg-rose-500" />
                <span>Settlement Failed</span>
              </div>
              {activeReceipt.error && (
                <p className="text-[11px] text-rose-600 dark:text-rose-400">
                  {activeReceipt.error}
                </p>
              )}
            </div>
          )}

          {/* Amount Display */}
          <div className="text-center py-2 border-y border-slate-100 dark:border-slate-800/80">
            <span className="text-[11px] font-semibold text-slate-500 uppercase tracking-wider block">
              Bounty Amount
            </span>
            <div data-testid="bounty-amount" className="text-2xl font-black text-slate-900 dark:text-white mt-0.5">
              {formatStroops(activeReceipt.amountStroops)} XLM
              {activeReceipt.simulated && (
                <span className="ml-1 text-xs font-semibold text-purple-600 dark:text-purple-400">
                  (Simulated)
                </span>
              )}
            </div>
          </div>

          {/* Key Attributes */}
          <div className="space-y-2 text-xs">
            <div className="flex items-center justify-between p-2 rounded-lg bg-slate-50 dark:bg-slate-800/60">
              <span className="text-slate-500">Claim ID</span>
              <button
                onClick={() => copyToClipboard(activeReceipt.claimId, 'claimId')}
                data-testid="copy-claim-id-btn"
                className="font-mono text-[11px] text-slate-700 dark:text-slate-300 hover:text-emerald-600 dark:hover:text-emerald-400 flex items-center gap-1.5"
                title="Copy Claim ID"
              >
                <span>{activeReceipt.claimId.slice(0, 16)}...</span>
                <span className="text-[10px] text-emerald-600 dark:text-emerald-400">
                  {copiedKey === 'claimId' ? 'Copied!' : 'Copy'}
                </span>
              </button>
            </div>

            <div className="flex items-center justify-between p-2 rounded-lg bg-slate-50 dark:bg-slate-800/60">
              <span className="text-slate-500">Wallet</span>
              <span className="font-mono text-[11px] text-slate-700 dark:text-slate-300">
                {activeReceipt.walletAddress.slice(0, 6)}...
                {activeReceipt.walletAddress.slice(-6)}
              </span>
            </div>

            {activeReceipt.txHash && (
              <div className="flex items-center justify-between p-2 rounded-lg bg-slate-50 dark:bg-slate-800/60">
                <span className="text-slate-500">Tx Hash</span>
                <button
                  onClick={() => copyToClipboard(activeReceipt.txHash!, 'txHash')}
                  data-testid="copy-tx-btn"
                  className="font-mono text-[11px] text-slate-700 dark:text-slate-300 hover:text-emerald-600 dark:hover:text-emerald-400 flex items-center gap-1.5"
                  title="Copy Transaction Hash"
                >
                  <span>{activeReceipt.txHash.slice(0, 10)}...</span>
                  <span className="text-[10px] text-emerald-600 dark:text-emerald-400">
                    {copiedKey === 'txHash' ? 'Copied!' : 'Copy'}
                  </span>
                </button>
              </div>
            )}

            <div className="flex items-center justify-between p-2 rounded-lg bg-slate-50 dark:bg-slate-800/60">
              <span className="text-slate-500">Timestamp</span>
              <span className="text-[11px] text-slate-700 dark:text-slate-300">
                {new Date(activeReceipt.timestamp).toLocaleString()}
              </span>
            </div>
          </div>

          {/* Action Row */}
          <div className="flex gap-2 pt-1">
            {activeReceipt.txHash && !activeReceipt.simulated && (
              <a
                href={
                  activeReceipt.explorerUrl ||
                  `https://stellar.expert/explorer/testnet/tx/${activeReceipt.txHash}`
                }
                target="_blank"
                rel="noopener noreferrer"
                data-testid="explorer-link"
                className="touch-target flex-1 py-2.5 px-3 rounded-xl bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-800 dark:text-slate-200 text-xs font-semibold text-center transition flex items-center justify-center gap-1.5"
              >
                <span>View on StellarExpert</span>
                <span className="text-[10px]">↗</span>
              </a>
            )}

            <button
              onClick={() => handleShare(activeReceipt)}
              data-testid="share-btn"
              className="touch-target flex-1 py-2.5 px-3 rounded-xl bg-emerald-600 hover:bg-emerald-500 active:scale-[0.98] text-white text-xs font-semibold text-center transition flex items-center justify-center gap-1.5 shadow-sm"
            >
              <span>Share Receipt</span>
            </button>
          </div>

          {shareFeedback && (
            <p className="text-[11px] text-center text-emerald-600 dark:text-emerald-400 font-medium">
              {shareFeedback}
            </p>
          )}
        </div>
      ) : (
        <div
          data-testid="empty-receipt-card"
          className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-6 text-center space-y-2 shadow-sm"
        >
          <div className="w-10 h-10 mx-auto rounded-full bg-slate-100 dark:bg-slate-800 flex items-center justify-center text-slate-400">
            📄
          </div>
          <h3 className="font-bold text-sm text-slate-800 dark:text-slate-200">
            No Settlement Selected
          </h3>
          <p className="text-xs text-slate-500 dark:text-slate-400 max-w-xs mx-auto">
            Power down an appliance during a peak grid event to generate an on-chain bounty receipt.
          </p>
        </div>
      )}

      {/* History Section */}
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-4 shadow-sm space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="font-bold text-xs uppercase tracking-wider text-slate-600 dark:text-slate-400">
            Previous Claims ({history.length})
          </h3>
          {history.length > 0 && (
            <button
              onClick={handleClear}
              data-testid="clear-history-btn"
              className="touch-target text-[11px] text-rose-600 dark:text-rose-400 hover:underline font-semibold"
            >
              Clear
            </button>
          )}
        </div>

        {history.length === 0 ? (
          <p className="text-xs text-slate-400 text-center py-3">No previous claims recorded.</p>
        ) : (
          <div data-testid="history-list" className="space-y-1.5 max-h-56 overflow-y-auto">
            {history.map((item) => (
              <button
                key={item.claimId}
                onClick={() => setActiveReceipt(item)}
                data-testid={`history-item-${item.claimId}`}
                className={`w-full text-left p-2.5 rounded-xl border text-xs transition flex items-center justify-between ${
                  activeReceipt?.claimId === item.claimId
                    ? 'border-emerald-500 bg-emerald-50/50 dark:bg-emerald-950/20'
                    : 'border-slate-100 dark:border-slate-800 hover:bg-slate-50 dark:hover:bg-slate-800/50'
                }`}
              >
                <div>
                  <div className="font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
                    <span>{formatStroops(item.amountStroops)} XLM</span>
                    {item.simulated && (
                      <span className="text-[10px] font-bold text-purple-600 dark:text-purple-400">
                        (Simulated)
                      </span>
                    )}
                  </div>
                  <span className="text-[10px] text-slate-400 font-mono">
                    {item.claimId.slice(0, 14)}...
                  </span>
                </div>
                <div className="text-right">
                  <span
                    className={`text-[10px] font-bold uppercase tracking-wide block ${
                      item.status === 'confirmed'
                        ? 'text-emerald-600'
                        : item.status === 'pending'
                          ? 'text-amber-500'
                          : 'text-rose-500'
                    }`}
                  >
                    {item.status}
                  </span>
                  <span className="text-[10px] text-slate-400">
                    {new Date(item.timestamp).toLocaleDateString()}
                  </span>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
