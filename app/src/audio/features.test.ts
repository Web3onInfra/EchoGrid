import { describe, it, expect } from 'vitest';
import fs from 'fs';
import path from 'path';
import { AudioFeatureExtractor, cooleyTukeyFft } from './features';

const fixturesPath = path.resolve(__dirname, '../../../research/fixtures/feature_parity.json');
const featureFixtures = JSON.parse(fs.readFileSync(fixturesPath, 'utf-8'));

describe('AudioFeatureExtractor', () => {
  it('correctly computes simple Cooley-Tukey FFT of impulse', () => {
    const real = new Float64Array([1, 0, 0, 0]);
    const imag = new Float64Array([0, 0, 0, 0]);
    cooleyTukeyFft(real, imag);
    // FFT of unit impulse is 1 across all bins
    for (let i = 0; i < 4; i++) {
      expect(real[i]).toBeCloseTo(1.0, 5);
      expect(imag[i]).toBeCloseTo(0.0, 5);
    }
  });

  describe('Feature Parity against Phase 0 Python research fixtures', () => {
    const extractor = new AudioFeatureExtractor();
    // Documented tolerance: 0.01 dB accounting for Float32Array precision and trigonometric math
    const TOLERANCE_DB = 0.01;

    for (const testCase of featureFixtures.single_frame_cases) {
      it(`matches single frame case: ${testCase.name}`, () => {
        const features = extractor.extractFrameFeatures(testCase.audio);

        expect(features.totalBandEnergyDb).toBeCloseTo(
          testCase.expected_total_energy_db,
          TOLERANCE_DB < 0.1 ? 1 : 0,
        );
        const diff = Math.abs(features.totalBandEnergyDb - testCase.expected_total_energy_db);
        expect(diff).toBeLessThanOrEqual(TOLERANCE_DB);
      });
    }

    it('matches multi-frame power-down streaming sequence', () => {
      const streamData = featureFixtures.streaming_sequence;
      const features = extractor.extractStreamFeatures(streamData.audio);

      expect(features.length).toBe(streamData.expected_frame_energies_db.length);

      for (let i = 0; i < features.length; i++) {
        const expected = streamData.expected_frame_energies_db[i];
        const actual = features[i].totalBandEnergyDb;
        const diff = Math.abs(actual - expected);
        expect(diff).toBeLessThanOrEqual(TOLERANCE_DB);
      }
    });
  });

  it('provides band-specific energy components and positive-frequency FFT magnitudes', () => {
    const extractor = new AudioFeatureExtractor();
    const frame = new Float32Array(1024);
    const features = extractor.extractFrameFeatures(frame);

    expect(features.bandEnergiesDb.length).toBe(3); // 50Hz, 60Hz, 120Hz
    expect(features.fftMagnitudes.length).toBe(513); // n/2 + 1 for 1024-point FFT
  });
});
