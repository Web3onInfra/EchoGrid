//! # EchoGrid Oracle — Telemetry & Metrics
//!
//! Provides in-memory counters for claims, confirmed payouts, failures, and
//! granular rejection reasons, along with low-balance warning triggers.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use tracing::warn;

/// Internal metrics registry for tracking operational throughput and error distributions.
#[derive(Default)]
pub struct Metrics {
    pub claims_total: AtomicU64,
    pub payouts_confirmed_total: AtomicU64,
    pub payouts_failed_total: AtomicU64,
    pub payouts_simulated_total: AtomicU64,
    pub rejections_by_reason: RwLock<HashMap<String, u64>>,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_claim(&self) {
        self.claims_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_payout_confirmed(&self) {
        self.payouts_confirmed_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_payout_failed(&self) {
        self.payouts_failed_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_payout_simulated(&self) {
        self.payouts_simulated_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_rejection(&self, reason: &str) {
        let mut map = self.rejections_by_reason.write().unwrap();
        *map.entry(reason.to_string()).or_insert(0) += 1;
    }

    /// Evaluates oracle hot wallet reserve against a minimum threshold, logging a warning if low.
    pub fn check_oracle_balance(&self, balance_stroops: i128, threshold_stroops: i128) {
        if balance_stroops < threshold_stroops {
            warn!(
                balance_stroops,
                threshold_stroops,
                "Low oracle account balance! Payout transaction fee reserves depleted"
            );
        }
    }

    /// Exports an immutable snapshot of all counters.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let rejections = self.rejections_by_reason.read().unwrap().clone();
        MetricsSnapshot {
            claims_total: self.claims_total.load(Ordering::Relaxed),
            payouts_confirmed_total: self.payouts_confirmed_total.load(Ordering::Relaxed),
            payouts_failed_total: self.payouts_failed_total.load(Ordering::Relaxed),
            payouts_simulated_total: self.payouts_simulated_total.load(Ordering::Relaxed),
            rejections_by_reason: rejections,
        }
    }
}

/// JSON-serializable snapshot of all internal metrics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct MetricsSnapshot {
    pub claims_total: u64,
    pub payouts_confirmed_total: u64,
    pub payouts_failed_total: u64,
    pub payouts_simulated_total: u64,
    pub rejections_by_reason: HashMap<String, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_counters_and_snapshot() {
        let metrics = Metrics::new();

        metrics.record_claim();
        metrics.record_claim();
        metrics.record_payout_confirmed();
        metrics.record_payout_simulated();
        metrics.record_payout_failed();
        metrics.record_rejection("INVALID_WALLET_ADDRESS");
        metrics.record_rejection("INVALID_WALLET_ADDRESS");
        metrics.record_rejection("COOLDOWN_ACTIVE");

        let snap = metrics.snapshot();
        assert_eq!(snap.claims_total, 2);
        assert_eq!(snap.payouts_confirmed_total, 1);
        assert_eq!(snap.payouts_simulated_total, 1);
        assert_eq!(snap.payouts_failed_total, 1);
        assert_eq!(
            snap.rejections_by_reason.get("INVALID_WALLET_ADDRESS"),
            Some(&2)
        );
        assert_eq!(snap.rejections_by_reason.get("COOLDOWN_ACTIVE"), Some(&1));
    }

    #[test]
    fn test_low_balance_warning_trigger() {
        let metrics = Metrics::new();
        // Should trigger warn! log without panic
        metrics.check_oracle_balance(10_000_000, 50_000_000);
        // Above threshold -> normal
        metrics.check_oracle_balance(100_000_000, 50_000_000);
    }
}
