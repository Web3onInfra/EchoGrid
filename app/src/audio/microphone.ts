export const AUDIO_CONSTRAINTS: MediaStreamConstraints = {
  audio: {
    echoCancellation: false,
    noiseSuppression: false,
    autoGainControl: false,
    sampleRate: 16000,
    channelCount: 1,
  },
  video: false,
};

export type MicErrorType =
  'UNSUPPORTED' | 'PERMISSION_DENIED' | 'NO_DEVICE' | 'INITIALIZATION_FAILED';

export class MicError extends Error {
  constructor(
    public readonly type: MicErrorType,
    message: string,
  ) {
    super(message);
    this.name = 'MicError';
  }
}

export interface TrackDebugSettings {
  echoCancellation?: boolean;
  noiseSuppression?: boolean;
  autoGainControl?: boolean;
  sampleRate?: number;
  channelCount?: number;
  deviceId?: string;
  label?: string;
}

export type SampleCallback = (samples: Float32Array) => void;

/**
 * High-fidelity microphone capture service for EchoGrid hum monitoring.
 * Disables noise suppression, AGC, and echo cancellation to preserve mains hum signals.
 */
export class MicrophoneCapture {
  private mediaStream: MediaStream | null = null;
  private audioContext: AudioContext | null = null;
  private sourceNode: MediaStreamAudioSourceNode | null = null;
  private processorNode: ScriptProcessorNode | AudioWorkletNode | null = null;
  private activeSettings: TrackDebugSettings | null = null;

  public isCapturing(): boolean {
    return this.mediaStream !== null && this.audioContext !== null;
  }

  public async resume(): Promise<void> {
    if (this.audioContext && this.audioContext.state === 'suspended') {
      try {
        await this.audioContext.resume();
      } catch {
        // Defensive: ignore resume failure
      }
    }
  }

  public getSettings(): TrackDebugSettings | null {
    return this.activeSettings;
  }

  public async start(onSamples: SampleCallback): Promise<TrackDebugSettings> {
    if (this.isCapturing()) {
      return this.activeSettings!;
    }

    if (typeof navigator === 'undefined' || !navigator.mediaDevices?.getUserMedia) {
      throw new MicError(
        'UNSUPPORTED',
        'Microphone capture is not supported in this browser environment.',
      );
    }

    let stream: MediaStream;
    try {
      stream = await navigator.mediaDevices.getUserMedia(AUDIO_CONSTRAINTS);
    } catch (err: unknown) {
      const error = err as { name?: string; message?: string };
      if (error.name === 'NotAllowedError' || error.name === 'PermissionDeniedError') {
        throw new MicError('PERMISSION_DENIED', 'Microphone permission was denied.');
      }
      if (error.name === 'NotFoundError' || error.name === 'DevicesNotFoundError') {
        throw new MicError('NO_DEVICE', 'No microphone device was detected.');
      }
      throw new MicError(
        'INITIALIZATION_FAILED',
        error.message || 'Failed to acquire microphone stream.',
      );
    }

    this.mediaStream = stream;
    const track = stream.getAudioTracks()[0];
    if (track) {
      const settings = track.getSettings();
      this.activeSettings = {
        echoCancellation: settings.echoCancellation,
        noiseSuppression: settings.noiseSuppression,
        autoGainControl: settings.autoGainControl,
        sampleRate: settings.sampleRate,
        channelCount: settings.channelCount,
        deviceId: settings.deviceId,
        label: track.label,
      };
    }

    try {
      const AudioCtxClass =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      const ctx = new AudioCtxClass({ sampleRate: 16000 });
      this.audioContext = ctx;

      if (ctx.state === 'suspended') {
        await ctx.resume();
      }

      const source = ctx.createMediaStreamSource(stream);
      this.sourceNode = source;

      // Use 1024-sample ScriptProcessorNode for universal capture into ring buffer
      const processor = ctx.createScriptProcessor(1024, 1, 1);
      processor.onaudioprocess = (e) => {
        const inputData = e.inputBuffer.getChannelData(0);
        onSamples(new Float32Array(inputData));
      };

      source.connect(processor);
      processor.connect(ctx.destination);
      this.processorNode = processor;
    } catch (err: unknown) {
      this.stop();
      throw new MicError(
        'INITIALIZATION_FAILED',
        err instanceof Error ? err.message : 'Failed to initialize AudioContext.',
      );
    }

    return this.activeSettings!;
  }

  public stop(): void {
    if (this.processorNode) {
      try {
        this.processorNode.disconnect();
      } catch {
        // ignore
      }
      this.processorNode = null;
    }

    if (this.sourceNode) {
      try {
        this.sourceNode.disconnect();
      } catch {
        // ignore
      }
      this.sourceNode = null;
    }

    if (this.mediaStream) {
      this.mediaStream.getTracks().forEach((track) => {
        try {
          track.stop();
        } catch {
          // ignore
        }
      });
      this.mediaStream = null;
    }

    if (this.audioContext) {
      try {
        if (this.audioContext.state !== 'closed') {
          this.audioContext.close();
        }
      } catch {
        // ignore
      }
      this.audioContext = null;
    }

    this.activeSettings = null;
  }
}
