import React, { useState, useEffect } from 'react';
import { useWallet } from '../context/WalletContext';
import { useOracle } from '../context/OracleContext';
import { isValidStellarAddress, formatAddress } from '../wallet/validation';
import { FreighterWalletProvider } from '../wallet/providers/freighterProvider';
import {
  Wallet,
  Copy,
  Check,
  LogOut,
  ArrowRight,
  ShieldCheck,
  AlertCircle,
  ExternalLink,
  Smartphone,
  Coins,
  KeyRound,
  Trash2,
  AlertTriangle,
  RefreshCw,
} from 'lucide-react';

export const WalletView: React.FC = () => {
  const {
    isConnected,
    address,
    type,
    balance,
    accountExists,
    isFunding,
    connectPaste,
    connectFreighter,
    createDemoWallet,
    fundDemoWallet,
    forgetDemoWallet,
    getDemoSecretKey,
    refreshBalance,
    disconnect,
  } = useWallet();

  const { config } = useOracle();
  const isNetworkTestnet = !config || config.network === 'testnet';

  const [pasteInput, setPasteInput] = useState('');
  const [pasteError, setPasteError] = useState<string | null>(null);
  const [freighterError, setFreighterError] = useState<string | null>(null);
  const [freighterLoading, setFreighterLoading] = useState(false);
  const [hasFreighter, setHasFreighter] = useState<boolean | null>(null);
  const [fundingError, setFundingError] = useState<string | null>(null);
  const [showSecret, setShowSecret] = useState(false);
  const [copiedAddress, setCopiedAddress] = useState(false);
  const [copiedSecret, setCopiedSecret] = useState(false);

  useEffect(() => {
    const provider = new FreighterWalletProvider();
    provider.isAvailable().then(setHasFreighter);
  }, []);

  const handlePasteSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setPasteError(null);
    const clean = pasteInput.trim();
    if (!isValidStellarAddress(clean)) {
      setPasteError('Please enter a valid 56-character Stellar G... public address.');
      return;
    }
    try {
      await connectPaste(clean);
      setPasteInput('');
    } catch (err: unknown) {
      setPasteError(err instanceof Error ? err.message : 'Failed to connect address');
    }
  };

  const handleFreighterConnect = async () => {
    setFreighterError(null);
    setFreighterLoading(true);
    try {
      await connectFreighter();
    } catch (err: unknown) {
      setFreighterError(err instanceof Error ? err.message : 'Freighter connection failed.');
    } finally {
      setFreighterLoading(false);
    }
  };

  const handleCreateDemo = async () => {
    if (!isNetworkTestnet) return;
    await createDemoWallet();
  };

  const handleFundFriendbot = async () => {
    setFundingError(null);
    try {
      await fundDemoWallet();
    } catch (err: unknown) {
      setFundingError(err instanceof Error ? err.message : 'Funding failed');
    }
  };

  const handleCopyAddress = () => {
    if (address) {
      navigator.clipboard?.writeText(address);
      setCopiedAddress(true);
      setTimeout(() => setCopiedAddress(false), 2000);
    }
  };

  const handleCopySecret = () => {
    const sec = getDemoSecretKey();
    if (sec) {
      navigator.clipboard?.writeText(sec);
      setCopiedSecret(true);
      setTimeout(() => setCopiedSecret(false), 2000);
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
              {type === 'paste'
                ? 'Read-Only Address'
                : type === 'freighter'
                  ? 'Freighter'
                  : 'Demo Keypair'}
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

          {/* Balance Row */}
          <div className="flex items-center justify-between p-3 rounded-xl bg-white/70 dark:bg-slate-800/70 border border-emerald-500/20">
            <div>
              <span className="text-[10px] uppercase font-bold text-slate-500 dark:text-slate-400 block tracking-wider">
                Testnet Balance
              </span>
              <span
                data-testid="wallet-balance-display"
                className="text-lg font-extrabold text-slate-900 dark:text-white font-mono"
              >
                {balance} XLM
              </span>
            </div>
            <div className="flex items-center gap-1.5">
              <span
                data-testid="account-status-badge"
                className={`text-[10px] font-semibold px-2 py-0.5 rounded-full ${
                  accountExists
                    ? 'bg-emerald-500/20 text-emerald-700 dark:text-emerald-300'
                    : 'bg-amber-500/20 text-amber-700 dark:text-amber-300'
                }`}
              >
                {accountExists ? 'Active on Ledger' : 'Unfunded'}
              </span>
              <button
                onClick={() => refreshBalance()}
                aria-label="Refresh balance"
                className="p-1 text-slate-500 hover:text-slate-900 dark:text-slate-400 dark:hover:text-white"
              >
                <RefreshCw className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>

          <div className="flex items-center gap-2 pt-2 border-t border-emerald-500/20">
            <button
              onClick={handleCopyAddress}
              data-testid="copy-address-btn"
              className="touch-target flex-1 flex items-center justify-center gap-1.5 px-3 py-2 rounded-xl bg-white dark:bg-slate-800 text-xs font-semibold text-slate-700 dark:text-slate-200 border border-slate-200 dark:border-slate-700 hover:bg-slate-50"
            >
              {copiedAddress ? (
                <Check className="w-4 h-4 text-emerald-500" />
              ) : (
                <Copy className="w-4 h-4" />
              )}
              {copiedAddress ? 'Copied!' : 'Copy Address'}
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

      {/* Demo Wallet & Friendbot Card */}
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-5 shadow-sm space-y-4 transition-colors">
        <div className="flex items-start justify-between">
          <div>
            <h3 className="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-1.5">
              <Coins className="w-4 h-4 text-amber-500" />
              Demo Wallet & Friendbot
            </h3>
            <p className="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
              One-tap ephemeral in-browser keypair funded with 10,000 test lumens.
            </p>
          </div>
          <span className="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/20">
            Demo
          </span>
        </div>

        {/* Mandatory Security Warning */}
        <div
          data-testid="demo-wallet-warning"
          className="p-3 rounded-xl bg-rose-500/10 border border-rose-500/20 text-xs text-rose-700 dark:text-rose-300 flex items-start gap-2"
        >
          <AlertTriangle className="w-4 h-4 text-rose-500 shrink-0 mt-0.5" />
          <div className="leading-relaxed">
            <strong className="font-semibold block">Testnet only, never reuse on mainnet.</strong>
            Keys are generated in browser storage for demo testing. Never store real funds in this
            wallet.
          </div>
        </div>

        {!isNetworkTestnet && (
          <div
            data-testid="demo-wallet-disabled-banner"
            className="p-3 rounded-xl bg-slate-100 dark:bg-slate-800 text-xs text-slate-600 dark:text-slate-300"
          >
            Demo wallet and Friendbot are hard-disabled on non-testnet environments.
          </div>
        )}

        {isNetworkTestnet && (
          <div className="space-y-2.5">
            {type !== 'demo' ? (
              <button
                onClick={handleCreateDemo}
                data-testid="generate-demo-wallet-btn"
                className="touch-target w-full py-2.5 px-4 rounded-xl bg-amber-600 hover:bg-amber-500 active:scale-[0.98] text-white font-semibold text-xs transition flex items-center justify-center gap-1.5"
              >
                <Coins className="w-4 h-4" />
                <span>Generate Testnet Demo Wallet</span>
              </button>
            ) : (
              <div className="space-y-2.5">
                {fundingError && (
                  <span className="text-xs text-rose-600 dark:text-rose-400 flex items-center gap-1">
                    <AlertCircle className="w-3.5 h-3.5" />
                    {fundingError}
                  </span>
                )}

                <button
                  onClick={handleFundFriendbot}
                  disabled={isFunding}
                  data-testid="fund-friendbot-btn"
                  className="touch-target w-full py-2.5 px-4 rounded-xl bg-emerald-600 hover:bg-emerald-500 active:scale-[0.98] disabled:opacity-50 text-white font-semibold text-xs transition flex items-center justify-center gap-1.5"
                >
                  <Coins className="w-4 h-4" />
                  <span>
                    {isFunding
                      ? 'Funding via Friendbot (+10,000 XLM)...'
                      : 'Fund Account (+10,000 XLM via Friendbot)'}
                  </span>
                </button>

                <div className="flex items-center gap-2">
                  <button
                    onClick={() => setShowSecret(!showSecret)}
                    data-testid="export-secret-btn"
                    className="touch-target flex-1 py-2 px-3 rounded-xl border border-slate-200 dark:border-slate-700 text-xs font-semibold text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-800 flex items-center justify-center gap-1.5"
                  >
                    <KeyRound className="w-3.5 h-3.5" />
                    <span>{showSecret ? 'Hide Secret Key' : 'Export Secret Key'}</span>
                  </button>

                  <button
                    onClick={() => forgetDemoWallet()}
                    data-testid="forget-demo-btn"
                    className="touch-target py-2 px-3 rounded-xl text-xs font-semibold text-rose-600 dark:text-rose-400 hover:bg-rose-500/10 flex items-center justify-center gap-1"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                    <span>Forget</span>
                  </button>
                </div>

                {showSecret && (
                  <div
                    data-testid="secret-key-display"
                    className="p-3 rounded-xl bg-slate-900 text-slate-100 font-mono text-xs space-y-2 border border-slate-800"
                  >
                    <span className="text-[10px] text-amber-400 uppercase font-bold block">
                      Secret Key (Ed25519)
                    </span>
                    <span className="block truncate text-[11px] select-all">
                      {getDemoSecretKey()}
                    </span>
                    <button
                      onClick={handleCopySecret}
                      data-testid="copy-secret-btn"
                      className="touch-target px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-[11px] font-sans font-semibold flex items-center gap-1"
                    >
                      {copiedSecret ? (
                        <Check className="w-3.5 h-3.5 text-emerald-400" />
                      ) : (
                        <Copy className="w-3.5 h-3.5" />
                      )}
                      {copiedSecret ? 'Copied Secret!' : 'Copy Secret'}
                    </button>
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Freighter Extension Card */}
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-5 shadow-sm space-y-3 transition-colors">
        <div className="flex items-start justify-between">
          <div>
            <h3 className="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-1.5">
              Freighter Browser Extension
            </h3>
            <p className="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
              Standard non-custodial Stellar wallet for desktop & supported Web3 browsers.
            </p>
          </div>
          <span className="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-600 dark:text-blue-400 border border-blue-500/20">
            Extension
          </span>
        </div>

        {hasFreighter === false && (
          <div
            data-testid="freighter-unavailable-notice"
            className="p-3 rounded-xl bg-slate-100 dark:bg-slate-800/80 border border-slate-200 dark:border-slate-700 text-xs text-slate-600 dark:text-slate-300 space-y-1.5"
          >
            <div className="flex items-center gap-1.5 font-medium text-slate-800 dark:text-slate-200">
              <Smartphone className="w-4 h-4 text-amber-500 shrink-0" />
              Not available on this device or mobile browser
            </div>
            <p className="text-[11px] leading-relaxed">
              Mobile browsers (iOS Safari, Android Chrome) do not support extensions. Please use the{' '}
              <strong>Demo Wallet</strong> above or <strong>Paste Address</strong>.
            </p>
            <a
              href="https://www.freighter.app/"
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-1 text-[11px] text-blue-600 dark:text-blue-400 font-semibold hover:underline"
            >
              Install Freighter on Desktop <ExternalLink className="w-3 h-3" />
            </a>
          </div>
        )}

        {freighterError && (
          <div
            data-testid="freighter-error-message"
            className="p-3 rounded-xl bg-rose-500/10 border border-rose-500/20 text-xs text-rose-700 dark:text-rose-300 flex items-start gap-2"
          >
            <AlertCircle className="w-4 h-4 text-rose-500 shrink-0 mt-0.5" />
            <span className="leading-relaxed">{freighterError}</span>
          </div>
        )}

        <button
          onClick={handleFreighterConnect}
          disabled={freighterLoading}
          data-testid="connect-freighter-btn"
          className="touch-target w-full py-2.5 px-4 rounded-xl bg-blue-600 hover:bg-blue-500 active:scale-[0.98] disabled:opacity-50 text-white font-semibold text-xs transition flex items-center justify-center gap-1.5"
        >
          <span>{freighterLoading ? 'Connecting to Freighter...' : 'Connect Freighter'}</span>
          <ArrowRight className="w-4 h-4" />
        </button>
      </div>

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
                if (pasteError) setPasteError(null);
              }}
              data-testid="paste-address-input"
              placeholder="e.g. GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT"
              className="w-full px-3.5 py-2.5 rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-white font-mono text-xs focus:outline-none focus:ring-2 focus:ring-emerald-500"
            />
            {pasteError && (
              <span
                data-testid="paste-error-message"
                className="text-xs text-rose-600 dark:text-rose-400 flex items-center gap-1 mt-1.5"
              >
                <AlertCircle className="w-3.5 h-3.5" />
                {pasteError}
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
    </div>
  );
};
