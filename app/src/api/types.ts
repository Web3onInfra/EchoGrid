export interface HealthResponse {
  status: string;
  service: string;
  version: string;
}

export interface SelfCheckReport {
  rpc_healthy: boolean;
  oracle_authorized: boolean;
  contract_paused: boolean;
  oracle_funded: boolean;
  contract_id: string;
  oracle_address: string;
}

export interface VersionResponse {
  service: string;
  version: string;
  mode: string;
}

export interface PublicConfigResponse {
  mode: string;
  network: string;
  contract_id: string | null;
  token_id: string | null;
  bounty_stroops: number;
  oracle_daily_cap_stroops: number;
  wallet_daily_claim_limit: number;
  cooldown_seconds: number;
  challenge_ttl_seconds: number;
  simulator_mode: boolean;
  explorer_base_url: string;
}

export interface ChallengeResponse {
  challenge_id: string;
  wallet_address: string;
  issued_at: string;
  expires_at: string;
}

export interface ProofV1 {
  wallet_address: string;
  challenge_id: string;
  timestamp: string;
  nonce: string;
  proof_hash: string;
  detector_version: string;
  duration_seconds: number;
  simulated: boolean;
}

export interface ProofConfirmedResponse {
  status: 'confirmed';
  claim_id: string;
  tx_hash: string;
  explorer_url: string;
  amount_stroops: number;
  simulated: boolean;
}

export interface ProofPendingResponse {
  status: 'pending';
  claim_id: string;
  poll_url: string;
}

export interface ClaimPollResponse {
  claim_id: string;
  wallet_address: string;
  status: 'reserved' | 'submitted' | 'confirmed' | 'failed' | 'expired';
  amount_stroops: number;
  tx_hash: string | null;
  explorer_url: string | null;
  error: string | null;
  simulated: boolean;
  created_at: string;
  updated_at: string;
}

export interface ApiErrorBody {
  code: string;
  message: string;
  status: number;
  details?: string | null;
}
