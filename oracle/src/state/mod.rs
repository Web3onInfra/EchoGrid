//! # Oracle State Management & Persistence Layer
//!
//! Domain types, pluggable state store trait, in-memory reference implementation,
//! SQLite WAL-mode persistent store, and clock abstraction for deterministic testing.

pub mod clock;
pub mod limits;
pub mod memory;
pub mod sqlite;
pub mod store;
pub mod types;

pub use clock::{Clock, MockClock, SystemClock};
pub use limits::{LimitConfig, LimitError};
pub use memory::MemoryStore;
pub use sqlite::SqliteStore;
pub use store::StateStore;
pub use types::{Challenge, Claim, PayoutStatus, StoreError};
