import React, { useState, useEffect } from 'react';
import { ShieldAlert, X } from 'lucide-react';
import { CANONICAL_TRUST_STATEMENT } from './AboutModal';

export interface FirstRunNoticeBannerProps {
  onOpenAbout: () => void;
}

const STORAGE_KEY = 'echogrid_first_run_dismissed';

export const FirstRunNoticeBanner: React.FC<FirstRunNoticeBannerProps> = ({
  onOpenAbout,
}) => {
  const [isDismissed, setIsDismissed] = useState<boolean>(true);

  useEffect(() => {
    try {
      const dismissed = localStorage.getItem(STORAGE_KEY);
      if (dismissed !== 'true') {
        setIsDismissed(false);
      }
    } catch {
      // Fallback for sandboxed environments
      setIsDismissed(false);
    }
  }, []);

  const handleDismiss = () => {
    setIsDismissed(true);
    try {
      localStorage.setItem(STORAGE_KEY, 'true');
    } catch {
      // Ignore storage errors
    }
  };

  if (isDismissed) return null;

  return (
    <div
      role="region"
      aria-label="First-run trust notice"
      data-testid="first-run-notice"
      className="bg-amber-500/10 border-b border-amber-500/20 px-4 py-3 text-slate-800 dark:text-slate-200 transition-all"
    >
      <div className="max-w-md mx-auto flex flex-col gap-2">
        <div className="flex items-start justify-between gap-2">
          <div className="flex items-center gap-1.5 font-semibold text-xs text-amber-800 dark:text-amber-300">
            <ShieldAlert className="w-4 h-4 text-amber-500 shrink-0" />
            <span>EchoGrid MVP Transparency Notice</span>
          </div>
          <button
            onClick={handleDismiss}
            aria-label="Dismiss trust notice"
            className="touch-target p-1 text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 rounded focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:outline-none"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <p
          data-testid="first-run-trust-statement"
          className="text-xs leading-relaxed text-slate-700 dark:text-slate-300"
        >
          {CANONICAL_TRUST_STATEMENT}
        </p>

        <div className="flex items-center gap-3 pt-1">
          <button
            onClick={onOpenAbout}
            className="text-xs font-semibold text-emerald-600 dark:text-emerald-400 hover:underline focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:outline-none"
          >
            How it works & Architecture &rarr;
          </button>
          <button
            onClick={handleDismiss}
            className="text-xs px-2.5 py-1 rounded-md bg-amber-500/20 hover:bg-amber-500/30 font-medium text-amber-900 dark:text-amber-200 transition-colors focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:outline-none"
          >
            Understood
          </button>
        </div>
      </div>
    </div>
  );
};
