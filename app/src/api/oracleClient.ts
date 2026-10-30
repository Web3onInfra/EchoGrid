import { env } from '../env';
import {
  PublicConfigResponse,
  SelfCheckReport,
  HealthResponse,
  ChallengeResponse,
  ProofV1,
  ProofConfirmedResponse,
  ProofPendingResponse,
  ClaimPollResponse,
  ApiErrorBody,
} from './types';
import { OracleError } from './errors';

export interface OracleClientOptions {
  baseUrl?: string;
  defaultTimeoutMs?: number;
}

export class OracleClient {
  private baseUrl: string;
  private defaultTimeoutMs: number;

  constructor(options: OracleClientOptions = {}) {
    this.baseUrl = options.baseUrl || env.oracleUrl;
    this.defaultTimeoutMs = options.defaultTimeoutMs || 10000;
  }

  private async request<T>(path: string, init: RequestInit = {}, timeoutMs?: number): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const timeout = timeoutMs ?? this.defaultTimeoutMs;

    let timer: ReturnType<typeof setTimeout> | undefined;

    const timeoutPromise = new Promise<never>((_, reject) => {
      timer = setTimeout(() => {
        reject(
          new OracleError({
            code: 'NETWORK_TIMEOUT',
            message: `Request to ${path} timed out after ${timeout}ms`,
            status: 408,
          }),
        );
      }, timeout);
    });

    const abortPromise = new Promise<never>((_, reject) => {
      if (init.signal) {
        if (init.signal.aborted) {
          reject(
            new OracleError({
              code: 'NETWORK_TIMEOUT',
              message: `Request to ${path} was aborted`,
              status: 408,
            }),
          );
        } else {
          init.signal.addEventListener('abort', () => {
            reject(
              new OracleError({
                code: 'NETWORK_TIMEOUT',
                message: `Request to ${path} was aborted`,
                status: 408,
              }),
            );
          });
        }
      }
    });

    // Strip signal from fetchInit to prevent jsdom/Node cross-realm AbortSignal prototype clashes
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const { signal: _ignoredSignal, ...fetchInit } = init;

    const fetchPromise = (async () => {
      const response = await fetch(url, {
        ...fetchInit,
        headers: {
          'Content-Type': 'application/json',
          ...(fetchInit.headers || {}),
        },
      });

      if (!response.ok) {
        let errorBody: ApiErrorBody;
        try {
          errorBody = await response.json();
        } catch {
          errorBody = {
            code: 'HTTP_ERROR',
            message: `HTTP ${response.status} ${response.statusText}`,
            status: response.status,
          };
        }
        throw new OracleError(errorBody);
      }

      return (await response.json()) as T;
    })();

    try {
      return await Promise.race([fetchPromise, timeoutPromise, abortPromise]);
    } catch (err: unknown) {
      if (err instanceof OracleError) {
        throw err;
      }
      throw new OracleError({
        code: 'ORACLE_UNREACHABLE',
        message: err instanceof Error ? err.message : 'Failed to connect to Oracle',
        status: 503,
      });
    } finally {
      if (timer) clearTimeout(timer);
    }
  }

  public async getHealth(signal?: AbortSignal): Promise<HealthResponse> {
    return this.request<HealthResponse>('/api/v1/health', { method: 'GET', signal });
  }

  public async getReady(signal?: AbortSignal): Promise<SelfCheckReport> {
    return this.request<SelfCheckReport>('/api/v1/ready', { method: 'GET', signal });
  }

  public async getConfig(signal?: AbortSignal): Promise<PublicConfigResponse> {
    return this.request<PublicConfigResponse>('/api/v1/config', { method: 'GET', signal });
  }

  public async getChallenge(wallet: string, signal?: AbortSignal): Promise<ChallengeResponse> {
    const search = new URLSearchParams({ wallet }).toString();
    return this.request<ChallengeResponse>(`/api/v1/challenge?${search}`, {
      method: 'GET',
      signal,
    });
  }

  public async submitProof(
    proof: ProofV1,
    signal?: AbortSignal,
  ): Promise<ProofConfirmedResponse | ProofPendingResponse> {
    return this.request<ProofConfirmedResponse | ProofPendingResponse>('/api/v1/proof', {
      method: 'POST',
      body: JSON.stringify(proof),
      signal,
    });
  }

  public async getClaim(id: string, signal?: AbortSignal): Promise<ClaimPollResponse> {
    return this.request<ClaimPollResponse>(`/api/v1/claims/${encodeURIComponent(id)}`, {
      method: 'GET',
      signal,
    });
  }
}

export const oracleClient = new OracleClient();
