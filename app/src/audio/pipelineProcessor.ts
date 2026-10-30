import { AudioFeatureExtractor, FrameFeatures } from './features';
import { StreamingPowerDownDetector } from '../detector/detector';
import { FrameResult, DetectorParams } from '../detector/types';
import { AudioRingBuffer } from './ringBuffer';

export interface ProcessedFrame {
  frameResult: FrameResult;
  features: FrameFeatures;
  processingTimeMs: number;
}

/**
 * Core streaming processor running feature extraction and detection over an audio ring buffer.
 * Supports backpressure management by dropping stale audio if backlog exceeds threshold.
 */
export class AudioPipelineProcessor {
  public readonly featureExtractor: AudioFeatureExtractor;
  public readonly detector: StreamingPowerDownDetector;
  public readonly ringBuffer: AudioRingBuffer;
  public droppedFramesCount: number = 0;

  constructor(params?: Partial<DetectorParams>, maxBufferCapacity: number = 16384) {
    this.featureExtractor = new AudioFeatureExtractor(params);
    this.detector = new StreamingPowerDownDetector(params);
    this.ringBuffer = new AudioRingBuffer(maxBufferCapacity);
  }

  public reset(): void {
    this.detector.reset();
    this.ringBuffer.clear();
    this.droppedFramesCount = 0;
  }

  /**
   * Push newly arrived audio samples and process all complete hop frames.
   */
  public pushAndProcess(samples: Float32Array): ProcessedFrame[] {
    const frameSize = this.featureExtractor.params.frame_size;
    const hopSize = this.featureExtractor.params.hop_size;

    // Backpressure: if buffer is nearly full, clear stale samples
    if (this.ringBuffer.available() + samples.length > this.ringBuffer.capacity) {
      this.ringBuffer.clear();
      this.droppedFramesCount++;
    }

    this.ringBuffer.push(samples);

    const results: ProcessedFrame[] = [];

    while (this.ringBuffer.canRead(frameSize)) {
      const t0 = performance.now();
      const frame = this.ringBuffer.read(frameSize, hopSize);
      if (!frame) break;

      const features = this.featureExtractor.extractFrameFeatures(frame);
      const frameResult = this.detector.processEnergyFrame(features.totalBandEnergyDb);
      const processingTimeMs = performance.now() - t0;

      results.push({
        frameResult,
        features,
        processingTimeMs,
      });
    }

    return results;
  }
}
