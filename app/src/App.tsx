import React, { useState, Suspense, lazy } from 'react';
import { ThemeProvider } from './context/ThemeContext';
import { OracleProvider } from './context/OracleContext';
import { WalletProvider } from './context/WalletContext';
import { Header } from './components/Header';
import { DegradedStateBanner } from './components/DegradedStateBanner';
import { InstallPromptBanner } from './components/InstallPromptBanner';
import { FirstRunNoticeBanner } from './components/FirstRunNoticeBanner';
import { AboutModal } from './components/AboutModal';
import { TabBar, TabKey } from './components/TabBar';
import { GridView } from './views/GridView';
import { WalletView } from './views/WalletView';
import { ReceiptView } from './views/ReceiptView';

// Lazy-load MonitorView to keep initial JS bundle size strictly under 200 KB gzipped
const LazyMonitorView = lazy(() =>
  import('./views/MonitorView').then((m) => ({ default: m.MonitorView })),
);

export const MonitorLoadingSkeleton: React.FC = () => (
  <div
    data-testid="monitor-skeleton"
    className="space-y-4 animate-pulse motion-reduce:animate-none"
  >
    <div className="h-24 bg-slate-200 dark:bg-slate-800 rounded-2xl" />
    <div className="h-48 bg-slate-200 dark:bg-slate-800 rounded-2xl" />
    <div className="h-32 bg-slate-200 dark:bg-slate-800 rounded-2xl" />
  </div>
);

export const AppContent: React.FC = () => {
  const [activeTab, setActiveTab] = useState<TabKey>('grid');
  const [isAboutOpen, setIsAboutOpen] = useState<boolean>(false);

  return (
    <div className="min-h-screen bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 flex flex-col transition-colors selection:bg-emerald-500 selection:text-white">
      <Header onOpenAbout={() => setIsAboutOpen(true)} />
      <FirstRunNoticeBanner onOpenAbout={() => setIsAboutOpen(true)} />
      <InstallPromptBanner />
      <DegradedStateBanner />

      <main className="flex-1 max-w-md w-full mx-auto p-4 pb-24 overflow-y-auto">
        {activeTab === 'grid' && <GridView />}
        {activeTab === 'monitor' && (
            <Suspense fallback={<MonitorLoadingSkeleton />}>
              <LazyMonitorView onNavigateToReceipt={() => setActiveTab('receipt')} />
            </Suspense>
        )}
        {activeTab === 'wallet' && <WalletView />}
        {activeTab === 'receipt' && <ReceiptView />}
      </main>

      <TabBar activeTab={activeTab} onTabChange={setActiveTab} />
      <AboutModal isOpen={isAboutOpen} onClose={() => setIsAboutOpen(false)} />
    </div>
  );
};

export const App: React.FC = () => {
  return (
    <ThemeProvider>
      <OracleProvider>
        <WalletProvider>
          <AppContent />
        </WalletProvider>
      </OracleProvider>
    </ThemeProvider>
  );
};

export default App;
