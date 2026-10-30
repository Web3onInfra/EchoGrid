import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { usePwaInstall, BeforeInstallPromptEvent } from './usePwaInstall';

describe('usePwaInstall hook', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('initializes with isInstallable false when no prompt event has fired', () => {
    const { result } = renderHook(() => usePwaInstall());
    expect(result.current.isInstallable).toBe(false);
    expect(result.current.isInstalled).toBe(false);
  });

  it('becomes installable when beforeinstallprompt event is dispatched', () => {
    const { result } = renderHook(() => usePwaInstall());

    const fakePromptEvent = new Event('beforeinstallprompt') as BeforeInstallPromptEvent;
    fakePromptEvent.prompt = vi.fn().mockResolvedValue(undefined);
    fakePromptEvent.userChoice = Promise.resolve({ outcome: 'accepted' });

    act(() => {
      window.dispatchEvent(fakePromptEvent);
    });

    expect(result.current.isInstallable).toBe(true);
  });

  it('handles user installation acceptance', async () => {
    const { result } = renderHook(() => usePwaInstall());

    const fakePromptEvent = new Event('beforeinstallprompt') as BeforeInstallPromptEvent;
    fakePromptEvent.prompt = vi.fn().mockResolvedValue(undefined);
    fakePromptEvent.userChoice = Promise.resolve({ outcome: 'accepted' });

    act(() => {
      window.dispatchEvent(fakePromptEvent);
    });

    let success = false;
    await act(async () => {
      success = await result.current.promptInstall();
    });

    expect(success).toBe(true);
    expect(result.current.isInstalled).toBe(true);
    expect(result.current.isInstallable).toBe(false);
  });

  it('allows user to dismiss install banner', () => {
    const { result } = renderHook(() => usePwaInstall());

    const fakePromptEvent = new Event('beforeinstallprompt') as BeforeInstallPromptEvent;
    fakePromptEvent.prompt = vi.fn().mockResolvedValue(undefined);
    fakePromptEvent.userChoice = Promise.resolve({ outcome: 'dismissed' });

    act(() => {
      window.dispatchEvent(fakePromptEvent);
    });

    expect(result.current.isInstallable).toBe(true);

    act(() => {
      result.current.dismissPrompt();
    });

    expect(result.current.isDismissed).toBe(true);
    expect(result.current.isInstallable).toBe(false);
  });
});
