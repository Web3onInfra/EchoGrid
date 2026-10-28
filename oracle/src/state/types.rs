//! # Domain State Types
//!
//! Data models for challenges, claims, and payout status.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Store backend error: {0}")]
    Backend(String),
    #[error("Item not found: {0}")]
    NotFound(String),
    #[error("Challenge already used: {0}")]
    ChallengeUsed(String),
}

/// Represents an issued wallet-bound challenge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Challenge {
    pub id: String,
    pub wallet_address: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
}

impl Challenge {
    pub fn is_valid(&self, current_time: DateTime<Utc>, wallet: &str) -> bool {
        !self.used && self.wallet_address == wallet && self.expires_at >= current_time
    }
}

/// Lifecycle states of a payout claim.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PayoutStatus {
    Reserved,
    Submitted,
    Confirmed,
    Failed,
    Expired,
}

impl PayoutStatus {
    pub fn counts_towards_limit(&self) -> bool {
        matches!(self, Self::Reserved | Self::Submitted | Self::Confirmed)
    }
}

/// Complete payout claim record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claim {
    pub id: String,
    pub wallet_address: String,
    pub challenge_id: String,
    pub proof_hash: String,
    pub amount_stroops: i128,
    pub status: PayoutStatus,
    pub tx_hash: Option<String>,
    pub error: Option<String>,
    pub simulated: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_challenge_validity_rules() {
        let now = Utc::now();
        let ch = Challenge {
            id: "ch_123".into(),
            wallet_address: "GA4Z...".into(),
            issued_at: now - Duration::seconds(10),
            expires_at: now + Duration::seconds(110),
            used: false,
        };

        // Happy path
        assert!(ch.is_valid(now, "GA4Z..."));

        // Wrong wallet
        assert!(!ch.is_valid(now, "GOTHER..."));

        // Expired
        assert!(!ch.is_valid(now + Duration::seconds(120), "GA4Z..."));

        // Already used
        let mut used_ch = ch.clone();
        used_ch.used = true;
        assert!(!used_ch.is_valid(now, "GA4Z..."));
    }
}
