import React from 'react';
import { useTheme } from '../context/ThemeContext';
import { Sun, Moon, Zap, Info } from 'lucide-react';

export interface HeaderProps {
  onOpenAbout?: () => void;
}

export const Header: React.FC<HeaderProps> = ({ onOpenAbout }) => {
  const { theme, toggleTheme } = useTheme();

  return (
    <header className="safe-top flex items-center justify-between px-4 py-3 bg-white dark:bg-slate-900 border-b border-slate-200 dark:border-slate-800 transition-colors">
      <div className="flex items-center gap-2">
        <div className="w-8 h-8 rounded-lg bg-emerald-500/10 text-emerald-500 flex items-center justify-center font-bold">
          <Zap className="w-5 h-5 text-emerald-500" />
        </div>
        <div>
          <span className="font-bold text-base tracking-tight text-slate-900 dark:text-white">
            EchoGrid
          </span>
        </div>
      </div>

      <div className="flex items-center gap-2">
        <span
          data-testid="testnet-indicator"
          className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/20"
        >
          <span className="w-1.5 h-1.5 rounded-full bg-amber-500 animate-pulse motion-reduce:animate-none" />
          Testnet
        </span>

        {onOpenAbout && (
          <button
            onClick={onOpenAbout}
            aria-label="About and how it works"
            data-testid="about-button"
            className="touch-target flex items-center justify-center p-2 rounded-lg text-slate-500 hover:text-slate-900 dark:text-slate-400 dark:hover:text-white focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:outline-none"
          >
            <Info className="w-5 h-5" />
          </button>
        )}

        <button
          onClick={toggleTheme}
          aria-label="Toggle color theme"
          className="touch-target flex items-center justify-center p-2 rounded-lg text-slate-500 hover:text-slate-900 dark:text-slate-400 dark:hover:text-white focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:outline-none"
        >
          {theme === 'dark' ? <Sun className="w-5 h-5" /> : <Moon className="w-5 h-5" />}
        </button>
      </div>
    </header>
  );
};
