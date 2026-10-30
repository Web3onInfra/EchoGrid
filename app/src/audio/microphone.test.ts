import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { MicrophoneCapture, MicError, AUDIO_CONSTRAINTS } from './microphone';

describe('MicrophoneCapture', () => {
  let capture: MicrophoneCapture;

  beforeEach(() => {
    capture = new MicrophoneCapture();
  });

  afterEach(() => {
    capture.stop();
    vi.restoreAllMocks();
  });

  it('verifies required Phase 0 audio constraints (AGC/NS/EC disabled)', () => {
    const audio = AUDIO_CONSTRAINTS.audio as MediaTrackConstraints;
    expect(audio.echoCancellation).toBe(false);
    expect(audio.noiseSuppression).toBe(false);
    expect(audio.autoGainControl).toBe(false);
    expect(audio.sampleRate).toBe(16000);
    expect(audio.channelCount).toBe(1);
  });

  it('throws UNSUPPORTED when navigator.mediaDevices is absent', async () => {
    const origNav = globalThis.navigator;
    // @ts-expect-error mocking navigator
    delete globalThis.navigator;

    await expect(capture.start(() => {})).rejects.toThrow(MicError);
    await expect(capture.start(() => {})).rejects.toMatchObject({ type: 'UNSUPPORTED' });

    globalThis.navigator = origNav;
  });

  it('throws PERMISSION_DENIED when user denies mic permission', async () => {
    const mockGetUserMedia = vi.fn().mockRejectedValue({
      name: 'NotAllowedError',
      message: 'Permission denied',
    });

    vi.stubGlobal('navigator', {
      mediaDevices: { getUserMedia: mockGetUserMedia },
    });

    try {
      await capture.start(() => {});
      expect.unreachable('Should have thrown MicError');
    } catch (err) {
      expect(err).toBeInstanceOf(MicError);
      expect((err as MicError).type).toBe('PERMISSION_DENIED');
    }
  });

  it('throws NO_DEVICE when no microphone hardware is found', async () => {
    const mockGetUserMedia = vi.fn().mockRejectedValue({
      name: 'NotFoundError',
      message: 'No devices found',
    });

    vi.stubGlobal('navigator', {
      mediaDevices: { getUserMedia: mockGetUserMedia },
    });

    try {
      await capture.start(() => {});
      expect.unreachable('Should have thrown MicError');
    } catch (err) {
      expect(err).toBeInstanceOf(MicError);
      expect((err as MicError).type).toBe('NO_DEVICE');
    }
  });

  it('starts capture and extracts track settings', async () => {
    const mockTrack = {
      label: 'Built-in Audio',
      getSettings: () => ({
        echoCancellation: false,
        noiseSuppression: false,
        autoGainControl: false,
        sampleRate: 16000,
        channelCount: 1,
        deviceId: 'default',
      }),
      stop: vi.fn(),
    };

    const mockStream = {
      getAudioTracks: () => [mockTrack],
      getTracks: () => [mockTrack],
    };

    const mockSourceNode = {
      connect: vi.fn(),
      disconnect: vi.fn(),
    };

    const mockProcessorNode = {
      onaudioprocess: null,
      connect: vi.fn(),
      disconnect: vi.fn(),
    };

    const mockAudioContext = vi.fn().mockImplementation(() => ({
      state: 'running',
      createMediaStreamSource: vi.fn().mockReturnValue(mockSourceNode),
      createScriptProcessor: vi.fn().mockReturnValue(mockProcessorNode),
      destination: {},
      resume: vi.fn().mockResolvedValue(undefined),
      close: vi.fn().mockResolvedValue(undefined),
    }));

    vi.stubGlobal('navigator', {
      mediaDevices: {
        getUserMedia: vi.fn().mockResolvedValue(mockStream),
      },
    });
    vi.stubGlobal('AudioContext', mockAudioContext);

    const onSamples = vi.fn();
    const settings = await capture.start(onSamples);

    expect(settings.echoCancellation).toBe(false);
    expect(settings.noiseSuppression).toBe(false);
    expect(settings.autoGainControl).toBe(false);
    expect(settings.sampleRate).toBe(16000);
    expect(capture.isCapturing()).toBe(true);

    capture.stop();
    expect(capture.isCapturing()).toBe(false);
    expect(mockTrack.stop).toHaveBeenCalled();
  });
});
