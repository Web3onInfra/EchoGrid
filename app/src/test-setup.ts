import '@testing-library/jest-dom';
import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';

// Fix prototype mismatch between Node Buffer and JSDOM Uint8Array for @noble/curves
if (typeof Buffer !== 'undefined' && typeof Uint8Array !== 'undefined') {
  Object.setPrototypeOf(Buffer.prototype, Uint8Array.prototype);
}

// Automatically clean up DOM after each test in singleFork mode
afterEach(() => {
  cleanup();
  if (typeof document !== 'undefined' && document.body) {
    document.body.innerHTML = '';
  }
});

// Ensure AbortController / AbortSignal in jsdom match Node's global for fetch
if (typeof globalThis.AbortController !== 'undefined') {
  window.AbortController = globalThis.AbortController;
  window.AbortSignal = globalThis.AbortSignal;
}

// JSDOM localStorage mock
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => {
      store[key] = value.toString();
    },
    removeItem: (key: string) => {
      delete store[key];
    },
    clear: () => {
      store = {};
    },
  };
})();

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock,
  writable: true,
});

// matchMedia mock
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
});

// HTMLCanvasElement mock for jsdom
if (typeof HTMLCanvasElement !== 'undefined') {
  HTMLCanvasElement.prototype.getContext = (() => ({
    drawImage: () => {},
    fillRect: () => {},
    fillStyle: '',
  })) as unknown as typeof HTMLCanvasElement.prototype.getContext;
}
