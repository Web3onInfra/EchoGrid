//! # EchoGrid Oracle — Typed Configuration
//!
//! Environment-driven configuration with validation and secret masking (`secrecy`).
//! Supports explicit `mock` vs `live` operational modes.

use secrecy::{ExposeSecret, SecretString};
use std::env;
use std::fmt;

/// Operational mode of the oracle service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleMode {
    /// Safe offline/local mock mode. No real network calls or signing keys required.
    Mock,
    /// Live network mode. Submits real Soroban transactions and requires secrets.
    Live,
}

impl fmt::Display for OracleMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mock => write!(f, "mock"),
            Self::Live => write!(f, "live"),
        }
    }
}

/// Errors raised during configuration loading and validation.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("Invalid integer in environment variable '{var}': {val}")]
    InvalidInteger { var: &'static str, val: String },
    #[error("Missing required environment variable '{0}' in live mode")]
    MissingRequiredEnv(&'static str),
    #[error("Invalid Stellar format for '{var}': {reason}")]
    InvalidStellarFormat {
        var: &'static str,
        reason: &'static str,
    },
    #[error("Unknown ORACLE_MODE '{0}'. Expected 'mock' or 'live'")]
    InvalidOracleMode(String),
}

/// Strongly-typed runtime configuration for the oracle service.
#[derive(Clone)]
pub struct AppConfig {
    pub mode: OracleMode,
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub rpc_url: String,
    pub network_passphrase: String,
    pub contract_id: Option<String>,
    pub token_id: Option<String>,
    pub oracle_secret_key: Option<SecretString>,
    pub allowed_origins: Vec<String>,
    pub challenge_ttl_seconds: u64,
    pub cooldown_seconds: u64,
    pub wallet_daily_claim_limit: u32,
    pub oracle_daily_cap_stroops: i128,
    pub bounty_stroops: i128,
    pub rate_limit_per_min: u32,
    pub challenge_rate_per_min: u32,
    pub sync_wait_timeout_seconds: u64,
}

impl fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppConfig")
            .field("mode", &self.mode)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("database_url", &self.database_url)
            .field("rpc_url", &self.rpc_url)
            .field("network_passphrase", &self.network_passphrase)
            .field("contract_id", &self.contract_id)
            .field("token_id", &self.token_id)
            .field(
                "oracle_secret_key",
                &self.oracle_secret_key.as_ref().map(|_| "[REDACTED]"),
            )
            .field("allowed_origins", &self.allowed_origins)
            .field("challenge_ttl_seconds", &self.challenge_ttl_seconds)
            .field("cooldown_seconds", &self.cooldown_seconds)
            .field("wallet_daily_claim_limit", &self.wallet_daily_claim_limit)
            .field("oracle_daily_cap_stroops", &self.oracle_daily_cap_stroops)
            .field("bounty_stroops", &self.bounty_stroops)
            .field("rate_limit_per_min", &self.rate_limit_per_min)
            .field("challenge_rate_per_min", &self.challenge_rate_per_min)
            .field("sync_wait_timeout_seconds", &self.sync_wait_timeout_seconds)
            .finish()
    }
}

impl AppConfig {
    /// Loads configuration from process environment variables and validates values.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_env_source(|k| env::var(k).ok())
    }

    /// Parameterized loader allowing pure, environment-independent unit testing.
    pub fn from_env_source<F>(get_var: F) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let mode = match get_var("ORACLE_MODE").as_deref() {
            Some("live") | Some("LIVE") => OracleMode::Live,
            Some("mock") | Some("MOCK") | None => OracleMode::Mock,
            Some(other) => return Err(ConfigError::InvalidOracleMode(other.to_string())),
        };

        let host = get_var("HOST").unwrap_or_else(|| "0.0.0.0".to_string());
        let port = parse_int_or(&get_var, "PORT", 8080)?;
        let database_url = get_var("DATABASE_URL").unwrap_or_else(|| "echogrid.db".to_string());
        let rpc_url = get_var("SOROBAN_RPC_URL")
            .or_else(|| get_var("RPC_URL"))
            .unwrap_or_else(|| "https://soroban-testnet.stellar.org".to_string());
        let network_passphrase = get_var("NETWORK_PASSPHRASE")
            .unwrap_or_else(|| "Test SDF Network ; September 2015".to_string());

        let contract_id = get_var("CONTRACT_ID");
        let token_id = get_var("TOKEN_CONTRACT_ID").or_else(|| get_var("TOKEN_ID"));
        let oracle_secret_key = get_var("ORACLE_SECRET_KEY").map(SecretString::new);

        let allowed_origins = get_var("ALLOWED_ORIGINS")
            .map(|s| {
                s.split(',')
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect()
            })
            .unwrap_or_else(|| vec!["*".to_string()]);

        let challenge_ttl_seconds = parse_int_or(&get_var, "CHALLENGE_TTL_SECONDS", 120)?;
        let cooldown_seconds = parse_int_or(&get_var, "COOLDOWN_SECONDS", 600)?;
        let wallet_daily_claim_limit = parse_int_or(&get_var, "WALLET_DAILY_CLAIM_LIMIT", 2)?;
        let oracle_daily_cap_stroops =
            parse_int_or(&get_var, "ORACLE_DAILY_CAP_STROOPS", 100_000_000)?;
        let bounty_stroops = parse_int_or(&get_var, "BOUNTY_STROOPS", 10_000_000)?;
        let rate_limit_per_min = parse_int_or(&get_var, "RATE_LIMIT_PER_MIN", 30)?;
        let challenge_rate_per_min = parse_int_or(&get_var, "CHALLENGE_RATE_PER_MIN", 6)?;
        let sync_wait_timeout_seconds = parse_int_or(&get_var, "SYNC_WAIT_TIMEOUT_SECONDS", 20)?;

        let config = Self {
            mode,
            host,
            port,
            database_url,
            rpc_url,
            network_passphrase,
            contract_id,
            token_id,
            oracle_secret_key,
            allowed_origins,
            challenge_ttl_seconds,
            cooldown_seconds,
            wallet_daily_claim_limit,
            oracle_daily_cap_stroops,
            bounty_stroops,
            rate_limit_per_min,
            challenge_rate_per_min,
            sync_wait_timeout_seconds,
        };

        config.validate()?;
        Ok(config)
    }

    /// Validates required fields and invariants depending on active operational mode.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.mode == OracleMode::Live {
            let secret = self
                .oracle_secret_key
                .as_ref()
                .ok_or(ConfigError::MissingRequiredEnv("ORACLE_SECRET_KEY"))?;

            let secret_raw = secret.expose_secret();
            if !secret_raw.starts_with('S') || secret_raw.len() != 56 {
                return Err(ConfigError::InvalidStellarFormat {
                    var: "ORACLE_SECRET_KEY",
                    reason: "must start with 'S' and be 56 characters",
                });
            }

            let contract = self
                .contract_id
                .as_ref()
                .ok_or(ConfigError::MissingRequiredEnv("CONTRACT_ID"))?;
            if !contract.starts_with('C') || contract.len() != 56 {
                return Err(ConfigError::InvalidStellarFormat {
                    var: "CONTRACT_ID",
                    reason: "must start with 'C' and be 56 characters",
                });
            }

            let token = self
                .token_id
                .as_ref()
                .ok_or(ConfigError::MissingRequiredEnv("TOKEN_CONTRACT_ID"))?;
            if !token.starts_with('C') || token.len() != 56 {
                return Err(ConfigError::InvalidStellarFormat {
                    var: "TOKEN_CONTRACT_ID",
                    reason: "must start with 'C' and be 56 characters",
                });
            }
        }

        Ok(())
    }
}

