import { MicrophoneCapture, TrackDebugSettings } from './microphone';
import { AudioPipelineProcessor } from './pipelineProcessor';
import { PipelineFrameEvent, PipelineListener } from './pipelineTypes';
import { DetectorParams } from '../detector/types';

export class AudioPipeline {
  private readonly microphone: MicrophoneCapture;
  private readonly processor: AudioPipelineProcessor;
  private readonly listeners: Set<PipelineListener> = new Set();
  private isRunning: boolean = false;

  constructor(params?: Partial<DetectorParams>) {
    this.microphone = new MicrophoneCapture();
    this.processor = new AudioPipelineProcessor(params);
  }

  public subscribe(listener: PipelineListener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  public getProcessor(): AudioPipelineProcessor {
    return this.processor;
  }

  public getMicSettings(): TrackDebugSettings | null {
    return this.microphone.getSettings();
  }

  public isActive(): boolean {
    return this.isRunning;
  }

  public async start(): Promise<TrackDebugSettings> {
    if (this.isRunning) {
      return this.microphone.getSettings()!;
    }

    this.processor.reset();
    const settings = await this.microphone.start((samples) => {
      this.feedSamples(samples);
    });

    this.isRunning = true;
    return settings;
  }

  public feedSamples(samples: Float32Array): void {
    const processed = this.processor.pushAndProcess(samples);
    for (const frame of processed) {
      this.dispatchFrame(frame);
    }
  }

  public stop(): void {
    this.microphone.stop();
    this.processor.reset();
    this.isRunning = false;
  }

  private dispatchFrame(event: PipelineFrameEvent): void {
    for (const listener of this.listeners) {
      try {
        listener(event);
      } catch {
        // Prevent listener failures from breaking audio loop
      }
    }
  }
}
