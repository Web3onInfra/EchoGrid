//! # State Store Trait
//!
//! Storage interface for challenges, claims, and payout limits.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::state::types::{Challenge, Claim, PayoutStatus, StoreError};

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn save_challenge(&self, challenge: Challenge) -> Result<(), StoreError>;
    async fn get_challenge(&self, id: &str) -> Result<Option<Challenge>, StoreError>;
    async fn mark_challenge_used(&self, id: &str) -> Result<(), StoreError>;

    async fn save_claim(&self, claim: Claim) -> Result<(), StoreError>;
    async fn get_claim(&self, id: &str) -> Result<Option<Claim>, StoreError>;
    async fn update_claim_status(
        &self,
        id: &str,
        status: PayoutStatus,
        tx_hash: Option<String>,
        error: Option<String>,
        updated_at: DateTime<Utc>,
    ) -> Result<(), StoreError>;

    async fn get_wallet_last_claim_time(
        &self,
        wallet: &str,
    ) -> Result<Option<DateTime<Utc>>, StoreError>;
    async fn count_wallet_daily_claims(
        &self,
        wallet: &str,
        since: DateTime<Utc>,
    ) -> Result<u32, StoreError>;
    async fn get_daily_spent_and_reserved(&self, since: DateTime<Utc>) -> Result<i128, StoreError>;
}
