//! # SQLite Persistent State Store
//!
//! Production-grade SQLite persistence in WAL mode with embedded migrations,
//! transactional consistency, and restart recovery.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::info;

use crate::state::limits::{LimitConfig, LimitError};
use crate::state::store::StateStore;
use crate::state::types::{Challenge, Claim, PayoutStatus, StoreError};

/// SQLite-backed persistent state store.
#[derive(Clone)]
pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
}

fn map_claim_row(row: &rusqlite::Row) -> rusqlite::Result<Claim> {
    let amount_str: String = row.get(4)?;
    let status_str: String = row.get(5)?;
    let simulated_int: i32 = row.get(8)?;
    let created_str: String = row.get(9)?;
    let updated_str: String = row.get(10)?;
    Ok(Claim {
        id: row.get(0)?,
        wallet_address: row.get(1)?,
        challenge_id: row.get(2)?,
        proof_hash: row.get(3)?,
        amount_stroops: amount_str.parse().unwrap_or(0),
        status: serde_json::from_str(&format!("\"{status_str}\"")).unwrap(),
        tx_hash: row.get(6)?,
        error: row.get(7)?,
        simulated: simulated_int != 0,
        created_at: DateTime::parse_from_rfc3339(&created_str)
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_str)
            .unwrap()
            .with_timezone(&Utc),
    })
}

