export const HAPTIC_PATTERNS = {
  CONFIRMED: [50, 50, 100], // Double pulse on power-down detected or payout confirmed
  TAP: 25,
  ERROR: [100, 50, 100],
} as const;

/**
 * Triggers a vibration feedback cue using the Vibration API if supported.
 * Fails silently and safely on desktop or unsupported browsers.
 */
export function triggerHaptic(pattern: number | readonly number[] = HAPTIC_PATTERNS.TAP): boolean {
  if (
    typeof navigator !== 'undefined' &&
    'vibrate' in navigator &&
    typeof navigator.vibrate === 'function'
  ) {
    try {
      const p = Array.isArray(pattern) ? [...pattern] : (pattern as number);
      return navigator.vibrate(p);
    } catch {
      return false;
    }
  }
  return false;
}
