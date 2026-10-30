import React, { useState } from 'react';
import { useWallet } from '../context/WalletContext';
import { isValidStellarAddress, formatAddress } from '../wallet/validation';
import { Wallet, Copy, Check, LogOut, ArrowRight, ShieldCheck, AlertCircle } from 'lucide-react';

export const WalletView: React.FC = () => {
  const { isConnected, address, type, connectPaste, disconnect } = useWallet();
  const [pasteInput, setPasteInput] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const handlePasteSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    const clean = pasteInput.trim();
    if (!isValidStellarAddress(clean)) {
      setError('Please enter a valid 56-character Stellar G... public address.');
      return;
    }
    try {
      await connectPaste(clean);
      setPasteInput('');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to connect address');
    }
  };

  const handleCopy = () => {
    if (address) {
      navigator.clipboard?.writeText(address);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div className="space-y-4">
      {/* Header Info */}
      <div className="p-4 rounded-2xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 shadow-sm transition-colors">
        <h2 className="text-xl font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <Wallet className="w-5 h-5 text-emerald-500" />
          Stellar Wallet
        </h2>
        <p className="text-xs text-slate-600 dark:text-slate-400 mt-1">
          Connect your Stellar account to receive on-chain NegaWatt payouts on Testnet.
        </p>
      </div>

      {/* Connected Account Card */}
      {isConnected && address && (
        <div
          data-testid="connected-wallet-card"
          className="p-5 rounded-2xl bg-emerald-500/10 border border-emerald-500/30 space-y-3"
        >
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold px-2.5 py-0.5 rounded-full bg-emerald-500/20 text-emerald-800 dark:text-emerald-300">
              {type === 'paste' ? 'Read-Only Address' : type}
            </span>
            <span className="text-xs text-emerald-700 dark:text-emerald-400 font-medium flex items-center gap-1">
              <ShieldCheck className="w-3.5 h-3.5" />
              Connected (Testnet)
            </span>
          </div>

          <div>
            <span className="text-xs text-slate-500 dark:text-slate-400 block">
              Stellar Address
            </span>
            <span
              data-testid="connected-address"
              className="text-base font-bold font-mono text-slate-900 dark:text-white block mt-0.5"
            >
              {formatAddress(address, 6)}
            </span>
            <span className="text-[11px] font-mono text-slate-500 dark:text-slate-400 block truncate mt-0.5">
              {address}
            </span>
          </div>

          <div className="flex items-center gap-2 pt-2 border-t border-emerald-500/20">
            <button
              onClick={handleCopy}
              data-testid="copy-address-btn"
              className="touch-target flex-1 flex items-center justify-center gap-1.5 px-3 py-2 rounded-xl bg-white dark:bg-slate-800 text-xs font-semibold text-slate-700 dark:text-slate-200 border border-slate-200 dark:border-slate-700 hover:bg-slate-50"
            >
              {copied ? (
                <Check className="w-4 h-4 text-emerald-500" />
              ) : (
                <Copy className="w-4 h-4" />
              )}
              {copied ? 'Copied!' : 'Copy Address'}
            </button>

            <button
              onClick={() => disconnect()}
              data-testid="disconnect-wallet-btn"
              className="touch-target flex items-center justify-center gap-1 px-3 py-2 rounded-xl text-xs font-semibold text-rose-600 dark:text-rose-400 hover:bg-rose-500/10 transition"
            >
              <LogOut className="w-4 h-4" />
              Disconnect
            </button>
          </div>
        </div>
      )}

      {/* Paste Address Provider Option */}
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-5 shadow-sm space-y-3 transition-colors">
        <h3 className="text-sm font-bold text-slate-900 dark:text-white">
          Enter Stellar Address (Read-Only)
        </h3>
        <p className="text-xs text-slate-500 dark:text-slate-400">
          Paste any testnet G... public key to monitor appliance power-downs and bind settlement
          challenges.
        </p>

        <form onSubmit={handlePasteSubmit} className="space-y-3">
          <div>
            <input
              type="text"
              value={pasteInput}
              onChange={(e) => {
                setPasteInput(e.target.value);
                if (error) setError(null);
              }}
              data-testid="paste-address-input"
              placeholder="e.g. GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT"
              className="w-full px-3.5 py-2.5 rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-white font-mono text-xs focus:outline-none focus:ring-2 focus:ring-emerald-500"
            />
            {error && (
              <span
                data-testid="paste-error-message"
                className="text-xs text-rose-600 dark:text-rose-400 flex items-center gap-1 mt-1.5"
              >
                <AlertCircle className="w-3.5 h-3.5" />
                {error}
              </span>
            )}
          </div>

          <button
            type="submit"
            data-testid="paste-submit-btn"
            className="touch-target w-full py-2.5 px-4 rounded-xl bg-emerald-600 hover:bg-emerald-500 active:scale-[0.98] text-white font-semibold text-xs transition flex items-center justify-center gap-1.5"
          >
            <span>Set Address</span>
            <ArrowRight className="w-4 h-4" />
          </button>
        </form>
      </div>

      {/* Freighter & Demo Wallet Provider Placeholders (PR 7 and PR 8) */}
      <div className="space-y-2 opacity-75">
        <div className="p-4 rounded-2xl bg-slate-100 dark:bg-slate-900/50 border border-slate-200 dark:border-slate-800 flex items-center justify-between">
          <div>
            <span className="text-xs font-bold text-slate-900 dark:text-white block">
              Freighter Browser Extension
            </span>
            <span className="text-[11px] text-slate-500 dark:text-slate-400">
              Direct connection for desktop & supported mobile browsers
            </span>
          </div>
          <span className="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-slate-200 dark:bg-slate-800 text-slate-600 dark:text-slate-400">
            PR 7
          </span>
        </div>

        <div className="p-4 rounded-2xl bg-slate-100 dark:bg-slate-900/50 border border-slate-200 dark:border-slate-800 flex items-center justify-between">
          <div>
            <span className="text-xs font-bold text-slate-900 dark:text-white block">
              Instant Demo Wallet & Friendbot
            </span>
            <span className="text-[11px] text-slate-500 dark:text-slate-400">
              Ephemeral in-browser testnet keypair funded with 10,000 test lumens
            </span>
          </div>
          <span className="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-slate-200 dark:bg-slate-800 text-slate-600 dark:text-slate-400">
            PR 8
          </span>
        </div>
      </div>
    </div>
  );
};