impl SqliteStore {
    /// Opens or creates an SQLite store at `path`, enabling WAL mode and running migrations.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StoreError> {
        let conn = Connection::open(path).map_err(|e| StoreError::Backend(e.to_string()))?;
        Self::initialize(conn)
    }

    /// Creates an in-memory SQLite store (useful for fast schema testing).
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory().map_err(|e| StoreError::Backend(e.to_string()))?;
        Self::initialize(conn)
    }

    fn initialize(conn: Connection) -> Result<Self, StoreError> {
        // Configure WAL mode and pragmas
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|e| StoreError::Backend(e.to_string()))?;

        // Run migrations
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS challenges (
                id TEXT PRIMARY KEY,
                wallet_address TEXT NOT NULL,
                issued_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                used INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_challenges_wallet ON challenges(wallet_address);

            CREATE TABLE IF NOT EXISTS claims (
                id TEXT PRIMARY KEY,
                wallet_address TEXT NOT NULL,
                challenge_id TEXT NOT NULL,
                proof_hash TEXT NOT NULL UNIQUE,
                amount_stroops TEXT NOT NULL,
                status TEXT NOT NULL,
                tx_hash TEXT,
                error TEXT,
                simulated INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_claims_wallet ON claims(wallet_address);
            CREATE INDEX IF NOT EXISTS idx_claims_status ON claims(status);
            CREATE INDEX IF NOT EXISTS idx_claims_created ON claims(created_at);",
        )
        .map_err(|e| StoreError::Backend(e.to_string()))?;

        info!("SQLite schema initialized successfully in WAL mode");
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl StateStore for SqliteStore {
    async fn save_challenge(&self, ch: Challenge) -> Result<(), StoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO challenges (id, wallet_address, issued_at, expires_at, used)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                ch.id,
                ch.wallet_address,
                ch.issued_at.to_rfc3339(),
                ch.expires_at.to_rfc3339(),
                if ch.used { 1 } else { 0 }
            ],
        )
        .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn get_challenge(&self, id: &str) -> Result<Option<Challenge>, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, wallet_address, issued_at, expires_at, used FROM challenges WHERE id = ?1")
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        stmt.query_row(params![id], |row| {
            let issued_str: String = row.get(2)?;
            let expires_str: String = row.get(3)?;
            let used_int: i32 = row.get(4)?;
            Ok(Challenge {
                id: row.get(0)?,
                wallet_address: row.get(1)?,
                issued_at: DateTime::parse_from_rfc3339(&issued_str)
                    .unwrap()
                    .with_timezone(&Utc),
                expires_at: DateTime::parse_from_rfc3339(&expires_str)
                    .unwrap()
                    .with_timezone(&Utc),
                used: used_int != 0,
            })
        })
        .optional()
        .map_err(|e| StoreError::Backend(e.to_string()))
    }

    async fn mark_challenge_used(&self, id: &str) -> Result<(), StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT used FROM challenges WHERE id = ?1")
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        let used: i32 = stmt
            .query_row(params![id], |r| r.get(0))
            .optional()
            .map_err(|e| StoreError::Backend(e.to_string()))?
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;

        if used != 0 {
            return Err(StoreError::ChallengeUsed(id.to_string()));
        }

        conn.execute("UPDATE challenges SET used = 1 WHERE id = ?1", params![id])
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn save_claim(&self, c: Claim) -> Result<(), StoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO claims (id, wallet_address, challenge_id, proof_hash, amount_stroops, status, tx_hash, error, simulated, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                c.id,
                c.wallet_address,
                c.challenge_id,
                c.proof_hash,
                c.amount_stroops.to_string(),
                serde_json::to_string(&c.status).unwrap().trim_matches('"'),
                c.tx_hash,
                c.error,
                if c.simulated { 1 } else { 0 },
                c.created_at.to_rfc3339(),
                c.updated_at.to_rfc3339()
            ],
        )
        .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn get_claim(&self, id: &str) -> Result<Option<Claim>, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, wallet_address, challenge_id, proof_hash, amount_stroops, status, tx_hash, error, simulated, created_at, updated_at FROM claims WHERE id = ?1")
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        stmt.query_row(params![id], map_claim_row)
            .optional()
            .map_err(|e| StoreError::Backend(e.to_string()))
    }

    async fn get_claim_by_proof_hash(&self, proof_hash: &str) -> Result<Option<Claim>, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, wallet_address, challenge_id, proof_hash, amount_stroops, status, tx_hash, error, simulated, created_at, updated_at FROM claims WHERE proof_hash = ?1")
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        stmt.query_row(params![proof_hash], map_claim_row)
            .optional()
            .map_err(|e| StoreError::Backend(e.to_string()))
    }

    async fn get_in_flight_claims(&self) -> Result<Vec<Claim>, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, wallet_address, challenge_id, proof_hash, amount_stroops, status, tx_hash, error, simulated, created_at, updated_at FROM claims WHERE status IN ('reserved', 'submitted')")
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        let rows = stmt
            .query_map([], map_claim_row)
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        let mut claims = Vec::new();
        for r in rows {
            claims.push(r.map_err(|e| StoreError::Backend(e.to_string()))?);
        }
        Ok(claims)
    }

    async fn update_claim_status(
        &self,
        id: &str,
        status: PayoutStatus,
        tx_hash: Option<String>,
        error: Option<String>,
        updated_at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let conn = self.conn.lock().unwrap();
        let status_str = serde_json::to_string(&status).unwrap();
        let status_clean = status_str.trim_matches('"');
        let rows = conn
            .execute(
                "UPDATE claims SET status = ?1, tx_hash = ?2, error = ?3, updated_at = ?4 WHERE id = ?5",
                params![status_clean, tx_hash, error, updated_at.to_rfc3339(), id],
            )
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        if rows == 0 {
            return Err(StoreError::NotFound(id.to_string()));
        }
        Ok(())
    }

    async fn get_wallet_last_claim_time(
        &self,
        wallet: &str,
    ) -> Result<Option<DateTime<Utc>>, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT MAX(created_at) FROM claims WHERE wallet_address = ?1 AND status IN ('reserved', 'submitted', 'confirmed')")
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        let res: Option<String> = stmt
            .query_row(params![wallet], |r| r.get(0))
            .optional()
            .map_err(|e| StoreError::Backend(e.to_string()))?
            .flatten();

        Ok(res.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|d| d.with_timezone(&Utc))
        }))
    }

    async fn count_wallet_daily_claims(
        &self,
        wallet: &str,
        since: DateTime<Utc>,
    ) -> Result<u32, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM claims WHERE wallet_address = ?1 AND created_at >= ?2 AND status IN ('reserved', 'submitted', 'confirmed')")
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        let count: i64 = stmt
            .query_row(params![wallet, since.to_rfc3339()], |r| r.get(0))
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        Ok(count as u32)
    }

    async fn get_daily_spent_and_reserved(&self, since: DateTime<Utc>) -> Result<i128, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT amount_stroops FROM claims WHERE simulated = 0 AND created_at >= ?1 AND status IN ('reserved', 'submitted', 'confirmed')")
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        let rows = stmt
            .query_map(params![since.to_rfc3339()], |r| r.get::<_, String>(0))
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        let mut total: i128 = 0;
        for r in rows {
            let amt_str = r.map_err(|e| StoreError::Backend(e.to_string()))?;
            total += amt_str.parse::<i128>().unwrap_or(0);
        }
        Ok(total)
    }

    async fn reserve_claim(
        &self,
        mut claim: Claim,
        limits: &LimitConfig,
        now: DateTime<Utc>,
    ) -> Result<(), LimitError> {
        let conn = self.conn.lock().unwrap();

        // 1. Check cooldown
        let mut cooldown_stmt = conn
            .prepare("SELECT MAX(created_at) FROM claims WHERE wallet_address = ?1 AND status IN ('reserved', 'submitted', 'confirmed')")
            .map_err(|e| LimitError::Store(e.to_string()))?;

        let last_time_str: Option<String> = cooldown_stmt
            .query_row(params![claim.wallet_address], |r| r.get(0))
            .optional()
            .map_err(|e| LimitError::Store(e.to_string()))?
            .flatten();

        if let Some(s) = last_time_str {
            if let Ok(last_time) = DateTime::parse_from_rfc3339(&s) {
                let elapsed = (now - last_time.with_timezone(&Utc)).num_seconds();
                if elapsed < limits.wallet_cooldown_seconds {
                    return Err(LimitError::CooldownActive {
                        remaining_seconds: limits.wallet_cooldown_seconds - elapsed,
                    });
                }
            }
        }

        // 2. Check wallet daily limit
        let window_start = (now - limits.rolling_window_duration()).to_rfc3339();
        let mut count_stmt = conn
            .prepare("SELECT COUNT(*) FROM claims WHERE wallet_address = ?1 AND created_at >= ?2 AND status IN ('reserved', 'submitted', 'confirmed')")
            .map_err(|e| LimitError::Store(e.to_string()))?;

        let daily_claims: i64 = count_stmt
            .query_row(params![claim.wallet_address, window_start], |r| r.get(0))
            .map_err(|e| LimitError::Store(e.to_string()))?;

        if (daily_claims as u32) >= limits.wallet_daily_claims_limit {
            return Err(LimitError::WalletDailyLimitExceeded {
                limit: limits.wallet_daily_claims_limit,
            });
        }

        // 3. Check global daily cap
        if !claim.simulated {
            let mut spent_stmt = conn
                .prepare("SELECT amount_stroops FROM claims WHERE simulated = 0 AND created_at >= ?1 AND status IN ('reserved', 'submitted', 'confirmed')")
                .map_err(|e| LimitError::Store(e.to_string()))?;

            let rows = spent_stmt
                .query_map(params![window_start], |r| r.get::<_, String>(0))
                .map_err(|e| LimitError::Store(e.to_string()))?;

            let mut daily_spent: i128 = 0;
            for r in rows {
                let amt_str = r.map_err(|e| LimitError::Store(e.to_string()))?;
                daily_spent += amt_str.parse::<i128>().unwrap_or(0);
            }

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

        conn.execute(
            "INSERT INTO claims (id, wallet_address, challenge_id, proof_hash, amount_stroops, status, tx_hash, error, simulated, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                claim.id,
                claim.wallet_address,
                claim.challenge_id,
                claim.proof_hash,
                claim.amount_stroops.to_string(),
                "reserved",
                claim.tx_hash,
                claim.error,
                if claim.simulated { 1 } else { 0 },
                claim.created_at.to_rfc3339(),
                claim.updated_at.to_rfc3339()
            ],
        )
        .map_err(|e| LimitError::Store(e.to_string()))?;

        Ok(())
    }

    async fn release_claim(
        &self,
        id: &str,
        reason: &str,
        now: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let conn = self.conn.lock().unwrap();
        let rows = conn
            .execute(
                "UPDATE claims SET status = 'failed', error = ?1, updated_at = ?2 WHERE id = ?3",
                params![reason, now.to_rfc3339(), id],
            )
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        if rows == 0 {
            return Err(StoreError::NotFound(id.to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn test_sqlite_store_restart_persistence() {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = std::env::temp_dir().join(format!("oracle_test_{unique_id}.db"));
        let now = Utc::now();

        // 1. Initial write
        {
            let store = SqliteStore::open(&db_path).unwrap();
            let ch = Challenge {
                id: "ch_sqlite".into(),
                wallet_address: "GA4Z...".into(),
                issued_at: now,
                expires_at: now + Duration::seconds(120),
                used: false,
            };
            store.save_challenge(ch).await.unwrap();

            let claim = Claim {
                id: "clm_sqlite".into(),
                wallet_address: "GA4Z...".into(),
                challenge_id: "ch_sqlite".into(),
                proof_hash: "hash_sq".into(),
                amount_stroops: 5_000_000,
                status: PayoutStatus::Reserved,
                tx_hash: None,
                error: None,
                simulated: false,
                created_at: now,
                updated_at: now,
            };
            store.save_claim(claim).await.unwrap();
        }

        // 2. Re-open database (simulating process restart)
        {
            let store = SqliteStore::open(&db_path).unwrap();
            let ch = store
                .get_challenge("ch_sqlite")
                .await
                .unwrap()
                .expect("Challenge lost across restart");
            assert_eq!(ch.wallet_address, "GA4Z...");
            assert!(!ch.used);

            let clm = store
                .get_claim("clm_sqlite")
                .await
                .unwrap()
                .expect("Claim lost across restart");
            assert_eq!(clm.amount_stroops, 5_000_000);
            assert_eq!(clm.status, PayoutStatus::Reserved);

            // Mark challenge used and confirm update
            store.mark_challenge_used("ch_sqlite").await.unwrap();
            assert!(matches!(
                store.mark_challenge_used("ch_sqlite").await,
                Err(StoreError::ChallengeUsed(_))
            ));
        }

        let _ = std::fs::remove_file(&db_path);
    }
}
