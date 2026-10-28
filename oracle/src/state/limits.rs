//! # Limits & Cooldowns
//!
//! Enforces per-wallet cooldown, per-wallet daily claims limits, and oracle-level
//! global daily payout caps.

use chrono::Duration;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

/// Configuration for oracle claim limits and cooldown periods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimitConfig {
    /// Minimum cooldown in seconds between claims for a single wallet (default 10 min = 600s).
    pub wallet_cooldown_seconds: i64,
    /// Maximum claims allowed for a single wallet in a rolling 24-hour window (default 2).
    pub wallet_daily_claims_limit: u32,
    /// Global daily payout cap in stroops across all payouts in a rolling 24-hour window (default 10 XLM = 100,000,000 stroops).
    pub global_daily_payout_cap_stroops: i128,
}

impl Default for LimitConfig {
    fn default() -> Self {
        Self {
            wallet_cooldown_seconds: 600,
            wallet_daily_claims_limit: 2,
            global_daily_payout_cap_stroops: 100_000_000,
        }
    }
}

impl LimitConfig {
    pub fn cooldown_duration(&self) -> Duration {
        Duration::seconds(self.wallet_cooldown_seconds)
    }

    pub fn rolling_window_duration(&self) -> Duration {
        Duration::hours(24)
    }
}

/// Errors raised when a claim reservation violates rate, cooldown, or volume limits.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum LimitError {
    #[error("Wallet cooldown active: must wait {remaining_seconds}s before submitting next claim")]
    CooldownActive { remaining_seconds: i64 },

    #[error("Wallet daily claim limit reached: maximum {limit} claims per 24 hours")]
    WalletDailyLimitExceeded { limit: u32 },

    #[error("Global daily payout cap exceeded: {spent_stroops}/{cap_stroops} stroops utilized")]
    GlobalDailyCapExceeded {
        spent_stroops: i128,
        cap_stroops: i128,
    },

    #[error("State store error: {0}")]
    Store(String),
}

