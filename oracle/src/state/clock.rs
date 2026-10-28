//! # Time Abstraction Layer
//!
//! Provides the `Clock` trait enabling deterministic time injection
//! without sleeping in cooldown and TTL tests.

use chrono::{DateTime, Duration, Utc};
use std::sync::{Arc, RwLock};

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Clone, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Clone)]
pub struct MockClock {
    now: Arc<RwLock<DateTime<Utc>>>,
}

impl MockClock {
    pub fn new(initial: DateTime<Utc>) -> Self {
        Self {
            now: Arc::new(RwLock::new(initial)),
        }
    }

    pub fn set(&self, time: DateTime<Utc>) {
        *self.now.write().unwrap() = time;
    }

    pub fn advance(&self, duration: Duration) {
        let mut guard = self.now.write().unwrap();
        *guard += duration;
    }
}

impl Clock for MockClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.read().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_clock_advance_and_set() {
        let base = DateTime::parse_from_rfc3339("2026-10-28T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let clock = MockClock::new(base);
        assert_eq!(clock.now(), base);

        clock.advance(Duration::seconds(60));
        assert_eq!(clock.now(), base + Duration::seconds(60));

        let new_time = base + Duration::hours(1);
        clock.set(new_time);
        assert_eq!(clock.now(), new_time);
    }
}
