//! # EchoGrid Oracle — Proof v1 Schema and Validation
//!
//! Canonical Proof v1 structure, pre-image serialization, SHA-256 derivation, and schema boundary validations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use stellar_strkey::Strkey;

use crate::state::Clock;

pub const MAX_CLOCK_SKEW_SECONDS: i64 = 30;
pub const MIN_DURATION_SECONDS: u32 = 5;
pub const MAX_DURATION_SECONDS: u32 = 300;

/// Canonical Proof-of-Power-Down payload submitted by clients for verification and payout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofV1 {
    pub wallet_address: String,
    pub challenge_id: String,
    pub timestamp: String,
    pub nonce: String,
    pub proof_hash: String,
    pub detector_version: String,
    pub duration_seconds: u32,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ProofValidationError {
    #[error("Invalid wallet address: must be a valid 56-char Stellar G... public key")]
    InvalidWalletAddress,
    #[error("Invalid challenge ID: challenge identifier cannot be empty")]
    InvalidChallengeId,
    #[error("Invalid timestamp format: must be valid RFC-3339 / ISO-8601 UTC string")]
    InvalidTimestampFormat,
    #[error("Timestamp outside acceptable clock skew: skewed by {skew_seconds}s (max 30s)")]
    TimestampSkewExceeded { skew_seconds: i64 },
    #[error("Invalid nonce: must be exactly 16 hexadecimal characters")]
    InvalidNonce,
    #[error("Invalid duration: {duration_seconds}s is out of range [5, 300]")]
    InvalidDuration { duration_seconds: u32 },
    #[error("Invalid detector version '{version}': must match pattern 'echonet-X.Y'")]
    InvalidDetectorVersion { version: String },
    #[error("Invalid proof hash format: must be exactly 64 hexadecimal characters")]
    InvalidProofHashFormat,
    #[error("Proof hash mismatch: expected {expected}, received {received}")]
    ProofHashMismatch { expected: String, received: String },
}

/// Derives canonical pre-image: `wallet_address | challenge_id | timestamp | nonce`
pub fn derive_proof_preimage(wallet: &str, challenge_id: &str, ts: &str, nonce: &str) -> String {
    format!("{wallet}|{challenge_id}|{ts}|{nonce}")
}

/// Derives the 32-byte SHA-256 proof hash from canonical fields.
pub fn derive_proof_hash(wallet: &str, challenge_id: &str, ts: &str, nonce: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(derive_proof_preimage(wallet, challenge_id, ts, nonce).as_bytes());
    hasher.finalize().into()
}

/// Derives the 64-character lowercase hex string of the proof hash.
pub fn derive_proof_hash_hex(wallet: &str, challenge_id: &str, ts: &str, nonce: &str) -> String {
    hex::encode(derive_proof_hash(wallet, challenge_id, ts, nonce))
}

pub fn is_valid_detector_version(version: &str) -> bool {
    if let Some(rest) = version.strip_prefix("echonet-") {
        if let Some((major, minor)) = rest.split_once('.') {
            return !major.is_empty()
                && major.chars().all(|c| c.is_ascii_digit())
                && !minor.is_empty()
                && minor.chars().all(|c| c.is_ascii_digit());
        }
    }
    false
}