impl From<LimitError> for ApiError {
    fn from(err: LimitError) -> Self {
        match err {
            LimitError::CooldownActive { .. } => {
                ApiError::too_many_requests("WALLET_COOLDOWN_ACTIVE", err.to_string())
            }
            LimitError::WalletDailyLimitExceeded { .. } => {
                ApiError::too_many_requests("WALLET_DAILY_LIMIT_EXCEEDED", err.to_string())
            }
            LimitError::GlobalDailyCapExceeded { .. } => {
                ApiError::too_many_requests("GLOBAL_CAP_EXCEEDED", err.to_string())
            }
            LimitError::Store(msg) => ApiError::internal("STORE_ERROR", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::types::{Claim, PayoutStatus};
    use crate::state::{MemoryStore, SqliteStore, StateStore};
    use chrono::Utc;
    use std::sync::Arc;

    fn make_claim(id: &str, wallet: &str, amount: i128) -> Claim {
        Claim {
            id: id.to_string(),
            wallet_address: wallet.to_string(),
            challenge_id: format!("ch_{id}"),
            proof_hash: format!("hash_{id}"),
            amount_stroops: amount,
            status: PayoutStatus::Reserved,
            tx_hash: None,
            error: None,
            simulated: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_cooldown_and_daily_limits() {
        let store = MemoryStore::new();
        let limits = LimitConfig::default();
        let now = Utc::now();
        let wallet = "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT";

        // Claim 1: succeeds
        assert!(store
            .reserve_claim(make_claim("c1", wallet, 10_000_000), &limits, now)
            .await
            .is_ok());

        // Claim 2 within 10 min: cooldown error
        let err = store
            .reserve_claim(
                make_claim("c2", wallet, 10_000_000),
                &limits,
                now + Duration::seconds(100),
            )
            .await;
        assert!(matches!(err, Err(LimitError::CooldownActive { .. })));

        // Claim 2 after cooldown (11 min): succeeds
        let t2 = now + Duration::seconds(660);
        assert!(store
            .reserve_claim(make_claim("c2", wallet, 10_000_000), &limits, t2)
            .await
            .is_ok());

        // Claim 3 after another cooldown (25 min): daily limit error (max 2 claims/24h)
        let t3 = t2 + Duration::seconds(660);
        let err3 = store
            .reserve_claim(make_claim("c3", wallet, 10_000_000), &limits, t3)
            .await;
        assert_eq!(err3, Err(LimitError::WalletDailyLimitExceeded { limit: 2 }));
    }

    #[tokio::test]
    async fn test_global_daily_cap_and_release() {
        let store = MemoryStore::new();
        let limits = LimitConfig::default(); // 100_000_000 stroops (10 XLM)
        let now = Utc::now();

        // 10 claims of 10_000_000 from 10 distinct wallets (no wallet cooldown hit)
        for i in 0..10 {
            let w = format!("G_WALLET_{i}");
            assert!(store
                .reserve_claim(make_claim(&format!("c_{i}"), &w, 10_000_000), &limits, now)
                .await
                .is_ok());
        }

        // 11th claim exceeds global daily cap
        let err = store
            .reserve_claim(make_claim("c_10", "G_WALLET_10", 10_000_000), &limits, now)
            .await;
        assert!(matches!(
            err,
            Err(LimitError::GlobalDailyCapExceeded {
                spent_stroops: 100_000_000,
                cap_stroops: 100_000_000
            })
        ));

        // Releasing one claim frees budget
        store
            .release_claim("c_0", "Transaction failed", now)
            .await
            .unwrap();
        assert!(store
            .reserve_claim(make_claim("c_10", "G_WALLET_10", 10_000_000), &limits, now)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_concurrency_same_wallet_parallel_claims() {
        let store = Arc::new(MemoryStore::new());
        let limits = LimitConfig::default();
        let now = Utc::now();
        let wallet = "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT";

        let mut handles = Vec::new();
        for i in 0..20 {
            let s = Arc::clone(&store);
            let w = wallet.to_string();
            handles.push(tokio::spawn(async move {
                s.reserve_claim(
                    make_claim(&format!("conc_{i}"), &w, 10_000_000),
                    &limits,
                    now,
                )
                .await
            }));
        }

        let mut successes = 0;
        let mut cooldown_fails = 0;
        for h in handles {
            match h.await.unwrap() {
                Ok(_) => successes += 1,
                Err(LimitError::CooldownActive { .. }) => cooldown_fails += 1,
                Err(e) => panic!("Unexpected error: {e:?}"),
            }
        }

        assert_eq!(successes, 1);
        assert_eq!(cooldown_fails, 19);
    }

    #[tokio::test]
    async fn test_concurrency_global_cap_parallel_wallets() {
        let store = Arc::new(SqliteStore::open_in_memory().unwrap());
        let limits = LimitConfig::default(); // 100_000_000 stroops
        let now = Utc::now();

        let mut handles = Vec::new();
        for i in 0..20 {
            let s = Arc::clone(&store);
            handles.push(tokio::spawn(async move {
                let w = format!("G_WALLET_{i}");
                s.reserve_claim(
                    make_claim(&format!("cap_{i}"), &w, 10_000_000),
                    &limits,
                    now,
                )
                .await
            }));
        }

        let mut successes = 0;
        let mut cap_fails = 0;
        for h in handles {
            match h.await.unwrap() {
                Ok(_) => successes += 1,
                Err(LimitError::GlobalDailyCapExceeded { .. }) => cap_fails += 1,
                Err(e) => panic!("Unexpected error: {e:?}"),
            }
        }

        // Exactly 10 claims of 10_000_000 allowed before 100_000_000 cap is hit
        assert_eq!(successes, 10);
        assert_eq!(cap_fails, 10);
    }
}
