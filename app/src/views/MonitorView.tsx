import React, { useState, useEffect, useRef, useCallback } from 'react';
import { AudioPipeline } from '../audio/pipeline';
import { HumSimulator } from '../audio/simulator';
import { SpectrogramCanvas } from '../components/SpectrogramCanvas';
import { BandMeters } from '../components/BandMeters';
import { DetectorState, FrameResult } from '../detector/types';

// Lightweight inline SVG icons
const ActivityIcon: React.FC<{ className?: string }> = ({ className = 'w-5 h-5' }) => (
  <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
    <path strokeLinecap="round" strokeLinejoin="round" d="M22 12h-4l-3 9L9 3l-3 9H2" />
  </svg>
);

const MicIcon: React.FC<{ className?: string }> = ({ className = 'w-4 h-4' }) => (
  <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      d="M12 1a3 3 0 00-3 3v8a3 3 0 006 0V4a3 3 0 00-3-3z"
    />
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      d="M19 10v2a7 7 0 01-14 0v-2M12 19v4m-4 0h8"
    />
  </svg>
);

const AlertCircleIcon: React.FC<{ className?: string }> = ({ className = 'w-4 h-4' }) => (
  <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
    <circle cx="12" cy="12" r="10" />
    <line x1="12" y1="8" x2="12" y2="12" />
    <line x1="12" y1="16" x2="12.01" y2="16" />
  </svg>
);

const CheckCircleIcon: React.FC<{ className?: string }> = ({ className = 'w-5 h-5' }) => (
  <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
    />
  </svg>
);

const SquareIcon: React.FC<{ className?: string }> = ({ className = 'w-3.5 h-3.5' }) => (
  <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
    <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
  </svg>
);

const SparklesIcon: React.FC<{ className?: string }> = ({ className = 'w-4 h-4' }) => (
  <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z"
    />
  </svg>
);

const ShieldCheckIcon: React.FC<{ className?: string }> = ({ className = 'w-4 h-4' }) => (
  <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
    />
  </svg>
);

const EyeOffIcon: React.FC<{ className?: string }> = ({ className = 'w-4 h-4' }) => (
  <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l18 18"
    />
  </svg>
);

const RadioIcon: React.FC<{ className?: string }> = ({ className = 'w-4 h-4' }) => (
  <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
    <circle cx="12" cy="12" r="2" />
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      d="M16.24 7.76a6 6 0 010 8.49m-8.48-.01a6 6 0 010-8.49m11.31-2.82a10 10 0 010 14.14m-14.14 0a10 10 0 010-14.14"
    />
  </svg>
);

const PowerOffIcon: React.FC<{ className?: string }> = ({ className = 'w-4 h-4' }) => (
  <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
    <path strokeLinecap="round" strokeLinejoin="round" d="M18.36 6.64a9 9 0 11-12.73 0M12 2v10" />
  </svg>
);

export type MonitorStatus = 'IDLE' | 'PERMISSION' | 'CALIBRATING' | 'ARMED' | 'DROP_CONFIRMED';

export interface MonitorViewProps {
  pipelineOverride?: AudioPipeline;
  onDropConfirmed?: (frame: FrameResult, simulated: boolean) => void;
}

