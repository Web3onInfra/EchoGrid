import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { MonitorView } from './MonitorView';
import { AudioPipeline } from '../audio/pipeline';
import { PipelineFrameEvent } from '../audio/pipelineTypes';
import { DetectorState, FrameResult } from '../detector/types';

describe('MonitorView State Machine', () => {
  let fakePipeline: AudioPipeline;
  let pipelineListener: ((event: PipelineFrameEvent) => void) | null = null;

  beforeEach(() => {
    vi.clearAllMocks();
    fakePipeline = new AudioPipeline();
    pipelineListener = null;

    vi.spyOn(fakePipeline, 'start').mockImplementation(async () => {
      return {
        sampleRate: 16000,
        echoCancellation: false,
        noiseSuppression: false,
        autoGainControl: false,
      };
    });

    vi.spyOn(fakePipeline, 'stop').mockImplementation(() => {});

    vi.spyOn(fakePipeline, 'subscribe').mockImplementation((listener) => {
      pipelineListener = listener;
      return () => {
        pipelineListener = null;
      };
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    Object.defineProperty(document, 'hidden', { value: false, configurable: true });
  });

  it('renders initial IDLE state with start monitoring action', () => {
    render(<MonitorView pipelineOverride={fakePipeline} />);

    expect(screen.getByTestId('monitor-status-badge')).toHaveTextContent('IDLE');
    expect(screen.getByTestId('start-monitoring-btn')).toBeInTheDocument();
    expect(screen.getByTestId('monitor-aria-live')).toHaveTextContent(/Monitor is idle/i);
  });

  it('progresses from CALIBRATING to ARMED when audio frames arrive', async () => {
    render(<MonitorView pipelineOverride={fakePipeline} />);

    const startBtn = screen.getByTestId('start-monitoring-btn');
    await act(async () => {
      fireEvent.click(startBtn);
    });

    expect(fakePipeline.start).toHaveBeenCalled();
    expect(screen.getByTestId('monitor-status-badge')).toHaveTextContent('CALIBRATING');
    expect(screen.getByTestId('calibrating-state')).toBeInTheDocument();

    // Emit 5 calibration frames
    for (let i = 0; i < 5; i++) {
      act(() => {
        pipelineListener?.({
          frameResult: {
            timestamp_s: i * 0.032,
            state: DetectorState.ON,
            event_triggered: false,
            current_energy_db: 45.0,
            running_baseline_db: 45.0,
            drop_duration_s: 0.0,
          },
          features: {
            totalBandEnergyDb: 45.0,
            bandEnergiesDb: [45.0, 42.0, 38.0],
            fftMagnitudes: new Float32Array(513),
          },
          processingTimeMs: 1.0,
        });
      });
    }

    expect(screen.getByTestId('monitor-status-badge')).toHaveTextContent('ARMED');
    expect(screen.getByTestId('armed-state')).toBeInTheDocument();
    expect(screen.getByTestId('monitor-aria-live')).toHaveTextContent(/Monitor is ARMED/i);
  });

  it('transitions to DROP_CONFIRMED when OFF_DETECTED event arrives', async () => {
    const onDropConfirmed = vi.fn();
    render(<MonitorView pipelineOverride={fakePipeline} onDropConfirmed={onDropConfirmed} />);

    await act(async () => {
      fireEvent.click(screen.getByTestId('start-monitoring-btn'));
    });

    // Calibrate into ARMED
    for (let i = 0; i < 5; i++) {
      act(() => {
        pipelineListener?.({
          frameResult: {
            timestamp_s: i * 0.032,
            state: DetectorState.ON,
            event_triggered: false,
            current_energy_db: 45.0,
            running_baseline_db: 45.0,
            drop_duration_s: 0.0,
          },
          features: {
            totalBandEnergyDb: 45.0,
            bandEnergiesDb: [45.0, 42.0, 38.0],
            fftMagnitudes: new Float32Array(513),
          },
          processingTimeMs: 1.0,
        });
      });
    }

    // Now emit OFF_DETECTED drop confirmed frame
    const dropFrame: FrameResult = {
      timestamp_s: 5.2,
      state: DetectorState.OFF_DETECTED,
      event_triggered: true,
      current_energy_db: 25.0,
      running_baseline_db: 45.0,
      drop_duration_s: 3.2,
    };

    act(() => {
      pipelineListener?.({
        frameResult: dropFrame,
        features: {
          totalBandEnergyDb: 25.0,
          bandEnergiesDb: [20.0, 22.0, 15.0],
          fftMagnitudes: new Float32Array(513),
        },
        processingTimeMs: 1.0,
      });
    });

    expect(screen.getByTestId('monitor-status-badge')).toHaveTextContent('DROP CONFIRMED');
    expect(screen.getByTestId('drop-confirmed-state')).toBeInTheDocument();
    expect(onDropConfirmed).toHaveBeenCalledWith(dropFrame);
    expect(screen.getByTestId('monitor-aria-live')).toHaveTextContent(/Power-down drop confirmed/i);
  });

  it('cancels monitoring and returns to IDLE state', async () => {
    render(<MonitorView pipelineOverride={fakePipeline} />);

    await act(async () => {
      fireEvent.click(screen.getByTestId('start-monitoring-btn'));
    });

    expect(screen.getByTestId('monitor-status-badge')).toHaveTextContent('CALIBRATING');

    const cancelBtn = screen.getByTestId('cancel-monitoring-btn');
    act(() => {
      fireEvent.click(cancelBtn);
    });

    expect(fakePipeline.stop).toHaveBeenCalled();
    expect(screen.getByTestId('monitor-status-badge')).toHaveTextContent('IDLE');
  });

  it('displays error banner if microphone start throws', async () => {
    vi.spyOn(fakePipeline, 'start').mockRejectedValueOnce(
      new Error('Microphone permission denied'),
    );

    render(<MonitorView pipelineOverride={fakePipeline} />);

    await act(async () => {
      fireEvent.click(screen.getByTestId('start-monitoring-btn'));
    });

    expect(screen.getByTestId('monitor-error-banner')).toHaveTextContent(
      'Microphone permission denied',
    );
    expect(screen.getByTestId('monitor-status-badge')).toHaveTextContent('IDLE');
  });

  it('displays hidden tab warning when document.hidden is triggered while monitoring', async () => {
    render(<MonitorView pipelineOverride={fakePipeline} />);

    await act(async () => {
      fireEvent.click(screen.getByTestId('start-monitoring-btn'));
    });

    Object.defineProperty(document, 'hidden', { value: true, configurable: true });
    act(() => {
      document.dispatchEvent(new Event('visibilitychange'));
    });

    expect(screen.getByTestId('hidden-tab-warning')).toBeInTheDocument();
    expect(screen.getByText(/Mobile browsers suspend microphone capture/i)).toBeInTheDocument();
  });
});
