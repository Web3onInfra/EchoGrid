import React from 'react';

export const WalletView: React.FC = () => {
  return (
    <div className="space-y-4">
      <div className="p-4 rounded-2xl bg-slate-100 dark:bg-slate-800 border border-slate-200 dark:border-slate-700">
        <span className="text-xs font-semibold uppercase tracking-wider text-emerald-600 dark:text-emerald-400">
          Stellar Wallet
        </span>
        <h2 className="text-xl font-bold mt-1 text-slate-900 dark:text-white">Account & Balance</h2>
        <p className="text-sm text-slate-600 dark:text-slate-400 mt-1">
          Connect Freighter or create an ephemeral testnet demo keypair.
        </p>
      </div>
    </div>
  );
};
