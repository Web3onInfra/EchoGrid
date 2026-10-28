//! # In-Memory State Store
//!
//! Thread-safe in-memory store for deterministic unit/integration testing.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::state::limits::{LimitConfig, LimitError};
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
        let mut d = self.data.write().unwrap();
        d.challenges.insert(challenge.id.clone(), challenge);
        Ok(())
    }

    async fn get_challenge(&self, id: &str) -> Result<Option<Challenge>, StoreError> {
        let d = self.data.read().unwrap();
        Ok(d.challenges.get(id).cloned())
    }

    async fn mark_challenge_used(&self, id: &str) -> Result<(), StoreError> {
        let mut d = self.data.write().unwrap();
        let ch = d
            .challenges
            .get_mut(id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        if ch.used {
            return Err(StoreError::ChallengeUsed(id.to_string()));
        }
        ch.used = true;
        Ok(())
    }

    async fn save_claim(&self, claim: Claim) -> Result<(), StoreError> {
        let mut d = self.data.write().unwrap();
        d.claims.insert(claim.id.clone(), claim);
        Ok(())
    }

    async fn get_claim(&self, id: &str) -> Result<Option<Claim>, StoreError> {
        let d = self.data.read().unwrap();
        Ok(d.claims.get(id).cloned())
    }

    async fn get_claim_by_proof_hash(&self, proof_hash: &str) -> Result<Option<Claim>, StoreError> {
        let d = self.data.read().unwrap();
        Ok(d.claims
            .values()
            .find(|c| c.proof_hash == proof_hash)
            .cloned())
    }

    async fn get_in_flight_claims(&self) -> Result<Vec<Claim>, StoreError> {
        let d = self.data.read().unwrap();
        Ok(d.claims
            .values()
            .filter(|c| matches!(c.status, PayoutStatus::Reserved | PayoutStatus::Submitted))
            .cloned()
            .collect())
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

    async fn reserve_claim(
        &self,
        mut claim: Claim,
        limits: &LimitConfig,
        now: DateTime<Utc>,
    ) -> Result<(), LimitError> {
        let mut d = self.data.write().unwrap();

        // 1. Check cooldown
        if let Some(last_time) = d
            .claims
            .values()
            .filter(|c| c.wallet_address == claim.wallet_address && c.status.counts_towards_limit())
            .map(|c| c.created_at)
            .max()
        {
            let elapsed = (now - last_time).num_seconds();
            if elapsed < limits.wallet_cooldown_seconds {
                return Err(LimitError::CooldownActive {
                    remaining_seconds: limits.wallet_cooldown_seconds - elapsed,
                });
            }
        }

        // 2. Check wallet daily limit
        let window_start = now - limits.rolling_window_duration();
        let daily_claims = d
            .claims
            .values()
            .filter(|c| {
                c.wallet_address == claim.wallet_address
                    && c.created_at >= window_start
                    && c.status.counts_towards_limit()
            })
            .count() as u32;

        if daily_claims >= limits.wallet_daily_claims_limit {
            return Err(LimitError::WalletDailyLimitExceeded {
                limit: limits.wallet_daily_claims_limit,
            });
        }

        // 3. Check global daily cap (simulated claims do not count towards chain payout cap)
        if !claim.simulated {
            let daily_spent: i128 = d
                .claims
                .values()
                .filter(|c| {
                    !c.simulated && c.created_at >= window_start && c.status.counts_towards_limit()
                })
                .map(|c| c.amount_stroops)
                .sum();

            if daily_spent + claim.amount_stroops > limits.global_daily_payout_cap_stroops {
                return Err(LimitError::GlobalDailyCapExceeded {
                    spent_stroops: daily_spent,
                    cap_stroops: limits.global_daily_payout_cap_stroops,
                });
            }
        }

        claim.status = PayoutStatus::Reserved;
        claim.created_at = now;
        claim.updated_at = now;
        d.claims.insert(claim.id.clone(), claim);
        Ok(())
    }

    async fn release_claim(
        &self,
        id: &str,
        reason: &str,
        now: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let mut d = self.data.write().unwrap();
        let c = d
            .claims
            .get_mut(id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        c.status = PayoutStatus::Failed;
        c.error = Some(reason.to_string());
        c.updated_at = now;
        Ok(())
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
        let fetched = store.get_challenge("ch_test").await.unwrap().unwrap();
        assert_eq!(fetched.id, "ch_test");

        store.mark_challenge_used("ch_test").await.unwrap();
        assert!(store.mark_challenge_used("ch_test").await.is_err());

        let claim = Claim {
            id: "clm_1".into(),
            wallet_address: "GA4Z...".into(),
            challenge_id: "ch_test".into(),
            proof_hash: "hash".into(),
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
            .update_claim_status("clm_1", PayoutStatus::Failed, None, Some("err".into()), now)
            .await
            .unwrap();
        assert_eq!(
            store
                .count_wallet_daily_claims("GA4Z...", now - Duration::hours(1))
                .await
                .unwrap(),
            0
        );
    }
}
