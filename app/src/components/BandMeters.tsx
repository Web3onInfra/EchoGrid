import React from 'react';

export interface BandMetersProps {
  bandEnergiesDb: number[];
  labels?: string[];
  minDb?: number;
  maxDb?: number;
}

const DEFAULT_LABELS = ['50 Hz (Mains)', '60 Hz (Mains)', '120 Hz (Harmonic)'];

export const BandMeters: React.FC<BandMetersProps> = ({
  bandEnergiesDb,
  labels = DEFAULT_LABELS,
  minDb = -60,
  maxDb = 60,
}) => {
  return (
    <div className="space-y-3" data-testid="band-meters-container">
      <div className="flex items-center justify-between text-xs font-semibold text-slate-700 dark:text-slate-300">
        <span>Acoustic Frequency Bands</span>
        <span className="text-[10px] text-slate-500 font-mono">dB Power</span>
      </div>

      <div className="space-y-2.5">
        {labels.map((label, index) => {
          const val = bandEnergiesDb[index] ?? minDb;
          const clampedVal = Math.min(Math.max(val, minDb), maxDb);
          const percent = Math.round(((clampedVal - minDb) / (maxDb - minDb)) * 100);

          // Color scale based on signal strength
          let colorClass = 'bg-slate-400 dark:bg-slate-600';
          if (val > 35) {
            colorClass = 'bg-emerald-500';
          } else if (val > 15) {
            colorClass = 'bg-amber-500';
          }

          return (
            <div key={label} className="space-y-1">
              <div className="flex justify-between items-center text-xs">
                <span className="font-medium text-slate-800 dark:text-slate-200">{label}</span>
                <span className="font-mono text-[11px] text-slate-600 dark:text-slate-400">
                  {val.toFixed(1)} dB
                </span>
              </div>

              <div
                role="meter"
                aria-label={label}
                aria-valuenow={Math.round(val)}
                aria-valuemin={minDb}
                aria-valuemax={maxDb}
                className="w-full h-2.5 rounded-full bg-slate-100 dark:bg-slate-800 overflow-hidden"
              >
                <div
                  className={`h-full rounded-full transition-all duration-150 ease-out ${colorClass}`}
                  style={{ width: `${percent}%` }}
                />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
