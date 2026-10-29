import { describe, it, expect } from 'vitest';
import { getAppEnv } from './env';

describe('getAppEnv', () => {
  it('returns default oracle URL when env is undefined', () => {
    const config = getAppEnv();
    expect(config.oracleUrl).toBeDefined();
    expect(typeof config.oracleUrl).toBe('string');
  });
});
