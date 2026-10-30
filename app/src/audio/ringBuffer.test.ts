import { describe, it, expect } from 'vitest';
import { AudioRingBuffer } from './ringBuffer';

describe('AudioRingBuffer', () => {
  it('initializes empty with fixed capacity', () => {
    const rb = new AudioRingBuffer(1024);
    expect(rb.capacity).toBe(1024);
    expect(rb.available()).toBe(0);
    expect(rb.canRead(512)).toBe(false);
  });

  it('pushes samples and allows reading with hop stepping', () => {
    const rb = new AudioRingBuffer(1024);
    const data1 = new Float32Array([1, 2, 3, 4]);
    rb.push(data1);
    expect(rb.available()).toBe(4);
    expect(rb.canRead(4)).toBe(true);

    // Read 3 samples, advance by 2 (hop = 2)
    const read1 = rb.read(3, 2);
    expect(read1).toEqual(new Float32Array([1, 2, 3]));
    expect(rb.available()).toBe(2); // 4 - 2 = 2 remaining: [3, 4]

    // Read 2 remaining samples
    const read2 = rb.read(2, 2);
    expect(read2).toEqual(new Float32Array([3, 4]));
    expect(rb.available()).toBe(0);
  });

  it('returns null when trying to read more samples than available', () => {
    const rb = new AudioRingBuffer(1024);
    rb.push(new Float32Array([10, 20]));
    expect(rb.read(5)).toBeNull();
  });

  it('clears all internal samples', () => {
    const rb = new AudioRingBuffer(1024);
    rb.push(new Float32Array([1, 2, 3, 4, 5]));
    rb.clear();
    expect(rb.available()).toBe(0);
    expect(rb.read(1)).toBeNull();
  });
});
