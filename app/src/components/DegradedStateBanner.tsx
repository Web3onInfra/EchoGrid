import React from 'react';
import { useOracle } from '../context/OracleContext';
import { AlertTriangle, RefreshCw } from 'lucide-react';

export const DegradedStateBanner: React.FC = () => {
  const { isDegraded, degradedReason, loading, refresh } = useOracle();

  if (loading || !isDegraded) {
    return null;
  }

  let title = 'Oracle Degraded';
  let message = 'Some functions may be limited.';

  if (degradedReason === 'oracle_down') {
    title = 'Oracle Unavailable';
    message = 'Cannot reach EchoGrid Oracle backend. Offline mode active.';
  } else if (degradedReason === 'wrong_network') {
    title = 'Unsupported Network';
    message = 'EchoGrid operates on Stellar Testnet only.';
  } else if (degradedReason === 'contract_paused') {
    title = 'Contract Paused';
    message = 'Bounty payouts are temporarily suspended by administrator.';
  }

  return (
    <aside
      role="alert"
      data-testid="degraded-banner"
      className="bg-amber-500/15 border-b border-amber-500/30 px-4 py-2.5 text-amber-800 dark:text-amber-200 transition-colors"
    >
      <div className="max-w-md mx-auto flex items-center justify-between gap-3">
        <div className="flex items-center gap-2.5 min-w-0">
          <AlertTriangle className="w-5 h-5 text-amber-600 dark:text-amber-400 shrink-0" />
          <div className="text-xs truncate">
            <span className="font-semibold block">{title}</span>
            <span className="text-amber-700/90 dark:text-amber-300/90 truncate block">
              {message}
            </span>
          </div>
        </div>

        <button
          onClick={() => refresh()}
          aria-label="Retry Oracle connection"
          className="touch-target flex items-center justify-center p-2 rounded-lg text-amber-800 dark:text-amber-200 hover:bg-amber-500/20 active:scale-95 transition"
        >
          <RefreshCw className="w-4 h-4" />
        </button>
      </div>
    </aside>
  );
};
