import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { HumSimulator } from './simulator';
import { AudioPipeline } from './pipeline';
import { DetectorState } from '../detector/types';

describe('HumSimulator & Real Detector Integration', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('generates non-zero acoustic hum when appliance is running', () => {
    const sim = new HumSimulator(16000);
    const chunks: Float32Array[] = [];

    sim.start((chunk) => chunks.push(chunk), 32);

    vi.advanceTimersByTime(32);
    expect(chunks.length).toBe(1);
    expect(chunks[0].length).toBe(512);

    // Verify non-zero samples
    const hasSignal = chunks[0].some((s) => Math.abs(s) > 0.05);
    expect(hasSignal).toBe(true);

    sim.stop();
  });

  it('cuts signal to zero when stopAppliance is called', () => {
    const sim = new HumSimulator(16000);
    const chunks: Float32Array[] = [];

    sim.start((chunk) => chunks.push(chunk), 32);
    vi.advanceTimersByTime(32);

    sim.stopAppliance();
    vi.advanceTimersByTime(32);

    expect(chunks.length).toBe(2);
    // Second chunk is silent (power down)
    const isSilent = chunks[1].every((s) => s === 0);
    expect(isSilent).toBe(true);

    sim.stop();
  });

  it('fires real detector OFF_DETECTED when synthetic hum powers down', () => {
    const pipeline = new AudioPipeline();
    const sim = new HumSimulator(16000);
    const states: DetectorState[] = [];

    pipeline.subscribe((event) => {
      states.push(event.frameResult.state);
    });

    sim.start((chunk) => {
      pipeline.feedSamples(chunk);
    }, 32);

    // 1. Run appliance hum for 2 seconds (62 frames)
    for (let i = 0; i < 62; i++) {
      vi.advanceTimersByTime(32);
    }

    expect(states[states.length - 1]).toBe(DetectorState.ON);

    // 2. Shut off appliance
    sim.stopAppliance();

    // 3. Advance by 3.5 seconds (110 frames > 3.0s min_sustained_drop)
    for (let i = 0; i < 110; i++) {
      vi.advanceTimersByTime(32);
    }

    // Detector must have fired OFF_DETECTED
    expect(states).toContain(DetectorState.OFF_DETECTED);
    expect(states[states.length - 1]).toBe(DetectorState.OFF_DETECTED);

    sim.stop();
    pipeline.stop();
  });
});