export const MonitorView: React.FC<MonitorViewProps> = ({ pipelineOverride, onDropConfirmed }) => {
  const [status, setStatus] = useState<MonitorStatus>('IDLE');
  const [isSimulatorMode, setIsSimulatorMode] = useState<boolean>(false);
  const [simApplianceStopped, setSimApplianceStopped] = useState<boolean>(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [statusAnnouncement, setStatusAnnouncement] = useState<string>(
    'Monitor is idle. Tap Start Monitoring to calibrate with your running appliance.',
  );
  const [isTabHidden, setIsTabHidden] = useState<boolean>(
    typeof document !== 'undefined' ? document.hidden : false,
  );

  const [bandEnergies, setBandEnergies] = useState<number[]>([-60, -60, -60]);
  const [fftMagnitudes, setFftMagnitudes] = useState<Float32Array | null>(null);
  const [confirmedFrame, setConfirmedFrame] = useState<FrameResult | null>(null);

  const pipelineRef = useRef<AudioPipeline>(pipelineOverride || new AudioPipeline());
  const simulatorRef = useRef<HumSimulator>(new HumSimulator(16000));
  const wakeLockRef = useRef<WakeLockSentinel | null>(null);
  const calibrationFramesRef = useRef<number>(0);

  // Monitor visibilitychange to warn user when mobile browser suspends background audio
  useEffect(() => {
    const handleVisibility = () => {
      setIsTabHidden(document.hidden);
    };
    document.addEventListener('visibilitychange', handleVisibility);
    return () => {
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  }, []);

  const requestWakeLock = async () => {
    try {
      if ('wakeLock' in navigator && navigator.wakeLock) {
        wakeLockRef.current = await navigator.wakeLock.request('screen');
      }
    } catch {
      // WakeLock not supported or denied
    }
  };

  const releaseWakeLock = () => {
    if (wakeLockRef.current) {
      try {
        wakeLockRef.current.release();
      } catch {
        // ignore
      }
      wakeLockRef.current = null;
    }
  };

  const stopMonitoring = useCallback(() => {
    pipelineRef.current.stop();
    simulatorRef.current.stop();
    releaseWakeLock();
    setStatus('IDLE');
    setSimApplianceStopped(false);
    setFftMagnitudes(null);
    setBandEnergies([-60, -60, -60]);
    calibrationFramesRef.current = 0;
    setStatusAnnouncement('Monitoring stopped. Idle.');
  }, []);

  const startMonitoring = async () => {
    setErrorMessage(null);
    setSimApplianceStopped(false);

    if (isSimulatorMode) {
      // Simulator mode: feed synthetic audio without requesting microphone permission
      setStatus('CALIBRATING');
      setStatusAnnouncement('Simulator running: Calibrating baseline 60+120 Hz hum...');
      calibrationFramesRef.current = 0;
      simulatorRef.current.start((chunk) => {
        pipelineRef.current.feedSamples(chunk);
      });
      return;
    }

    setStatus('PERMISSION');
    setStatusAnnouncement('Requesting microphone permission...');

    try {
      await pipelineRef.current.start();
      await requestWakeLock();
      calibrationFramesRef.current = 0;
      setStatus('CALIBRATING');
      setStatusAnnouncement('Calibrating baseline hum with your running appliance...');
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Microphone initialization failed';
      setErrorMessage(msg);
      setStatus('IDLE');
      setStatusAnnouncement(`Error: ${msg}`);
    }
  };

  const handleSimulateStopAppliance = () => {
    setSimApplianceStopped(true);
    simulatorRef.current.stopAppliance();
    setStatusAnnouncement(
      'Appliance stopped! Waiting for sustained power-down drop verification...',
    );
  };

  // Subscribe to pipeline audio frame events
  useEffect(() => {
    const pipeline = pipelineRef.current;
    const unsubscribe = pipeline.subscribe((event) => {
      setBandEnergies(event.features.bandEnergiesDb);
      setFftMagnitudes(event.features.fftMagnitudes);

      if (status === 'CALIBRATING') {
        calibrationFramesRef.current += 1;
        if (calibrationFramesRef.current >= 5) {
          setStatus('ARMED');
          setStatusAnnouncement(
            isSimulatorMode
              ? 'Simulator ARMED. Tap Simulate Power-Down to trigger detector.'
              : 'Appliance hum calibrated. Monitor is ARMED. Waiting for power-down.',
          );
        }
      } else if (status === 'ARMED') {
        if (event.frameResult.state === DetectorState.OFF_DETECTED) {
          setStatus('DROP_CONFIRMED');
          setConfirmedFrame(event.frameResult);
          setStatusAnnouncement(
            'Power-down drop confirmed! Appliance shut down detected. Ready to claim payout.',
          );
          onDropConfirmed?.(event.frameResult, isSimulatorMode);
        }
      }
    });

    return () => {
      unsubscribe();
    };
  }, [status, isSimulatorMode, onDropConfirmed]);

  // Clean up on component unmount
  useEffect(() => {
    const pipeline = pipelineRef.current;
    const simulator = simulatorRef.current;
    return () => {
      pipeline.stop();
      simulator.stop();
      releaseWakeLock();
    };
  }, []);

  return (
    <div className="space-y-4" data-testid="monitor-view-container">
      {/* Aria-Live Region for Screen Readers */}
      <div className="sr-only" aria-live="polite" data-testid="monitor-aria-live">
        {statusAnnouncement}
      </div>

      {/* Header Card with Simulator Switcher */}
      <div className="p-4 rounded-2xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 shadow-sm transition-colors space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="text-xl font-bold text-slate-900 dark:text-white flex items-center gap-2">
            <ActivityIcon className="w-5 h-5 text-emerald-500" />
            Acoustic Monitor
          </h2>
          <div className="flex items-center gap-2">
            {isSimulatorMode && (
              <span
                data-testid="simulated-mode-pill"
                className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/20"
              >
                Simulated
              </span>
            )}
            <span
              data-testid="monitor-status-badge"
              className={`text-[10px] font-bold uppercase tracking-wider px-2.5 py-0.5 rounded-full border ${
                status === 'ARMED'
                  ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20 animate-pulse'
                  : status === 'CALIBRATING'
                    ? 'bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-500/20'
                    : status === 'DROP_CONFIRMED'
                      ? 'bg-indigo-500/10 text-indigo-600 dark:text-indigo-400 border-indigo-500/20'
                      : 'bg-slate-100 dark:bg-slate-800 text-slate-500 dark:text-slate-400 border-slate-200 dark:border-slate-700'
              }`}
            >
              {status.replace('_', ' ')}
            </span>
          </div>
        </div>

        <p className="text-xs text-slate-600 dark:text-slate-400">
          Detects appliance power-down transitions via 50/60/120 Hz acoustic hum drops.
        </p>

        {/* Mode Toggle Button */}
        {status === 'IDLE' && (
          <div className="pt-1 flex items-center justify-between border-t border-slate-100 dark:border-slate-800">
            <span className="text-[11px] text-slate-500 flex items-center gap-1.5">
              <RadioIcon className="w-3.5 h-3.5 text-amber-500" />
              <span>Input Mode</span>
            </span>
            <button
              onClick={() => setIsSimulatorMode(!isSimulatorMode)}
              data-testid="toggle-simulator-mode-btn"
              className="touch-target text-xs font-semibold text-emerald-600 dark:text-emerald-400 hover:underline flex items-center gap-1"
            >
              <span>{isSimulatorMode ? 'Switch to Real Mic' : 'Switch to Simulator Mode'}</span>
            </button>
          </div>
        )}
      </div>

      {/* Simulator Mode Policy Notice */}
      {isSimulatorMode && (
        <div
          data-testid="simulator-policy-banner"
          className="p-3 rounded-xl bg-amber-500/10 border border-amber-500/20 text-xs text-amber-800 dark:text-amber-300 flex items-start gap-2"
        >
          <RadioIcon className="w-4 h-4 text-amber-600 shrink-0 mt-0.5" />
          <div className="space-y-0.5">
            <span className="font-bold block">Simulator Mode Active (Simulated)</span>
            <p className="text-[11px] leading-relaxed">
              Uses an oscillator-synthesized 60+120 Hz acoustic profile to test the real detector.
              Per Phase 2 policy, simulated claims verify against the oracle but yield zero on-chain
              payout.
            </p>
          </div>
        </div>
      )}

      {/* Hidden Tab Warning (Foreground-only audio notice) */}
      {isTabHidden && status !== 'IDLE' && (
        <div
          data-testid="hidden-tab-warning"
          className="p-3 rounded-xl bg-amber-500/10 border border-amber-500/20 text-xs text-amber-800 dark:text-amber-300 flex items-start gap-2"
        >
          <EyeOffIcon className="w-4 h-4 text-amber-600 shrink-0 mt-0.5" />
          <div className="space-y-0.5">
            <span className="font-bold block">Tab Hidden — Audio Paused</span>
            <p className="text-[11px] leading-relaxed">
              Mobile browsers suspend microphone capture in background tabs. Please return to this
              tab to continue monitoring.
            </p>
          </div>
        </div>
      )}

      {/* Error Banner */}
      {errorMessage && (
        <div
          data-testid="monitor-error-banner"
          className="p-3 rounded-xl bg-rose-500/10 border border-rose-500/20 text-xs text-rose-700 dark:text-rose-300 flex items-start gap-2"
        >
          <AlertCircleIcon className="w-4 h-4 text-rose-600 shrink-0 mt-0.5" />
          <span>{errorMessage}</span>
        </div>
      )}

      {/* Spectrogram & Meters Visualizer */}
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-4 shadow-sm space-y-4 transition-colors">
        <SpectrogramCanvas fftMagnitudes={fftMagnitudes} />
        <BandMeters bandEnergiesDb={bandEnergies} />
      </div>

      {/* State Actions & Guidance Card */}
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-4 shadow-sm space-y-3 transition-colors">
        {status === 'IDLE' && (
          <div className="space-y-3 text-center py-2">
            <p className="text-xs text-slate-600 dark:text-slate-400">
              {isSimulatorMode
                ? 'Synthesizes an acoustic refrigerator/compressor hum directly into the detector pipeline.'
                : 'Ensure your appliance is actively humming, then tap below to begin calibration.'}
            </p>
            <button
              onClick={startMonitoring}
              data-testid="start-monitoring-btn"
              className="touch-target w-full py-3 px-4 rounded-xl bg-emerald-600 hover:bg-emerald-500 active:scale-[0.98] text-white font-semibold text-xs transition flex items-center justify-center gap-2 shadow-sm"
            >
              {isSimulatorMode ? (
                <RadioIcon className="w-4 h-4" />
              ) : (
                <MicIcon className="w-4 h-4" />
              )}
              <span>{isSimulatorMode ? 'Start Simulator Stream' : 'Start Monitoring'}</span>
            </button>
            <span className="text-[10px] text-slate-500 block">
              Audio is strictly analyzed locally on your device. Zero audio data is ever
              transmitted.
            </span>
          </div>
        )}

        {status === 'CALIBRATING' && (
          <div className="space-y-3 text-center py-2" data-testid="calibrating-state">
            <div className="flex items-center justify-center gap-2 text-xs font-bold text-amber-600 dark:text-amber-400">
              <SparklesIcon className="w-4 h-4 animate-spin" />
              <span>Calibrating Baseline Hum...</span>
            </div>
            <p className="text-[11px] text-slate-500 dark:text-slate-400">
              Measuring hum baseline. Keep device steady.
            </p>
            <button
              onClick={stopMonitoring}
              data-testid="cancel-monitoring-btn"
              className="touch-target w-full py-2.5 px-4 rounded-xl border border-slate-200 dark:border-slate-700 text-xs font-semibold text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-800 flex items-center justify-center gap-1.5"
            >
              <SquareIcon className="w-3.5 h-3.5" />
              <span>Cancel</span>
            </button>
          </div>
        )}

        {status === 'ARMED' && (
          <div className="space-y-3" data-testid="armed-state">
            <div className="p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/20 text-xs text-emerald-800 dark:text-emerald-300 flex items-center gap-2">
              <ShieldCheckIcon className="w-4 h-4 text-emerald-600 shrink-0" />
              <span className="font-semibold">
                {isSimulatorMode
                  ? 'Simulator Armed — Tap below to simulate turning off the appliance!'
                  : 'Monitor Armed — Turn off appliance to trigger payout!'}
              </span>
            </div>

            {isSimulatorMode && !simApplianceStopped && (
              <button
                onClick={handleSimulateStopAppliance}
                data-testid="simulate-stop-appliance-btn"
                className="touch-target w-full py-3 px-4 rounded-xl bg-rose-600 hover:bg-rose-500 active:scale-[0.98] text-white font-bold text-xs transition flex items-center justify-center gap-2 shadow-sm"
              >
                <PowerOffIcon className="w-4 h-4" />
                <span>Simulate Power-Down (Stop Appliance)</span>
              </button>
            )}

            {isSimulatorMode && simApplianceStopped && (
              <div
                data-testid="sim-drop-verifying"
                className="p-3 rounded-xl bg-indigo-500/10 border border-indigo-500/20 text-xs text-indigo-800 dark:text-indigo-300 text-center font-medium animate-pulse"
              >
                Appliance stopped! Verifying &gt;3.0s sustained drop...
              </div>
            )}

            <button
              onClick={stopMonitoring}
              data-testid="cancel-monitoring-btn"
              className="touch-target w-full py-2.5 px-4 rounded-xl border border-rose-200 dark:border-rose-900/50 text-xs font-semibold text-rose-600 dark:text-rose-400 hover:bg-rose-50 dark:hover:bg-rose-950/20 flex items-center justify-center gap-1.5"
            >
              <SquareIcon className="w-3.5 h-3.5" />
              <span>Disarm / Cancel</span>
            </button>
          </div>
        )}

        {status === 'DROP_CONFIRMED' && (
          <div className="space-y-3 text-center py-2" data-testid="drop-confirmed-state">
            <div className="flex items-center justify-center gap-2 text-xs font-bold text-emerald-600 dark:text-emerald-400">
              <CheckCircleIcon className="w-5 h-5 text-emerald-500" />
              <span>Power-Down Confirmed!</span>
            </div>
            <p className="text-xs text-slate-600 dark:text-slate-400">
              A sustained &gt;6 dB acoustic drop was verified locally at{' '}
              {confirmedFrame?.timestamp_s.toFixed(1)}s.
              {isSimulatorMode && ' (Simulated Claim — No real XLM payout)'}
            </p>
            <button
              onClick={stopMonitoring}
              data-testid="reset-monitor-btn"
              className="touch-target w-full py-2.5 px-4 rounded-xl bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 text-xs font-semibold text-slate-700 dark:text-slate-300 transition"
            >
              Monitor Again
            </button>
          </div>
        )}
      </div>
    </div>
  );
};
