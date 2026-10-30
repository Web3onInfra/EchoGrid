import { describe, it, expect } from 'vitest';
import goldenVectors from '../../../research/hum_detection/golden_vectors.json';
import { StreamingPowerDownDetector, defaultDetectorParams } from './detector';
import { DetectorParams } from './types';

describe('StreamingPowerDownDetector', () => {
  it('loads detector params matching detector_params.json', () => {
    expect(defaultDetectorParams.sample_rate).toBe(16000);
    expect(defaultDetectorParams.frame_size).toBe(1024);
    expect(defaultDetectorParams.hop_size).toBe(512);
    expect(defaultDetectorParams.drop_threshold_db).toBe(6.0);
    expect(defaultDetectorParams.hysteresis_db).toBe(2.0);
    expect(defaultDetectorParams.min_sustained_drop_s).toBe(3.0);
    expect(defaultDetectorParams.bands).toEqual([
      [45.0, 55.0],
      [55.0, 65.0],
      [115.0, 125.0],
    ]);
  });

  describe('Golden vector parity against Phase 0 Python implementation', () => {
    const params = goldenVectors.params as unknown as DetectorParams;

    for (const scenario of goldenVectors.scenarios) {
      it(`matches golden vector scenario: ${scenario.scenario_name}`, () => {
        const detector = new StreamingPowerDownDetector(params);

        for (let i = 0; i < scenario.frames.length; i++) {
          const frame = scenario.frames[i];
          const result = detector.processEnergyFrame(frame.energy_db);

          expect(result.state).toBe(frame.expected_state);
          expect(result.event_triggered).toBe(frame.expected_triggered);
        }
      });
    }
  });

  it('resets state properly', () => {
    const detector = new StreamingPowerDownDetector();
    detector.processEnergyFrame(50);
    detector.processEnergyFrame(30);
    expect(detector.framesProcessed).toBe(2);

    detector.reset();
    expect(detector.framesProcessed).toBe(0);
    expect(detector.runningBaselineDb).toBeNull();
    expect(detector.sustainedDropS).toBe(0);
    expect(detector.hasTriggered).toBe(false);
  });
});
