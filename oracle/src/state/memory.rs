//! # In-Memory State Store
//!
//! Thread-safe in-memory store for deterministic unit/integration testing.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::state::store::StateStore;
use crate::state::types::{Challenge, Claim, PayoutStatus, StoreError};

#[derive(Default)]
struct MemoryData {
    challenges: HashMap<String, Challenge>,
    claims: HashMap<String, Claim>,
}

#[derive(Clone, Default)]
pub struct MemoryStore {
    data: Arc<RwLock<MemoryData>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl StateStore for MemoryStore {
    async fn save_challenge(&self, challenge: Challenge) -> Result<(), StoreError> {
        self.data
            .write()
            .unwrap()
            .challenges
            .insert(challenge.id.clone(), challenge);
        Ok(())
    }

    async fn get_challenge(&self, id: &str) -> Result<Option<Challenge>, StoreError> {
        Ok(self.data.read().unwrap().challenges.get(id).cloned())
    }

    async fn mark_challenge_used(&self, id: &str) -> Result<(), StoreError> {
        let mut d = self.data.write().unwrap();
        let ch = d
            .challenges
            .get_mut(id)
            .ok_or_else(|| StoreError::NotFound(id.into()))?;
        if ch.used {
            return Err(StoreError::ChallengeUsed(id.into()));
        }
        ch.used = true;
        Ok(())
    }

    async fn save_claim(&self, claim: Claim) -> Result<(), StoreError> {
        self.data
            .write()
            .unwrap()
            .claims
            .insert(claim.id.clone(), claim);
        Ok(())
    }

    async fn get_claim(&self, id: &str) -> Result<Option<Claim>, StoreError> {
        Ok(self.data.read().unwrap().claims.get(id).cloned())
    }

    async fn update_claim_status(
        &self,
        id: &str,
        status: PayoutStatus,
        tx_hash: Option<String>,
        error: Option<String>,
        updated_at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let mut d = self.data.write().unwrap();
        let c = d
            .claims
            .get_mut(id)
            .ok_or_else(|| StoreError::NotFound(id.into()))?;
        c.status = status;
        c.tx_hash = tx_hash;
        c.error = error;
        c.updated_at = updated_at;
        Ok(())
    }

    async fn get_wallet_last_claim_time(
        &self,
        wallet: &str,
    ) -> Result<Option<DateTime<Utc>>, StoreError> {
        let d = self.data.read().unwrap();
        Ok(d.claims
            .values()
            .filter(|c| c.wallet_address == wallet && c.status.counts_towards_limit())
            .map(|c| c.created_at)
            .max())
    }

    async fn count_wallet_daily_claims(
        &self,
        wallet: &str,
        since: DateTime<Utc>,
    ) -> Result<u32, StoreError> {
        let d = self.data.read().unwrap();
        Ok(d.claims
            .values()
            .filter(|c| {
                c.wallet_address == wallet
                    && c.created_at >= since
                    && c.status.counts_towards_limit()
            })
            .count() as u32)
    }

    async fn get_daily_spent_and_reserved(&self, since: DateTime<Utc>) -> Result<i128, StoreError> {
        let d = self.data.read().unwrap();
        Ok(d.claims
            .values()
            .filter(|c| !c.simulated && c.created_at >= since && c.status.counts_towards_limit())
            .map(|c| c.amount_stroops)
            .sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn test_memory_store_challenge_and_claim_lifecycle() {
        let store = MemoryStore::new();
        let now = Utc::now();

        let ch = Challenge {
            id: "ch_test".into(),
            wallet_address: "GA4Z...".into(),
            issued_at: now,
            expires_at: now + Duration::seconds(120),
            used: false,
        };
        store.save_challenge(ch.clone()).await.unwrap();
        assert_eq!(store.get_challenge("ch_test").await.unwrap(), Some(ch));

        store.mark_challenge_used("ch_test").await.unwrap();
        assert!(matches!(
            store.mark_challenge_used("ch_test").await,
            Err(StoreError::ChallengeUsed(_))
        ));

        let claim = Claim {
            id: "clm_1".into(),
            wallet_address: "GA4Z...".into(),
            challenge_id: "ch_test".into(),
            proof_hash: "abcd".into(),
            amount_stroops: 10_000_000,
            status: PayoutStatus::Reserved,
            tx_hash: None,
            error: None,
            simulated: false,
            created_at: now,
            updated_at: now,
        };
        store.save_claim(claim).await.unwrap();

        assert_eq!(
            store
                .count_wallet_daily_claims("GA4Z...", now - Duration::hours(1))
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            store
                .get_daily_spent_and_reserved(now - Duration::hours(1))
                .await
                .unwrap(),
            10_000_000
        );

        store
            .update_claim_status(
                "clm_1",
                PayoutStatus::Confirmed,
                Some("tx_123".into()),
                None,
                now,
            )
            .await
            .unwrap();
        let updated = store.get_claim("clm_1").await.unwrap().unwrap();
        assert_eq!(updated.status, PayoutStatus::Confirmed);
        assert_eq!(updated.tx_hash.as_deref(), Some("tx_123"));
    }
}
