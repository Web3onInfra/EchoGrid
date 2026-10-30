import { describe, it, expect, vi, beforeEach } from 'vitest';
import { triggerHaptic, HAPTIC_PATTERNS } from './haptics';

describe('triggerHaptic', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('calls navigator.vibrate with the provided pattern when supported', () => {
    const vibrateSpy = vi.fn().mockReturnValue(true);
    Object.assign(navigator, { vibrate: vibrateSpy });

    const success = triggerHaptic(HAPTIC_PATTERNS.CONFIRMED);
    expect(success).toBe(true);
    expect(vibrateSpy).toHaveBeenCalledWith([50, 50, 100]);
  });

  it('returns false when navigator.vibrate is not available', () => {
    // Delete vibrate property
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    delete (navigator as any).vibrate;

    const success = triggerHaptic();
    expect(success).toBe(false);
  });

  it('safely catches exceptions when vibrate throws', () => {
    const vibrateSpy = vi.fn().mockImplementation(() => {
      throw new Error('Vibration permission denied');
    });
    Object.assign(navigator, { vibrate: vibrateSpy });

    const success = triggerHaptic();
    expect(success).toBe(false);
  });
});
