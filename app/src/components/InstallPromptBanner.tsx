import React from 'react';
import { usePwaInstall } from '../pwa/usePwaInstall';
import { Download, X } from 'lucide-react';

export const InstallPromptBanner: React.FC = () => {
  const { isInstallable, promptInstall, dismissPrompt } = usePwaInstall();

  if (!isInstallable) {
    return null;
  }

  return (
    <div
      data-testid="pwa-install-banner"
      className="bg-emerald-600 text-white px-4 py-2 text-xs flex items-center justify-between shadow-md"
    >
      <div className="flex items-center gap-2">
        <Download className="w-4 h-4 shrink-0" />
        <span className="font-medium">
          Install EchoGrid for reliable offline &amp; background monitoring
        </span>
      </div>
      <div className="flex items-center gap-2">
        <button
          onClick={promptInstall}
          data-testid="pwa-install-btn"
          className="touch-target px-3 py-1 bg-white text-emerald-800 font-bold rounded-lg hover:bg-emerald-50 active:scale-95 transition"
        >
          Install
        </button>
        <button
          onClick={dismissPrompt}
          aria-label="Dismiss installation prompt"
          data-testid="pwa-dismiss-btn"
          className="touch-target p-1 hover:bg-emerald-700 rounded-lg text-emerald-100"
        >
          <X className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
};