fn parse_int_or<T, F>(get_var: &F, var: &'static str, default: T) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
    F: Fn(&str) -> Option<String>,
{
    match get_var(var) {
        Some(val) => val
            .trim()
            .parse::<T>()
            .map_err(|_| ConfigError::InvalidInteger { var, val }),
        None => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_default_mock_config() {
        let env_map = HashMap::<String, String>::new();
        let config = AppConfig::from_env_source(|k| env_map.get(k).cloned()).unwrap();

        assert_eq!(config.mode, OracleMode::Mock);
        assert_eq!(config.port, 8080);
        assert_eq!(config.challenge_ttl_seconds, 120);
        assert_eq!(config.cooldown_seconds, 600);
        assert_eq!(config.wallet_daily_claim_limit, 2);
        assert_eq!(config.oracle_daily_cap_stroops, 100_000_000);
        assert_eq!(config.bounty_stroops, 10_000_000);
        assert_eq!(config.rate_limit_per_min, 30);
    }

    #[test]
    fn test_live_mode_validation_missing_secret() {
        let mut env_map = HashMap::<String, String>::new();
        env_map.insert("ORACLE_MODE".into(), "live".into());
        env_map.insert(
            "CONTRACT_ID".into(),
            "CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF".into(),
        );
        env_map.insert(
            "TOKEN_CONTRACT_ID".into(),
            "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".into(),
        );

        let err = AppConfig::from_env_source(|k| env_map.get(k).cloned()).unwrap_err();
        assert_eq!(err, ConfigError::MissingRequiredEnv("ORACLE_SECRET_KEY"));
    }

    #[test]
    fn test_live_mode_validation_invalid_secret_format() {
        let mut env_map = HashMap::<String, String>::new();
        env_map.insert("ORACLE_MODE".into(), "live".into());
        env_map.insert("ORACLE_SECRET_KEY".into(), "G12345".into());
        env_map.insert(
            "CONTRACT_ID".into(),
            "CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF".into(),
        );
        env_map.insert(
            "TOKEN_CONTRACT_ID".into(),
            "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".into(),
        );

        let err = AppConfig::from_env_source(|k| env_map.get(k).cloned()).unwrap_err();
        assert_eq!(
            err,
            ConfigError::InvalidStellarFormat {
                var: "ORACLE_SECRET_KEY",
                reason: "must start with 'S' and be 56 characters",
            }
        );
    }

    #[test]
    fn test_live_mode_success() {
        let mut env_map = HashMap::<String, String>::new();
        env_map.insert("ORACLE_MODE".into(), "live".into());
        env_map.insert(
            "ORACLE_SECRET_KEY".into(),
            "SB7NFFU342E7GZ2U2V535HILPFEZJ6R3N4O5P6Q7R8S9T0U1V2W3X4Y5".into(),
        );
        env_map.insert(
            "CONTRACT_ID".into(),
            "CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF".into(),
        );
        env_map.insert(
            "TOKEN_CONTRACT_ID".into(),
            "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".into(),
        );

        let config = AppConfig::from_env_source(|k| env_map.get(k).cloned()).unwrap();
        assert_eq!(config.mode, OracleMode::Live);

        let debug_str = format!("{:?}", config);
        assert!(!debug_str.contains("SB7NFFU342E7GZ2U2V535HILPFEZJ6R3N4O5P6Q7R8S9T0U1V2W3X4Y5"));
        assert!(debug_str.contains("[REDACTED]"));
    }
}
