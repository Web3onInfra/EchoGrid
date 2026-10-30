import { OracleClient, oracleClient as defaultOracleClient } from '../api/oracleClient';
import { OracleError } from '../api/errors';
import { buildProofV1 } from './proofBuilder';

export type ClaimProgress =
  'building_proof' | 'submitting' | 'polling' | 'retrying' | 'confirmed' | 'failed';

export interface SubmitClaimParams {
  walletAddress: string;
  challengeId: string;
  durationSeconds: number;
  simulated?: boolean;
  refreshChallenge?: () => Promise<string>;
  onProgress?: (progress: ClaimProgress) => void;
  signal?: AbortSignal;
}

export interface ClaimResult {
  claimId: string;
  status: 'confirmed';
  amountStroops: number;
  txHash: string | null;
  explorerUrl: string | null;
  simulated: boolean;
  walletAddress: string;
  timestamp: string;
}

export interface SubmissionServiceOptions {
  oracleClient?: OracleClient;
  pollIntervalMs?: number;
  maxPollAttempts?: number;
}

export class ClaimSubmissionService {
  private client: OracleClient;
  private pollIntervalMs: number;
  private maxPollAttempts: number;
  private isSubmitting = false;

  constructor(options: SubmissionServiceOptions = {}) {
    this.client = options.oracleClient || defaultOracleClient;
    this.pollIntervalMs = options.pollIntervalMs ?? 1000;
    this.maxPollAttempts = options.maxPollAttempts ?? 30;
  }

  public get isBusy(): boolean {
    return this.isSubmitting;
  }

  /**
   * Submits a Proof-of-Power-Down to the Oracle, handles 200/202 responses,
   * polls pending claims, implements double-tap protection and automatic
   * 1-time retry on expired challenges.
   */
  public async submitClaim(params: SubmitClaimParams): Promise<ClaimResult> {
    if (this.isSubmitting) {
      throw new Error('Claim submission is already in progress');
    }

    this.isSubmitting = true;
    try {
      return await this.executeSubmit(params, false);
    } finally {
      this.isSubmitting = false;
    }
  }

  private async executeSubmit(params: SubmitClaimParams, isRetry: boolean): Promise<ClaimResult> {
    params.onProgress?.(isRetry ? 'retrying' : 'building_proof');

    const proof = await buildProofV1({
      walletAddress: params.walletAddress,
      challengeId: params.challengeId,
      durationSeconds: params.durationSeconds,
      simulated: params.simulated,
    });

    params.onProgress?.('submitting');

    let response;
    try {
      response = await this.client.submitProof(proof, params.signal);
    } catch (err: unknown) {
      if (
        !isRetry &&
        err instanceof OracleError &&
        err.code === 'CHALLENGE_EXPIRED' &&
        params.refreshChallenge
      ) {
        // Automatic 1-retry on expired challenge with fresh challenge token
        const freshChallenge = await params.refreshChallenge();
        return this.executeSubmit({ ...params, challengeId: freshChallenge }, true);
      }
      params.onProgress?.('failed');
      throw err;
    }

    // 200 OK: immediate on-chain settlement
    if (response.status === 'confirmed') {
      params.onProgress?.('confirmed');
      return {
        claimId: response.claim_id,
        status: 'confirmed',
        amountStroops: response.amount_stroops,
        txHash: response.tx_hash,
        explorerUrl: response.explorer_url,
        simulated: response.simulated,
        walletAddress: params.walletAddress,
        timestamp: new Date().toISOString(),
      };
    }

    // 202 Accepted: async settlement, poll /api/v1/claims/{id}
    params.onProgress?.('polling');
    const result = await this.pollClaim(response.claim_id, params.signal);
    params.onProgress?.('confirmed');
    return result;
  }

  private async pollClaim(claimId: string, signal?: AbortSignal): Promise<ClaimResult> {
    let attempts = 0;

    while (attempts < this.maxPollAttempts) {
      if (signal?.aborted) {
        throw new OracleError({
          code: 'NETWORK_TIMEOUT',
          message: 'Claim polling was aborted',
          status: 408,
        });
      }

      await this.sleep(this.pollIntervalMs);
      attempts++;

      const pollRes = await this.client.getClaim(claimId, signal);

      if (pollRes.status === 'confirmed') {
        return {
          claimId: pollRes.claim_id,
          status: 'confirmed',
          amountStroops: pollRes.amount_stroops,
          txHash: pollRes.tx_hash,
          explorerUrl: pollRes.explorer_url,
          simulated: pollRes.simulated,
          walletAddress: pollRes.wallet_address,
          timestamp: pollRes.updated_at || new Date().toISOString(),
        };
      }

      if (pollRes.status === 'failed') {
        throw new OracleError({
          code: 'CLAIM_FAILED',
          message: pollRes.error || 'Claim processing failed on-chain',
          status: 400,
        });
      }

      if (pollRes.status === 'expired') {
        throw new OracleError({
          code: 'CHALLENGE_EXPIRED',
          message: 'Claim session expired before settlement completed',
          status: 401,
        });
      }

      // 'submitted' or 'reserved' -> loop again
    }

    throw new OracleError({
      code: 'NETWORK_TIMEOUT',
      message: `Claim settlement timed out after ${attempts} polling attempts`,
      status: 408,
    });
  }

  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}

export const claimSubmissionService = new ClaimSubmissionService();
