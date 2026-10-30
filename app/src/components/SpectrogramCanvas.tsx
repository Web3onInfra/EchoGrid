import React, { useEffect, useRef } from 'react';

export interface SpectrogramCanvasProps {
  fftMagnitudes: Float32Array | null;
  maxFrequencyHz?: number;
  sampleRate?: number;
  fftSize?: number;
  height?: number;
}

export const SpectrogramCanvas: React.FC<SpectrogramCanvasProps> = ({
  fftMagnitudes,
  maxFrequencyHz = 200,
  sampleRate = 16000,
  fftSize = 1024,
  height = 120,
}) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const isHiddenRef = useRef<boolean>(typeof document !== 'undefined' ? document.hidden : false);
  const reducedMotionRef = useRef<boolean>(false);

  useEffect(() => {
    const handleVisibility = () => {
      isHiddenRef.current = document.hidden;
    };
    document.addEventListener('visibilitychange', handleVisibility);

    if (typeof window !== 'undefined' && window.matchMedia) {
      const mql = window.matchMedia('(prefers-reduced-motion: reduce)');
      reducedMotionRef.current = mql.matches;
      const handleMotionChange = (e: MediaQueryListEvent) => {
        reducedMotionRef.current = e.matches;
      };
      mql.addEventListener?.('change', handleMotionChange);
      return () => {
        document.removeEventListener('visibilitychange', handleVisibility);
        mql.removeEventListener?.('change', handleMotionChange);
      };
    }

    return () => {
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  }, []);

  useEffect(() => {
    let animId: number | null = null;

    if (isHiddenRef.current || reducedMotionRef.current) {
      return;
    }

    if (!fftMagnitudes || fftMagnitudes.length === 0) {
      return;
    }

    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    animId = requestAnimationFrame(() => {
      const width = canvas.width;
      const canvasHeight = canvas.height;

      // Shift existing image left by 2 pixels (waterfall)
      ctx.drawImage(canvas, 2, 0, width - 2, canvasHeight, 0, 0, width - 2, canvasHeight);

      // Calculate maximum bin index corresponding to maxFrequencyHz (e.g. 200 Hz)
      const binWidth = sampleRate / fftSize; // 15.625 Hz
      const maxBin = Math.min(Math.ceil(maxFrequencyHz / binWidth), fftMagnitudes.length - 1);

      // Draw new column at right edge (x = width - 2, width = 2)
      const colX = width - 2;
      for (let bin = 0; bin <= maxBin; bin++) {
        const mag = fftMagnitudes[bin];
        const norm = Math.min(Math.max((Math.log10(mag + 1e-6) + 3) / 5, 0), 1);

        // Colormap: navy -> emerald -> amber -> coral
        const r = Math.round(norm * 255);
        const g = Math.round(Math.sin(norm * Math.PI) * 220);
        const b = Math.round((1 - norm) * 160);

        const y = Math.round(canvasHeight - (bin / maxBin) * canvasHeight);
        const binH = Math.max(Math.ceil(canvasHeight / maxBin), 2);

        ctx.fillStyle = `rgb(${r}, ${g}, ${b})`;
        ctx.fillRect(colX, y - binH, 2, binH);
      }
    });

    return () => {
      if (animId !== null) {
        cancelAnimationFrame(animId);
      }
    };
  }, [fftMagnitudes, maxFrequencyHz, sampleRate, fftSize]);

  return (
    <div className="space-y-1.5" data-testid="spectrogram-container">
      <div className="flex justify-between items-center text-[11px] text-slate-500 dark:text-slate-400 font-medium">
        <span>Live Spectrogram (0 - {maxFrequencyHz} Hz)</span>
        <span className="font-mono text-[10px]">Mains Spectrum</span>
      </div>
      <div className="relative rounded-xl overflow-hidden border border-slate-200 dark:border-slate-800 bg-slate-950">
        <canvas
          ref={canvasRef}
          width={320}
          height={height}
          data-testid="spectrogram-canvas"
          className="w-full h-[120px] block"
        />
        {/* Frequency guide lines overlay */}
        <div className="absolute inset-0 pointer-events-none flex flex-col justify-between p-1.5 text-[9px] font-mono text-slate-500 opacity-60">
          <span>{maxFrequencyHz} Hz</span>
          <span>120 Hz (Harmonic)</span>
          <span>60 Hz (Mains)</span>
          <span>0 Hz</span>
        </div>
      </div>
    </div>
  );
};
