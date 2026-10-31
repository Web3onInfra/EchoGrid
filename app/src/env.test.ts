import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { getAppEnv } from './env';

describe('getAppEnv', () => {
  const originalLocation = window.location;

  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    window.localStorage.clear();
    Object.defineProperty(window, 'location', {
      writable: true,
      value: originalLocation,
    });
  });

  it('returns default oracle URL when env is undefined', () => {
    const config = getAppEnv();
    expect(config.oracleUrl).toBeDefined();
    expect(typeof config.oracleUrl).toBe('string');
  });

  it('honors query parameter override ?oracle=', () => {
    Object.defineProperty(window, 'location', {
      writable: true,
      value: new URL('https://echogrid-pwa.vercel.app?oracle=https://custom-oracle.railway.app/'),
    });

    const config = getAppEnv();
    expect(config.oracleUrl).toBe('https://custom-oracle.railway.app');
    expect(window.localStorage.getItem('echogrid_oracle_url')).toBe('https://custom-oracle.railway.app/');
  });

  it('honors localStorage override when query param is not present', () => {
    window.localStorage.setItem('echogrid_oracle_url', 'https://saved-oracle.fly.dev/');
    Object.defineProperty(window, 'location', {
      writable: true,
      value: new URL('https://echogrid-pwa.vercel.app'),
    });

    const config = getAppEnv();
    expect(config.oracleUrl).toBe('https://saved-oracle.fly.dev');
  });
});
