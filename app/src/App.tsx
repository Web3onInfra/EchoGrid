import React, { useState } from 'react';
import { ThemeProvider } from './context/ThemeContext';
import { Header } from './components/Header';
import { TabBar, TabKey } from './components/TabBar';
import { GridView } from './views/GridView';
import { MonitorView } from './views/MonitorView';
import { WalletView } from './views/WalletView';
import { ReceiptView } from './views/ReceiptView';

export const AppContent: React.FC = () => {
  const [activeTab, setActiveTab] = useState<TabKey>('grid');

  return (
    <div className="min-h-screen bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 flex flex-col transition-colors selection:bg-emerald-500 selection:text-white">
      <Header />

      <main className="flex-1 max-w-md w-full mx-auto p-4 pb-24 overflow-y-auto">
        {activeTab === 'grid' && <GridView />}
        {activeTab === 'monitor' && <MonitorView />}
        {activeTab === 'wallet' && <WalletView />}
        {activeTab === 'receipt' && <ReceiptView />}
      </main>

      <TabBar activeTab={activeTab} onTabChange={setActiveTab} />
    </div>
  );
};

export const App: React.FC = () => {
  return (
    <ThemeProvider>
      <AppContent />
    </ThemeProvider>
  );
};

export default App;
