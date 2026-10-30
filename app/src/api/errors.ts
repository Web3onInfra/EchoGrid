import { ApiErrorBody } from './types';

export class OracleError extends Error {
  public readonly code: string;
  public readonly status: number;
  public readonly details?: string | null;
  public readonly userMessage: string;

  constructor(body: ApiErrorBody) {
    super(body.message);
    this.name = 'OracleError';
    this.code = body.code;
    this.status = body.status;
    this.details = body.details;
    this.userMessage = getUserErrorMessage(body.code, body.status);
  }
}

export function getUserErrorMessage(code: string, status?: number): string {
  switch (code) {
    case 'INVALID_WALLET_ADDRESS':
      return 'The Stellar wallet address provided is invalid. Please check the address.';
    case 'CHALLENGE_RATE_LIMIT_EXCEEDED':
      return 'Too many challenge requests for this wallet. Please wait a moment before trying again.';
    case 'INVALID_PROOF_HASH':
      return 'The proof hash does not match computed values. Verification failed.';
    case 'CHALLENGE_EXPIRED':
      return 'Your monitoring session challenge has expired. A fresh challenge will be requested.';
    case 'REPLAY_PROOF':
    case 'REPLAY_CLAIM':
      return 'This power-down event has already been claimed. New challenge required.';
    case 'COOLDOWN_ACTIVE':
      return 'Wallet cooldown active. Please wait 10 minutes between consecutive claims.';
    case 'WALLET_DAILY_LIMIT_EXCEEDED':
      return 'Daily claim limit reached for this wallet (max 2 claims per 24 hours).';
    case 'GLOBAL_CAP_EXCEEDED':
    case 'POOL_DEPLETED':
      return 'Grid bounty pool is depleted for the day. Please check back tomorrow.';
    case 'CONTRACT_PAUSED':
      return 'EchoGrid payouts are temporarily paused by municipal admin.';
    case 'RATE_LIMIT_EXCEEDED':
      return 'Request rate limit reached. Please wait 1 minute before retrying.';
    case 'CLAIM_NOT_FOUND':
      return 'The requested claim ID was not found.';
    case 'NETWORK_TIMEOUT':
      return 'Oracle network request timed out. Please check your internet connection.';
    case 'ORACLE_UNREACHABLE':
      return 'Cannot reach the EchoGrid Oracle service. It may be offline or unreachable.';
    case 'WRONG_NETWORK':
      return 'EchoGrid operates exclusively on Stellar Testnet. Mainnet is not supported.';
    default:
      if (status && status >= 500) {
        return 'The oracle service encountered an internal error. Please try again later.';
      }
      return 'An unexpected error occurred during oracle communication.';
  }
}
