import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import { SpectrogramCanvas } from './SpectrogramCanvas';

describe('SpectrogramCanvas', () => {
  const mockDrawImage = vi.fn();
  const mockFillRect = vi.fn();
  const originalGetContext = HTMLCanvasElement.prototype.getContext;
  const originalMatchMedia = window.matchMedia;
  const originalRAF = window.requestAnimationFrame;

  beforeEach(() => {
    vi.clearAllMocks();
    window.requestAnimationFrame = vi.fn().mockImplementation((cb: FrameRequestCallback) => {
      cb(performance.now());
      return 1;
    });
    HTMLCanvasElement.prototype.getContext = vi.fn().mockReturnValue({
      drawImage: mockDrawImage,
      fillRect: mockFillRect,
      fillStyle: '',
    }) as unknown as typeof HTMLCanvasElement.prototype.getContext;
  });

  afterEach(() => {
    HTMLCanvasElement.prototype.getContext = originalGetContext;
    window.matchMedia = originalMatchMedia;
    window.requestAnimationFrame = originalRAF;
    Object.defineProperty(document, 'hidden', { value: false, configurable: true });
  });

  it('renders canvas element and container', () => {
    render(<SpectrogramCanvas fftMagnitudes={null} />);
    expect(screen.getByTestId('spectrogram-container')).toBeInTheDocument();
    expect(screen.getByTestId('spectrogram-canvas')).toBeInTheDocument();
  });

  it('draws waterfall column when new FFT magnitudes arrive', () => {
    const magnitudes = new Float32Array(513);
    magnitudes[3] = 10.0; // 50 Hz bin
    magnitudes[4] = 8.0; // 60 Hz bin

    render(<SpectrogramCanvas fftMagnitudes={magnitudes} />);

    expect(mockDrawImage).toHaveBeenCalled();
    expect(mockFillRect).toHaveBeenCalled();
  });

  it('pauses rendering when document is hidden', () => {
    const magnitudes = new Float32Array(513);

    render(<SpectrogramCanvas fftMagnitudes={magnitudes} />);
    expect(mockDrawImage).toHaveBeenCalledTimes(1);

    // Simulate document visibility change to hidden
    Object.defineProperty(document, 'hidden', { value: true, configurable: true });
    act(() => {
      document.dispatchEvent(new Event('visibilitychange'));
    });

    render(<SpectrogramCanvas fftMagnitudes={magnitudes} />);
    // Should not draw additional waterfall columns while hidden
    expect(mockDrawImage).toHaveBeenCalledTimes(1);
  });

  it('respects prefers-reduced-motion', () => {
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: query.includes('prefers-reduced-motion'),
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    const magnitudes = new Float32Array(513);
    render(<SpectrogramCanvas fftMagnitudes={magnitudes} />);

    expect(mockDrawImage).not.toHaveBeenCalled();
  });
});
