/**
 * Fixed-capacity circular ring buffer for streaming audio samples.
 */
export class AudioRingBuffer {
  public readonly capacity: number;
  private readonly buffer: Float32Array;
  private writeIndex: number = 0;
  private count: number = 0;

  constructor(capacity: number = 16384) {
    this.capacity = capacity;
    this.buffer = new Float32Array(capacity);
  }

  public push(samples: ArrayLike<number>): void {
    const len = samples.length;
    for (let i = 0; i < len; i++) {
      this.buffer[this.writeIndex] = samples[i];
      this.writeIndex = (this.writeIndex + 1) % this.capacity;
      if (this.count < this.capacity) {
        this.count++;
      }
    }
  }

  public available(): number {
    return this.count;
  }

  public canRead(needed: number): boolean {
    return this.count >= needed;
  }

  /**
   * Reads `count` samples from head without losing data, and advances read pointer by `advanceBy` (defaults to count).
   */
  public read(count: number, advanceBy: number = count): Float32Array | null {
    if (this.count < count) {
      return null;
    }

    const out = new Float32Array(count);
    const readStart = (this.writeIndex - this.count + this.capacity) % this.capacity;

    for (let i = 0; i < count; i++) {
      out[i] = this.buffer[(readStart + i) % this.capacity];
    }

    const actualAdvance = Math.min(advanceBy, this.count);
    this.count -= actualAdvance;

    return out;
  }

  public clear(): void {
    this.writeIndex = 0;
    this.count = 0;
  }
}
