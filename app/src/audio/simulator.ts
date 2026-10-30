/**
 * Acoustic appliance hum simulator.
 * Generates synthetic mains hum (60 Hz + 120 Hz harmonic) fed directly into the audio pipeline
 * allowing end-to-end verification without requiring physical appliance operation.
 */
export class HumSimulator {
  private isGenerating: boolean = false;
  private applianceRunning: boolean = true;
  private sampleRate: number;
  private timerId: ReturnType<typeof setInterval> | null = null;
  private sampleOffset: number = 0;

  constructor(sampleRate: number = 16000) {
    this.sampleRate = sampleRate;
  }

  public isActive(): boolean {
    return this.isGenerating;
  }

  public isRunning(): boolean {
    return this.applianceRunning;
  }

  public start(onChunk: (samples: Float32Array) => void, intervalMs: number = 32): void {
    if (this.isGenerating) return;
    this.isGenerating = true;
    this.applianceRunning = true;
    this.sampleOffset = 0;

    const chunkSize = Math.round((this.sampleRate * intervalMs) / 1000); // 512 samples for 32ms

    this.timerId = setInterval(() => {
      const chunk = new Float32Array(chunkSize);
      for (let i = 0; i < chunkSize; i++) {
        const t = (this.sampleOffset + i) / this.sampleRate;
        if (this.applianceRunning) {
          // 60 Hz mains hum fundamental + 120 Hz second harmonic
          const fundamental = 0.35 * Math.sin(2.0 * Math.PI * 60.0 * t);
          const harmonic = 0.15 * Math.sin(2.0 * Math.PI * 120.0 * t);
          chunk[i] = fundamental + harmonic;
        } else {
          // Power down: zero hum signal (silent/noise floor)
          chunk[i] = 0.0;
        }
      }
      this.sampleOffset += chunkSize;
      onChunk(chunk);
    }, intervalMs);
  }

  /**
   * Simulates appliance shutoff. Keeps audio streaming at zero hum power so
   * the real detector observes a sustained acoustic drop.
   */
  public stopAppliance(): void {
    this.applianceRunning = false;
  }

  public stop(): void {
    if (this.timerId !== null) {
      clearInterval(this.timerId);
      this.timerId = null;
    }
    this.isGenerating = false;
    this.applianceRunning = true;
    this.sampleOffset = 0;
  }
}
