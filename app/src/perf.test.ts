import { describe, it, expect } from 'vitest';
import fs from 'fs';
import path from 'path';
import zlib from 'zlib';
import { AudioPipelineProcessor } from './audio/pipelineProcessor';

describe('Performance, Frame Time & Bundle Size Budget', () => {
  it('enforces UI thread processing budget: average frame compute time < 16 ms', () => {
    const processor = new AudioPipelineProcessor();
    processor.reset();

    // 100 frames at 512 samples hop size
    const testSamples = new Float32Array(512);
    for (let i = 0; i < testSamples.length; i++) {
      testSamples[i] = Math.sin((2 * Math.PI * 60 * i) / 16000) * 0.2;
    }

    // Warm-up JIT
    for (let i = 0; i < 10; i++) {
      processor.pushAndProcess(testSamples);
    }

    const iterations = 100;
    const start = performance.now();
    for (let i = 0; i < iterations; i++) {
      processor.pushAndProcess(testSamples);
    }
    const elapsed = performance.now() - start;
    const avgFrameTimeMs = elapsed / iterations;

    // Enforce 16 ms 60 FPS frame budget
    expect(avgFrameTimeMs).toBeLessThan(16.0);
    // Typical runs are under 3 ms on modern CPUs
    expect(avgFrameTimeMs).toBeLessThan(8.0);
  });

  it('enforces Initial JS bundle budget: <= 200 KB gzipped', () => {
    const distAssetsDir = path.resolve(__dirname, '../dist/assets');
    if (!fs.existsSync(distAssetsDir)) {
      // If run before build, skip gracefully
      return;
    }

    const files = fs.readdirSync(distAssetsDir);
    const indexJsFile = files.find((f) => f.startsWith('index-') && f.endsWith('.js'));
    expect(indexJsFile).toBeDefined();

    const fullPath = path.join(distAssetsDir, indexJsFile!);
    const rawContent = fs.readFileSync(fullPath);
    const gzipped = zlib.gzipSync(rawContent);

    const maxBudgetBytes = 200 * 1024; // 200 KB
    expect(gzipped.byteLength).toBeLessThanOrEqual(maxBudgetBytes);
  });
});
