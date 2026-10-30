import { describe, it, expect } from 'vitest';
import { AudioPipelineProcessor } from './pipelineProcessor';
import { AudioPipeline } from './pipeline';

describe('AudioPipeline', () => {
  it('processes incoming audio chunks and dispatches frame events to listeners', () => {
    const pipeline = new AudioPipeline();
    const received: number[] = [];

    const unsubscribe = pipeline.subscribe((event) => {
      received.push(event.features.totalBandEnergyDb);
    });

    // Feed 1024 samples (1 frame)
    const samples = new Float32Array(1024);
    for (let i = 0; i < 1024; i++) {
      samples[i] = 0.5 * Math.sin((2 * Math.PI * 60 * i) / 16000);
    }

    pipeline.feedSamples(samples);
    expect(received.length).toBe(1);
    expect(received[0]).toBeGreaterThan(30);

    unsubscribe();
    pipeline.feedSamples(samples);
    expect(received.length).toBe(1); // Unsubscribed
  });

  describe('Backpressure & Frame Time Budget', () => {
    it('enforces backpressure handling when ring buffer exceeds capacity', () => {
      // Small capacity processor to test overflow handling
      const smallCapacityProcessor = new AudioPipelineProcessor(undefined, 2048);
      expect(smallCapacityProcessor.droppedFramesCount).toBe(0);

      // Push 4000 samples exceeding 2048 capacity
      const bigChunk = new Float32Array(4000);
      smallCapacityProcessor.pushAndProcess(bigChunk);

      expect(smallCapacityProcessor.droppedFramesCount).toBeGreaterThan(0);
    });

    it('enforces frame-time processing budget < 16ms per frame', () => {
      const processor = new AudioPipelineProcessor();
      const numFrames = 100;
      const totalSamples = 1024 + (numFrames - 1) * 512;
      const audio = new Float32Array(totalSamples);
      for (let i = 0; i < totalSamples; i++) {
        audio[i] =
          0.3 * Math.sin((2 * Math.PI * 60 * i) / 16000) +
          0.1 * Math.sin((2 * Math.PI * 120 * i) / 16000);
      }

      // Stream audio in realistic chunks of 512 samples
      const allResults = [];
      const tStart = performance.now();
      // Prime with first 1024 samples
      allResults.push(...processor.pushAndProcess(audio.slice(0, 1024)));
      for (let offset = 1024; offset <= totalSamples - 512; offset += 512) {
        allResults.push(...processor.pushAndProcess(audio.slice(offset, offset + 512)));
      }
      const totalElapsedMs = performance.now() - tStart;

      expect(allResults.length).toBe(numFrames);

      const meanFrameTimeMs = totalElapsedMs / numFrames;
      // Budget requires keeping UI thread under 16 ms per frame
      expect(meanFrameTimeMs).toBeLessThan(16.0);
      // In fact, verify it is very fast (typically < 5ms)
      expect(meanFrameTimeMs).toBeLessThan(5.0);

      for (const res of allResults) {
        expect(res.processingTimeMs).toBeLessThan(16.0);
      }
    });
  });
});
