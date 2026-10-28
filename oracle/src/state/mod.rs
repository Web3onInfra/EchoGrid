//! # Oracle State Management & Persistence Layer
//!
//! Domain types, pluggable state store trait, in-memory reference implementation,
//! and clock abstraction for deterministic testing.

pub mod clock;
pub mod memory;
pub mod store;
pub mod types;

pub use clock::{Clock, MockClock, SystemClock};
pub use memory::MemoryStore;
pub use store::StateStore;
pub use types::{Challenge, Claim, PayoutStatus, StoreError};
