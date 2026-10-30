import { OracleClient } from '../api/oracleClient';
import { ChallengeResponse } from '../api/types';

export interface ChallengeManagerOptions {
  ttlSeconds?: number;
  safetyMarginSeconds?: number;
  maxRetries?: number;
}

export class ChallengeManager {
  private readonly client: OracleClient;
  private readonly ttlSeconds: number;
  private readonly safetyMarginSeconds: number;
  private challenges: ChallengeResponse[] = [];
  private refreshTimer: ReturnType<typeof setTimeout> | null = null;
  private currentWallet: string | null = null;
  private isRunning: boolean = false;
  private retryCount: number = 0;

  constructor(client: OracleClient, options?: ChallengeManagerOptions) {
    this.client = client;
    this.ttlSeconds = options?.ttlSeconds ?? 120;
    this.safetyMarginSeconds = options?.safetyMarginSeconds ?? 10;
  }

  public isActive(): boolean {
    return this.isRunning;
  }

  public getActiveChallenges(): ChallengeResponse[] {
    this.purgeExpired();
    return [...this.challenges];
  }

  /**
   * Retrieves the newest challenge that has not yet expired.
   */
  public getFreshestUnexpiredChallenge(): ChallengeResponse | null {
    this.purgeExpired();
    if (this.challenges.length === 0) {
      return null;
    }
    // Return freshest (last added)
    return this.challenges[this.challenges.length - 1];
  }

  /**
   * Start requesting and refreshing challenges on arming.
   */
  public async start(walletAddress: string): Promise<ChallengeResponse> {
    this.stop();
    this.isRunning = true;
    this.currentWallet = walletAddress;
    this.retryCount = 0;

    return await this.fetchAndSchedule();
  }

  public stop(): void {
    if (this.refreshTimer !== null) {
      clearTimeout(this.refreshTimer);
      this.refreshTimer = null;
    }
    this.isRunning = false;
    this.currentWallet = null;
    this.retryCount = 0;
  }

  public clear(): void {
    this.stop();
    this.challenges = [];
  }

  private async fetchAndSchedule(): Promise<ChallengeResponse> {
    if (!this.isRunning || !this.currentWallet) {
      throw new Error('ChallengeManager is not running');
    }

    try {
      const challenge = await this.client.getChallenge(this.currentWallet);
      this.challenges.push(challenge);
      this.purgeExpired();
      this.retryCount = 0;

      // Schedule refresh at half TTL (e.g. every 60s for 120s TTL)
      const refreshDelayMs = (this.ttlSeconds / 2) * 1000;
      this.scheduleNext(refreshDelayMs);

      return challenge;
    } catch (err) {
      if (!this.isRunning) {
        throw err;
      }
      // Exponential backoff: 2s, 4s, 8s, up to max 30s
      this.retryCount++;
      const backoffMs = Math.min(Math.pow(2, this.retryCount) * 1000, 30000);
      this.scheduleNext(backoffMs);
      throw err;
    }
  }

  private scheduleNext(delayMs: number): void {
    if (this.refreshTimer !== null) {
      clearTimeout(this.refreshTimer);
    }
    if (!this.isRunning) return;

    this.refreshTimer = setTimeout(() => {
      this.fetchAndSchedule().catch(() => {
        // Retry scheduled in catch block
      });
    }, delayMs);
  }

  private purgeExpired(): void {
    const now = Date.now();
    this.challenges = this.challenges.filter((c) => {
      const expiresAt = new Date(c.expires_at).getTime();
      // Drop if expires within safety margin
      return expiresAt - now > this.safetyMarginSeconds * 1000;
    });
  }
}
