import { describe, it, expect } from 'vitest';
import { buildProofV1, sha256Hex, MAX_PROOF_PAYLOAD_BYTES } from './proofBuilder';

describe('ProofBuilder (Proof v1 canonical schema & privacy assertion)', () => {
  const validWallet = 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT';
  const validChallenge = 'ch_test_12345678';

  it('builds canonical Proof v1 matching Phase 2 pre-image SHA-256 hash', async () => {
    const fixedTimestamp = '2026-10-30T12:00:00.000Z';
    const fixedNonce = '0123456789abcdef';

    const proof = await buildProofV1({
      walletAddress: validWallet,
      challengeId: validChallenge,
      durationSeconds: 15,
      timestamp: fixedTimestamp,
      nonce: fixedNonce,
      simulated: false,
    });

    expect(proof.wallet_address).toBe(validWallet);
    expect(proof.challenge_id).toBe(validChallenge);
    expect(proof.timestamp).toBe(fixedTimestamp);
    expect(proof.nonce).toBe(fixedNonce);
    expect(proof.duration_seconds).toBe(15);
    expect(proof.simulated).toBe(false);
    expect(proof.detector_version).toBe('echonet-1.2');

    // Verify SHA-256 pre-image derivation
    const expectedPreImage = `${validWallet}|${validChallenge}|${fixedTimestamp}|${fixedNonce}`;
    const expectedHash = await sha256Hex(expectedPreImage);
    expect(proof.proof_hash).toBe(expectedHash);
    expect(proof.proof_hash.length).toBe(64);
  });

  it('strictly enforces the 1024-byte payload size cap', async () => {
    const proof = await buildProofV1({
      walletAddress: validWallet,
      challengeId: validChallenge,
      durationSeconds: 10,
    });

    const jsonString = JSON.stringify(proof);
    const bytes = new TextEncoder().encode(jsonString).length;

    expect(bytes).toBeLessThanOrEqual(MAX_PROOF_PAYLOAD_BYTES);
    // Typical proof is around 250-300 bytes
    expect(bytes).toBeLessThan(400);
  });

  it('CRITICAL PRIVACY TEST: asserts zero raw audio buffers ever enter the proof payload', async () => {
    const proof = await buildProofV1({
      walletAddress: validWallet,
      challengeId: validChallenge,
      durationSeconds: 8.5,
      simulated: true,
    });

    const forbiddenKeys = [
      'audio',
      'samples',
      'raw_audio',
      'pcm',
      'waveform',
      'buffer',
      'recording',
      'stream',
      'magnitudes',
      'spectrum',
    ];

    const proofKeys = Object.keys(proof);
    for (const forbidden of forbiddenKeys) {
      expect(proofKeys).not.toContain(forbidden);
    }

    // Inspect every value in the payload
    for (const value of Object.values(proof)) {
      expect(value).not.toBeInstanceOf(Float32Array);
      expect(value).not.toBeInstanceOf(Float64Array);
      expect(value).not.toBeInstanceOf(ArrayBuffer);
      expect(value).not.toBeInstanceOf(Uint8Array);
      expect(Array.isArray(value)).toBe(false);

      if (typeof value === 'string') {
        // Must not contain base64 encoded audio or data URLs
        expect(value.startsWith('data:audio')).toBe(false);
      }
    }
  });

  it('clamps duration_seconds within [5, 300] per spec', async () => {
    const tooShort = await buildProofV1({
      walletAddress: validWallet,
      challengeId: validChallenge,
      durationSeconds: 2, // < 5s
    });
    expect(tooShort.duration_seconds).toBe(5);

    const tooLong = await buildProofV1({
      walletAddress: validWallet,
      challengeId: validChallenge,
      durationSeconds: 500, // > 300s
    });
    expect(tooLong.duration_seconds).toBe(300);
  });

  it('rejects invalid wallet address formats', async () => {
    await expect(
      buildProofV1({
        walletAddress: 'INVALID_STELLAR_ADDR',
        challengeId: validChallenge,
        durationSeconds: 10,
      }),
    ).rejects.toThrow(/Invalid Stellar wallet address/i);
  });

  it('rejects invalid nonce formats', async () => {
    await expect(
      buildProofV1({
        walletAddress: validWallet,
        challengeId: validChallenge,
        durationSeconds: 10,
        nonce: 'short_nonce', // not 16 hex chars
      }),
    ).rejects.toThrow(/nonce must be exactly 16 hexadecimal characters/i);
  });
});
