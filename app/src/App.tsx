import React from 'react';
import { env } from './env';

export const App: React.FC = () => {
  return (
    <main className="min-h-screen p-4 flex flex-col items-center justify-center">
      <h1 className="text-2xl font-bold">EchoGrid</h1>
      <p className="text-sm opacity-80 mt-2">Decentralized NegaWatt Marketplace</p>
      <div className="mt-4 p-3 bg-slate-800 rounded text-xs font-mono">Oracle: {env.oracleUrl}</div>
    </main>
  );
};

export default App;
