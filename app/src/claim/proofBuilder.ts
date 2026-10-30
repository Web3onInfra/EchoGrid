import { ProofV1 } from '../api/types';
import { isValidStellarAddress } from '../wallet/validation';

export interface BuildProofParams {
  walletAddress: string;
  challengeId: string;
  durationSeconds: number;
  simulated?: boolean;
  detectorVersion?: string;
  timestamp?: string;
  nonce?: string;
}

export const DEFAULT_DETECTOR_VERSION = 'echonet-1.2';
export const MAX_PROOF_PAYLOAD_BYTES = 1024; // 1 KB strict cap

/**
 * Computes SHA-256 hash of a string using Web Crypto API.
 */
export async function sha256Hex(message: string): Promise<string> {
  const msgBuffer = new TextEncoder().encode(message);
  const hashBuffer = await crypto.subtle.digest('SHA-256', msgBuffer);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
}

/**
 * Generates a random 16-character hexadecimal nonce (8 random bytes).
 */
export function generateNonce(): string {
  const bytes = new Uint8Array(8);
  crypto.getRandomValues(bytes);
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

/**
 * Constructs a Proof v1 payload strictly from derived detector summaries and challenge session.
 * Audio never leaves the device: guarantees zero raw audio is included.
 */
export async function buildProofV1(params: BuildProofParams): Promise<ProofV1> {
  if (!isValidStellarAddress(params.walletAddress)) {
    throw new Error(`Invalid Stellar wallet address format: ${params.walletAddress}`);
  }

  if (!params.challengeId || params.challengeId.length === 0) {
    throw new Error('challengeId must not be empty');
  }

  const timestamp = params.timestamp || new Date().toISOString();
  const nonce = params.nonce || generateNonce();

  if (nonce.length !== 16 || !/^[0-9a-f]{16}$/i.test(nonce)) {
    throw new Error(`nonce must be exactly 16 hexadecimal characters, got ${nonce}`);
  }

  // Pre-image derivation per Phase 2 design Section 4:
  // pre_image = wallet_address | challenge_id | timestamp | nonce
  const preImage = `${params.walletAddress}|${params.challengeId}|${timestamp}|${nonce}`;
  const proofHash = await sha256Hex(preImage);

  const detectorVersion = params.detectorVersion || DEFAULT_DETECTOR_VERSION;
  if (!/^echonet-[0-9]+\.[0-9]+$/.test(detectorVersion)) {
    throw new Error(`Invalid detector_version format: ${detectorVersion}`);
  }

  // Clamp duration bound: 5 <= duration <= 300 seconds per Phase 2 spec
  const clampedDuration = Math.min(Math.max(Math.round(params.durationSeconds), 5), 300);

  const proof: ProofV1 = {
    wallet_address: params.walletAddress,
    challenge_id: params.challengeId,
    timestamp,
    nonce,
    proof_hash: proofHash,
    detector_version: detectorVersion,
    duration_seconds: clampedDuration,
    simulated: Boolean(params.simulated),
  };

  // Enforce size cap
  const jsonStr = JSON.stringify(proof);
  const byteLength = new TextEncoder().encode(jsonStr).length;
  if (byteLength > MAX_PROOF_PAYLOAD_BYTES) {
    throw new Error(
      `Proof payload size (${byteLength} bytes) exceeds maximum size cap of ${MAX_PROOF_PAYLOAD_BYTES} bytes.`,
    );
  }

  return proof;
}