pub fn is_valid_nonce(nonce: &str) -> bool {
    nonce.len() == 16 && nonce.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn is_valid_wallet_address(address: &str) -> bool {
    address.len() == 56
        && address.starts_with('G')
        && matches!(
            Strkey::from_string(address),
            Ok(Strkey::PublicKeyEd25519(_))
        )
}

impl ProofV1 {
    /// Validates schema bounds, clock skew, and cryptographic hash integrity.
    pub fn validate(&self, now: DateTime<Utc>) -> Result<[u8; 32], ProofValidationError> {
        if !is_valid_wallet_address(&self.wallet_address) {
            return Err(ProofValidationError::InvalidWalletAddress);
        }
        if self.challenge_id.trim().is_empty() {
            return Err(ProofValidationError::InvalidChallengeId);
        }
        let ts = DateTime::parse_from_rfc3339(&self.timestamp)
            .map_err(|_| ProofValidationError::InvalidTimestampFormat)?
            .with_timezone(&Utc);
        let skew = (ts - now).num_seconds().abs();
        if skew > MAX_CLOCK_SKEW_SECONDS {
            return Err(ProofValidationError::TimestampSkewExceeded { skew_seconds: skew });
        }
        if !is_valid_nonce(&self.nonce) {
            return Err(ProofValidationError::InvalidNonce);
        }
        if !is_valid_detector_version(&self.detector_version) {
            return Err(ProofValidationError::InvalidDetectorVersion {
                version: self.detector_version.clone(),
            });
        }
        if !(MIN_DURATION_SECONDS..=MAX_DURATION_SECONDS).contains(&self.duration_seconds) {
            return Err(ProofValidationError::InvalidDuration {
                duration_seconds: self.duration_seconds,
            });
        }
        if self.proof_hash.len() != 64 || !self.proof_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ProofValidationError::InvalidProofHashFormat);
        }
        let expected_hash = derive_proof_hash_hex(
            &self.wallet_address,
            &self.challenge_id,
            &self.timestamp,
            &self.nonce,
        );
        if !self.proof_hash.eq_ignore_ascii_case(&expected_hash) {
            return Err(ProofValidationError::ProofHashMismatch {
                expected: expected_hash,
                received: self.proof_hash.clone(),
            });
        }
        let mut hash_bytes = [0u8; 32];
        hex::decode_to_slice(&self.proof_hash, &mut hash_bytes)
            .map_err(|_| ProofValidationError::InvalidProofHashFormat)?;
        Ok(hash_bytes)
    }

    pub fn validate_with_clock(&self, clock: &dyn Clock) -> Result<[u8; 32], ProofValidationError> {
        self.validate(clock.now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::MockClock;

    const VALID_WALLET: &str = "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT";
    const VALID_CHALLENGE: &str = "ch_9f8b4c3e2a1d0f5e";
    const VALID_TS: &str = "2026-10-28T12:00:00Z";
    const VALID_NONCE: &str = "a1b2c3d4e5f60718";
    const GOLDEN_HASH: &str = "0b8097a1093f2ad3c44dd572c1ef54d7119edfc1ea625de477899cd6fd899dd0";

    fn sample_proof() -> ProofV1 {
        ProofV1 {
            wallet_address: VALID_WALLET.to_string(),
            challenge_id: VALID_CHALLENGE.to_string(),
            timestamp: VALID_TS.to_string(),
            nonce: VALID_NONCE.to_string(),
            proof_hash: GOLDEN_HASH.to_string(),
            detector_version: "echonet-1.2".to_string(),
            duration_seconds: 15,
        }
    }

    #[test]
    fn test_golden_vector_and_validation() {
        let preimage = derive_proof_preimage(VALID_WALLET, VALID_CHALLENGE, VALID_TS, VALID_NONCE);
        assert_eq!(
            preimage,
            "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT|ch_9f8b4c3e2a1d0f5e|2026-10-28T12:00:00Z|a1b2c3d4e5f60718"
        );
        assert_eq!(
            derive_proof_hash_hex(VALID_WALLET, VALID_CHALLENGE, VALID_TS, VALID_NONCE),
            GOLDEN_HASH
        );
        let now = DateTime::parse_from_rfc3339(VALID_TS)
            .unwrap()
            .with_timezone(&Utc);
        let clock = MockClock::new(now);
        let hash_bytes = sample_proof().validate_with_clock(&clock).unwrap();
        assert_eq!(hex::encode(hash_bytes), GOLDEN_HASH);
    }

    #[test]
    fn test_schema_and_boundary_rejections() {
        let now = DateTime::parse_from_rfc3339(VALID_TS)
            .unwrap()
            .with_timezone(&Utc);
        let mut p = sample_proof();
        p.wallet_address = "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEBAD".to_string();
        assert_eq!(
            p.validate(now),
            Err(ProofValidationError::InvalidWalletAddress)
        );

        p = sample_proof();
        p.challenge_id = "  ".to_string();
        assert_eq!(
            p.validate(now),
            Err(ProofValidationError::InvalidChallengeId)
        );

        p = sample_proof();
        p.timestamp = "invalid-date".to_string();
        assert_eq!(
            p.validate(now),
            Err(ProofValidationError::InvalidTimestampFormat)
        );

        p = sample_proof();
        p.nonce = "short".to_string();
        assert_eq!(p.validate(now), Err(ProofValidationError::InvalidNonce));

        p = sample_proof();
        p.detector_version = "echonet-1".to_string();
        assert_eq!(
            p.validate(now),
            Err(ProofValidationError::InvalidDetectorVersion {
                version: "echonet-1".into()
            })
        );

        p = sample_proof();
        p.duration_seconds = 4;
        assert_eq!(
            p.validate(now),
            Err(ProofValidationError::InvalidDuration {
                duration_seconds: 4
            })
        );
        p.duration_seconds = 301;
        assert_eq!(
            p.validate(now),
            Err(ProofValidationError::InvalidDuration {
                duration_seconds: 301
            })
        );

        p = sample_proof();
        p.proof_hash = "00".repeat(32);
        assert!(matches!(
            p.validate(now),
            Err(ProofValidationError::ProofHashMismatch { .. })
        ));
    }

    #[test]
    fn test_clock_skew_bounds() {
        let base_time = DateTime::parse_from_rfc3339(VALID_TS)
            .unwrap()
            .with_timezone(&Utc);
        let p = sample_proof();
        assert!(p
            .validate(base_time + chrono::Duration::seconds(30))
            .is_ok());
        assert!(p
            .validate(base_time - chrono::Duration::seconds(30))
            .is_ok());
        assert!(matches!(
            p.validate(base_time + chrono::Duration::seconds(31)),
            Err(ProofValidationError::TimestampSkewExceeded { skew_seconds: 31 })
        ));
    }
}
