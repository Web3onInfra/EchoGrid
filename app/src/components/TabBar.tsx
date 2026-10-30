import React from 'react';
import { Activity, Mic, Wallet, ReceiptText } from 'lucide-react';

export type TabKey = 'grid' | 'monitor' | 'wallet' | 'receipt';

interface TabBarProps {
  activeTab: TabKey;
  onTabChange: (tab: TabKey) => void;
}

const TABS = [
  { key: 'grid' as const, label: 'Grid', icon: Activity },
  { key: 'monitor' as const, label: 'Monitor', icon: Mic },
  { key: 'wallet' as const, label: 'Wallet', icon: Wallet },
  { key: 'receipt' as const, label: 'Receipt', icon: ReceiptText },
];

export const TabBar: React.FC<TabBarProps> = ({ activeTab, onTabChange }) => {
  return (
    <nav
      aria-label="Bottom Navigation"
      className="safe-bottom fixed bottom-0 left-0 right-0 bg-white/95 dark:bg-slate-900/95 backdrop-blur border-t border-slate-200 dark:border-slate-800 z-50 transition-colors"
    >
      <div className="max-w-md mx-auto flex items-center justify-around px-2 py-1">
        {TABS.map(({ key, label, icon: Icon }) => {
          const isActive = activeTab === key;
          return (
            <button
              key={key}
              role="tab"
              aria-selected={isActive}
              aria-label={label}
              onClick={() => onTabChange(key)}
              className={`touch-target flex flex-col items-center justify-center flex-1 py-1 px-2 rounded-xl transition-all ${
                isActive
                  ? 'text-emerald-600 dark:text-emerald-400 font-semibold'
                  : 'text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200'
              }`}
            >
              <Icon className={`w-5 h-5 ${isActive ? 'scale-110' : ''}`} />
              <span className="text-[11px] mt-0.5">{label}</span>
            </button>
          );
        })}
      </div>
    </nav>
  );
};
