import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ClaimSubmissionService, ClaimProgress } from './submissionService';
import { OracleClient } from '../api/oracleClient';
import { OracleError } from '../api/errors';
import { ProofConfirmedResponse, ProofPendingResponse, ClaimPollResponse } from '../api/types';

describe('ClaimSubmissionService', () => {
  const validWallet = 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT';
  const validChallenge = 'ch_test_12345678';
  let mockClient: OracleClient;

  const makePoll = (overrides: Partial<ClaimPollResponse> = {}): ClaimPollResponse => ({
    claim_id: 'clm_default',
    wallet_address: validWallet,
    status: 'submitted',
    amount_stroops: 10000000,
    tx_hash: null,
    explorer_url: null,
    error: null,
    simulated: false,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    ...overrides,
  });

  beforeEach(() => {
    mockClient = new OracleClient({ baseUrl: 'http://mock-oracle:8080' });
  });

  it('handles immediate 200 OK confirmed response', async () => {
    const mockConfirmed: ProofConfirmedResponse = {
      status: 'confirmed',
      claim_id: 'clm_1001',
      tx_hash: 'tx_hash_abc',
      explorer_url: 'https://stellar.expert/explorer/testnet/tx/tx_hash_abc',
      amount_stroops: 10000000,
      simulated: false,
    };
    vi.spyOn(mockClient, 'submitProof').mockResolvedValue(mockConfirmed);

    const progressLogs: ClaimProgress[] = [];
    const service = new ClaimSubmissionService({ oracleClient: mockClient });
    const result = await service.submitClaim({
      walletAddress: validWallet,
      challengeId: validChallenge,
      durationSeconds: 15,
      simulated: false,
      onProgress: (p) => progressLogs.push(p),
    });

    expect(result.status).toBe('confirmed');
    expect(result.claimId).toBe('clm_1001');
    expect(result.txHash).toBe('tx_hash_abc');
    expect(progressLogs).toEqual(['building_proof', 'submitting', 'confirmed']);
    expect(service.isBusy).toBe(false);
  });

  it('handles 202 Accepted response and polls until confirmed', async () => {
    const mockPending: ProofPendingResponse = {
      status: 'pending',
      claim_id: 'clm_2002',
      poll_url: '/api/v1/claims/clm_2002',
    };
    const poll1 = makePoll({ claim_id: 'clm_2002', status: 'submitted', simulated: true });
    const poll2 = makePoll({
      claim_id: 'clm_2002',
      status: 'confirmed',
      tx_hash: 'tx_poll_success',
      simulated: true,
    });

    vi.spyOn(mockClient, 'submitProof').mockResolvedValue(mockPending);
    const getClaimSpy = vi
      .spyOn(mockClient, 'getClaim')
      .mockResolvedValueOnce(poll1)
      .mockResolvedValueOnce(poll2);

    const progressLogs: ClaimProgress[] = [];
    const service = new ClaimSubmissionService({ oracleClient: mockClient, pollIntervalMs: 5 });
    const result = await service.submitClaim({
      walletAddress: validWallet,
      challengeId: validChallenge,
      durationSeconds: 10,
      simulated: true,
      onProgress: (p) => progressLogs.push(p),
    });

    expect(result.status).toBe('confirmed');
    expect(result.txHash).toBe('tx_poll_success');
    expect(getClaimSpy).toHaveBeenCalledTimes(2);
    expect(progressLogs).toEqual(['building_proof', 'submitting', 'polling', 'confirmed']);
  });

  it('enforces double-tap protection when submission is in flight', async () => {
    vi.spyOn(mockClient, 'submitProof').mockImplementation(
      () =>
        new Promise((resolve) =>
          setTimeout(
            () =>
              resolve({
                status: 'confirmed',
                claim_id: 'clm_3003',
                tx_hash: 'hash',
                explorer_url: 'url',
                amount_stroops: 10000000,
                simulated: false,
              }),
            50,
          ),
        ),
    );

    const service = new ClaimSubmissionService({ oracleClient: mockClient });
    const call1 = service.submitClaim({
      walletAddress: validWallet,
      challengeId: validChallenge,
      durationSeconds: 10,
    });

    expect(service.isBusy).toBe(true);
    await expect(
      service.submitClaim({
        walletAddress: validWallet,
        challengeId: validChallenge,
        durationSeconds: 10,
      }),
    ).rejects.toThrow(/already in progress/i);

    await call1;
    expect(service.isBusy).toBe(false);
  });

  it('performs 1-time automatic retry when CHALLENGE_EXPIRED occurs', async () => {
    const expiredError = new OracleError({
      code: 'CHALLENGE_EXPIRED',
      message: 'Challenge has expired',
      status: 401,
    });
    const confirmedResult: ProofConfirmedResponse = {
      status: 'confirmed',
      claim_id: 'clm_retry_success',
      tx_hash: 'hash_retry',
      explorer_url: 'url_retry',
      amount_stroops: 10000000,
      simulated: false,
    };

    const submitSpy = vi
      .spyOn(mockClient, 'submitProof')
      .mockRejectedValueOnce(expiredError)
      .mockResolvedValueOnce(confirmedResult);

    const refreshChallenge = vi.fn().mockResolvedValue('ch_fresh_99999999');
    const progressLogs: ClaimProgress[] = [];
    const service = new ClaimSubmissionService({ oracleClient: mockClient });

    const result = await service.submitClaim({
      walletAddress: validWallet,
      challengeId: validChallenge,
      durationSeconds: 12,
      refreshChallenge,
      onProgress: (p) => progressLogs.push(p),
    });

    expect(result.claimId).toBe('clm_retry_success');
    expect(refreshChallenge).toHaveBeenCalledTimes(1);
    expect(submitSpy).toHaveBeenCalledTimes(2);
    expect(progressLogs).toContain('retrying');
  });

  it('fails if retry also encounters an error', async () => {
    const expiredError = new OracleError({
      code: 'CHALLENGE_EXPIRED',
      message: 'Challenge has expired',
      status: 401,
    });
    vi.spyOn(mockClient, 'submitProof').mockRejectedValue(expiredError);
    const refreshChallenge = vi.fn().mockResolvedValue('ch_fresh_99999999');
    const service = new ClaimSubmissionService({ oracleClient: mockClient });

    await expect(
      service.submitClaim({
        walletAddress: validWallet,
        challengeId: validChallenge,
        durationSeconds: 12,
        refreshChallenge,
      }),
    ).rejects.toThrow(/Challenge has expired/i);
    expect(refreshChallenge).toHaveBeenCalledTimes(1);
  });

  it('handles polling failure and propagates error', async () => {
    vi.spyOn(mockClient, 'submitProof').mockResolvedValue({
      status: 'pending',
      claim_id: 'clm_failed',
      poll_url: '/api/v1/claims/clm_failed',
    });
    vi.spyOn(mockClient, 'getClaim').mockResolvedValue(
      makePoll({
        claim_id: 'clm_failed',
        status: 'failed',
        error: 'Smart contract transaction reverted',
      }),
    );

    const service = new ClaimSubmissionService({ oracleClient: mockClient, pollIntervalMs: 5 });
    await expect(
      service.submitClaim({
        walletAddress: validWallet,
        challengeId: validChallenge,
        durationSeconds: 10,
      }),
    ).rejects.toThrow(/Smart contract transaction reverted/i);
  });

  it('propagates domain errors such as COOLDOWN_ACTIVE and RATE_LIMIT_EXCEEDED', async () => {
    const cooldownError = new OracleError({
      code: 'COOLDOWN_ACTIVE',
      message: 'Per-wallet cooldown period of 600 seconds is active',
      status: 422,
    });
    vi.spyOn(mockClient, 'submitProof').mockRejectedValue(cooldownError);
    const service = new ClaimSubmissionService({ oracleClient: mockClient });

    await expect(
      service.submitClaim({
        walletAddress: validWallet,
        challengeId: validChallenge,
        durationSeconds: 10,
      }),
    ).rejects.toThrow(/cooldown period/i);
  });
});
