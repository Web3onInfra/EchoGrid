import { defaultDetectorParams } from '../detector/detector';
import { DetectorParams } from '../detector/types';

export interface FrameFeatures {
  totalBandEnergyDb: number;
  bandEnergiesDb: number[];
  fftMagnitudes: Float32Array;
}

/**
 * Radix-2 Cooley-Tukey FFT implementation in pure TypeScript.
 * In-place computation on real and imaginary Float64Array buffers.
 */
export function cooleyTukeyFft(real: Float64Array, imag: Float64Array): void {
  const n = real.length;
  // Bit-reversal permutation
  let j = 0;
  for (let i = 0; i < n - 1; i++) {
    if (i < j) {
      const tr = real[i];
      real[i] = real[j];
      real[j] = tr;
      const ti = imag[i];
      imag[i] = imag[j];
      imag[j] = ti;
    }
    let k = n >> 1;
    while (k <= j) {
      j -= k;
      k >>= 1;
    }
    j += k;
  }

  // Butterfly stages
  for (let len = 2; len <= n; len <<= 1) {
    const half = len >> 1;
    const angle = (-2.0 * Math.PI) / len;
    const wstepR = Math.cos(angle);
    const wstepI = Math.sin(angle);

    for (let i = 0; i < n; i += len) {
      let wr = 1.0;
      let wi = 0.0;
      for (let k = 0; k < half; k++) {
        const uR = real[i + k];
        const uI = imag[i + k];
        const vR = real[i + k + half] * wr - imag[i + k + half] * wi;
        const vI = real[i + k + half] * wi + imag[i + k + half] * wr;

        real[i + k] = uR + vR;
        imag[i + k] = uI + vI;
        real[i + k + half] = uR - vR;
        imag[i + k + half] = uI - vI;

        const nextWr = wr * wstepR - wi * wstepI;
        wi = wr * wstepI + wi * wstepR;
        wr = nextWr;
      }
    }
  }
}

/**
 * Streaming Audio Feature Extractor for appliance hum detection.
 * Performs Hann-windowed STFT and extracts monitored band energies.
 */
export class AudioFeatureExtractor {
  public readonly params: DetectorParams;
  public readonly window: Float64Array;
  public readonly bandBins: number[][];
  private readonly realBuffer: Float64Array;
  private readonly imagBuffer: Float64Array;

  constructor(params?: Partial<DetectorParams>) {
    this.params = { ...defaultDetectorParams, ...params };
    const n = this.params.frame_size;
    const sr = this.params.sample_rate;

    // Precompute Hann window matching NumPy: 0.5 - 0.5 * cos(2 * pi * i / (n - 1))
    this.window = new Float64Array(n);
    for (let i = 0; i < n; i++) {
      this.window[i] = 0.5 - 0.5 * Math.cos((2.0 * Math.PI * i) / (n - 1));
    }

    // Precompute FFT bin indices for each monitored frequency band
    this.bandBins = this.params.bands.map(([fLow, fHigh]) => {
      const bins: number[] = [];
      const numBins = n / 2 + 1;
      for (let k = 0; k < numBins; k++) {
        const f = (k * sr) / n;
        if (f >= fLow && f <= fHigh) {
          bins.push(k);
        }
      }
      return bins;
    });

    this.realBuffer = new Float64Array(n);
    this.imagBuffer = new Float64Array(n);
  }

  /**
   * Process a single audio frame of length frame_size.
   */
  public extractFrameFeatures(frame: ArrayLike<number>): FrameFeatures {
    const n = this.params.frame_size;
    const frameLen = frame.length;

    // Window and copy into real buffer, zero-padding if shorter than frame_size
    for (let i = 0; i < n; i++) {
      const sample = i < frameLen ? frame[i] : 0.0;
      this.realBuffer[i] = sample * this.window[i];
      this.imagBuffer[i] = 0.0;
    }

    cooleyTukeyFft(this.realBuffer, this.imagBuffer);

    const numPositiveBins = n / 2 + 1;
    const fftMagnitudes = new Float32Array(numPositiveBins);
    for (let k = 0; k < numPositiveBins; k++) {
      const magSq =
        this.realBuffer[k] * this.realBuffer[k] + this.imagBuffer[k] * this.imagBuffer[k];
      fftMagnitudes[k] = Math.sqrt(magSq);
    }

    // Compute linear power per band
    let totalPower = 0.0;
    const bandEnergiesDb: number[] = [];

    for (const bins of this.bandBins) {
      let bandPower = 0.0;
      for (const k of bins) {
        const power =
          this.realBuffer[k] * this.realBuffer[k] + this.imagBuffer[k] * this.imagBuffer[k];
        bandPower += power;
      }
      totalPower += bandPower;
      bandEnergiesDb.push(10.0 * Math.log10(Math.max(bandPower, 1e-12)));
    }

    const totalBandEnergyDb = 10.0 * Math.log10(Math.max(totalPower, 1e-12));

    return {
      totalBandEnergyDb,
      bandEnergiesDb,
      fftMagnitudes,
    };
  }

  /**
   * Extract features across an audio buffer with hop_size stepping.
   */
  public extractStreamFeatures(audio: ArrayLike<number>): FrameFeatures[] {
    const results: FrameFeatures[] = [];
    const frameSize = this.params.frame_size;
    const hopSize = this.params.hop_size;
    const len = audio.length;

    for (let start = 0; start <= len - frameSize; start += hopSize) {
      const frame = Array.from(audio).slice(start, start + frameSize);
      results.push(this.extractFrameFeatures(frame));
    }

    return results;
  }
}
