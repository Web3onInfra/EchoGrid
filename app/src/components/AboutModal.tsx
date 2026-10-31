import React, { useEffect, useRef } from 'react';
import { X, ExternalLink, ShieldAlert, Cpu, CheckCircle2 } from 'lucide-react';

export const CANONICAL_TRUST_STATEMENT =
  "The EchoGrid MVP trusts the client's claim of an appliance power-down. While cryptographic challenge nonces, recipient cooldowns, and on-chain spending caps strictly limit potential abuse, they do not prove that an appliance physically turned off. This live demonstration showcases the end-to-end micropayment loop and real-time acoustic detection user experience on Stellar Testnet; physical and cryptographic verification remains post-MVP work.";

export interface AboutModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const AboutModal: React.FC<AboutModalProps> = ({ isOpen, onClose }) => {
  const modalRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    if (isOpen) {
      document.body.style.overflow = 'hidden';
      window.addEventListener('keydown', handleKeyDown);
      modalRef.current?.focus();
    } else {
      document.body.style.overflow = '';
    }

    return () => {
      document.body.style.overflow = '';
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="about-modal-title"
      className="fixed inset-0 z-50 flex items-end sm:items-center justify-center p-0 sm:p-4 bg-slate-950/80 backdrop-blur-sm animate-in fade-in duration-200"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={modalRef}
        tabIndex={-1}
        className="w-full max-w-lg max-h-[85vh] overflow-y-auto bg-white dark:bg-slate-900 border-t sm:border border-slate-200 dark:border-slate-800 rounded-t-3xl sm:rounded-2xl p-6 shadow-2xl flex flex-col gap-5 focus:outline-none"
      >
        <div className="flex items-center justify-between border-b border-slate-100 dark:border-slate-800 pb-3">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-emerald-500/10 text-emerald-500 flex items-center justify-center font-bold">
              <Cpu className="w-5 h-5 text-emerald-500" />
            </div>
            <div>
              <h2
                id="about-modal-title"
                className="text-lg font-bold text-slate-900 dark:text-white leading-tight"
              >
                About EchoGrid
              </h2>
              <span className="text-xs text-slate-500 dark:text-slate-400 font-mono">
                Version v0.1.0-mvp (Testnet)
              </span>
            </div>
          </div>
          <button
            onClick={onClose}
            aria-label="Close dialog"
            className="touch-target p-2 text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:outline-none"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="p-4 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-900 dark:text-amber-200 flex flex-col gap-2">
          <div className="flex items-center gap-2 font-semibold text-sm text-amber-800 dark:text-amber-300">
            <ShieldAlert className="w-4 h-4 text-amber-500 shrink-0" />
            <span>Honest Framing & Trust Model</span>
          </div>
          <p
            data-testid="canonical-trust-statement"
            className="text-xs leading-relaxed font-normal text-slate-700 dark:text-slate-300"
          >
            {CANONICAL_TRUST_STATEMENT}
          </p>
        </div>

        <div className="space-y-3">
          <h3 className="text-sm font-semibold text-slate-900 dark:text-white flex items-center gap-1.5">
            <CheckCircle2 className="w-4 h-4 text-emerald-500" />
            Architecture & Privacy Guarantees
          </h3>
          <ul className="text-xs text-slate-600 dark:text-slate-400 space-y-2 list-disc list-inside">
            <li>
              <strong>Zero Audio Egress:</strong> Microphone audio is processed 100% locally in-browser within Web Workers. No raw sound or audio waveforms ever leave your device.
            </li>
            <li>
              <strong>On-Chain Escrow:</strong> Soroban smart contract holds municipal bounty pools and executes non-custodial payouts on Stellar Testnet.
            </li>
            <li>
              <strong>Hardware Verification:</strong> Smart-meter and acoustic watermark attestation is post-MVP work.
            </li>
          </ul>
        </div>

        <div className="pt-2 border-t border-slate-100 dark:border-slate-800 flex flex-col sm:flex-row gap-3">
          <a
            href="https://github.com/Web3onInfra/EchoGrid#readme"
            target="_blank"
            rel="noopener noreferrer"
            className="flex-1 touch-target flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl text-xs font-semibold bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-800 dark:text-slate-200 transition-colors focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:outline-none"
          >
            <span>Read Full Documentation</span>
            <ExternalLink className="w-3.5 h-3.5" />
          </a>
          <a
            href="https://stellar.expert/explorer/testnet/contract/CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF"
            target="_blank"
            rel="noopener noreferrer"
            className="flex-1 touch-target flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl text-xs font-semibold bg-emerald-600 hover:bg-emerald-500 text-white transition-colors focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:outline-none"
          >
            <span>View Escrow Contract</span>
            <ExternalLink className="w-3.5 h-3.5" />
          </a>
        </div>
      </div>
    </div>
  );
};
