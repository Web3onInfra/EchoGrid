import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ChallengeManager } from './challengeManager';
import { OracleClient } from '../api/oracleClient';
import { ChallengeResponse } from '../api/types';

describe('ChallengeManager (TTL lifecycle and fake clock tests)', () => {
  let mockClient: OracleClient;
  const testWallet = 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT';

  beforeEach(() => {
    vi.useFakeTimers();
    mockClient = new OracleClient({ baseUrl: 'http://localhost:8080' });
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('requests challenge on arming and stores it', async () => {
    const mockChallenge: ChallengeResponse = {
      challenge_id: 'ch_1',
      wallet_address: testWallet,
      issued_at: new Date(Date.now()).toISOString(),
      expires_at: new Date(Date.now() + 120000).toISOString(),
    };
    vi.spyOn(mockClient, 'getChallenge').mockResolvedValueOnce(mockChallenge);

    const manager = new ChallengeManager(mockClient, { ttlSeconds: 120 });
    const challenge = await manager.start(testWallet);

    expect(challenge.challenge_id).toBe('ch_1');
    expect(manager.getFreshestUnexpiredChallenge()?.challenge_id).toBe('ch_1');
    manager.stop();
  });

  it('automatically refreshes challenge in background before expiry', async () => {
    let callCount = 0;
    vi.spyOn(mockClient, 'getChallenge').mockImplementation(async () => {
      callCount++;
      return {
        challenge_id: `ch_${callCount}`,
        wallet_address: testWallet,
        issued_at: new Date(Date.now()).toISOString(),
        expires_at: new Date(Date.now() + 120000).toISOString(),
      };
    });

    const manager = new ChallengeManager(mockClient, { ttlSeconds: 120, safetyMarginSeconds: 5 });
    await manager.start(testWallet);
    expect(manager.getFreshestUnexpiredChallenge()?.challenge_id).toBe('ch_1');

    // Advance by 60 seconds (half TTL)
    await vi.advanceTimersByTimeAsync(60000);
    expect(mockClient.getChallenge).toHaveBeenCalledTimes(2);
    expect(manager.getFreshestUnexpiredChallenge()?.challenge_id).toBe('ch_2');

    // Advance by another 60 seconds (120s total from start)
    await vi.advanceTimersByTimeAsync(60000);
    expect(mockClient.getChallenge).toHaveBeenCalledTimes(3);
    expect(manager.getFreshestUnexpiredChallenge()?.challenge_id).toBe('ch_3');

    // ch_1 should now be expired and purged
    const active = manager.getActiveChallenges();
    expect(active.some((c) => c.challenge_id === 'ch_1')).toBe(false);

    manager.stop();
  });

  it('retries with exponential backoff on fetch failure', async () => {
    let callCount = 0;
    vi.spyOn(mockClient, 'getChallenge').mockImplementation(async () => {
      callCount++;
      if (callCount === 1) {
        return {
          challenge_id: 'ch_1',
          wallet_address: testWallet,
          issued_at: new Date(Date.now()).toISOString(),
          expires_at: new Date(Date.now() + 120000).toISOString(),
        };
      }
      if (callCount === 2) {
        throw new Error('Network timeout');
      }
      return {
        challenge_id: 'ch_recovered',
        wallet_address: testWallet,
        issued_at: new Date(Date.now()).toISOString(),
        expires_at: new Date(Date.now() + 120000).toISOString(),
      };
    });

    const manager = new ChallengeManager(mockClient, { ttlSeconds: 120 });
    await manager.start(testWallet);

    // Advance to scheduled refresh (60s) which fails
    await vi.advanceTimersByTimeAsync(60000);
    expect(callCount).toBe(2);
    // Freshest is still ch_1
    expect(manager.getFreshestUnexpiredChallenge()?.challenge_id).toBe('ch_1');

    // Advance by 2s exponential backoff
    await vi.advanceTimersByTimeAsync(2000);
    expect(callCount).toBe(3);
    // Recovered!
    expect(manager.getFreshestUnexpiredChallenge()?.challenge_id).toBe('ch_recovered');

    manager.stop();
  });

  it('purges expired challenges and returns null if all are expired', async () => {
    const expiredTime = new Date(Date.now() + 1000).toISOString(); // expires in 1s
    const mockChallenge: ChallengeResponse = {
      challenge_id: 'ch_expired_soon',
      wallet_address: testWallet,
      issued_at: new Date(Date.now()).toISOString(),
      expires_at: expiredTime,
    };
    vi.spyOn(mockClient, 'getChallenge').mockResolvedValueOnce(mockChallenge);

    const manager = new ChallengeManager(mockClient, { safetyMarginSeconds: 5 });
    await manager.start(testWallet);

    // Since safety margin is 5s and it expires in 1s, it is already considered expired
    expect(manager.getFreshestUnexpiredChallenge()).toBeNull();
    manager.stop();
  });
});
