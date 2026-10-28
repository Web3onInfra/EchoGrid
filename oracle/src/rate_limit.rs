//! # EchoGrid Oracle — Rate Limiting & Client IP Extraction
//!
//! Provides IP-based rate limiting (30 req/min) and per-wallet challenge issuance
//! rate limiting (6 req/min). Extracts client IP addresses securely behind platform
//! reverse proxies, only trusting proxy headers (e.g. `Fly-Client-IP`, `X-Forwarded-For`)
//! when explicitly configured.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use axum::http::HeaderMap;

use crate::error::ApiError;
use crate::state::Clock;

/// Safely extracts the client's IP address from HTTP headers or socket connection info.
///
/// If `trust_proxy` is `false`, proxy headers like `Fly-Client-IP` and `X-Forwarded-For`
/// are strictly ignored to prevent client IP spoofing.
pub fn extract_client_ip(
    headers: &HeaderMap,
    peer_addr: Option<SocketAddr>,
    trust_proxy: bool,
) -> IpAddr {
    if trust_proxy {
        // Fly.io injects the verified client IP into Fly-Client-IP
        if let Some(val) = headers.get("fly-client-ip").and_then(|v| v.to_str().ok()) {
            if let Ok(ip) = IpAddr::from_str(val.trim()) {
                return ip;
            }
        }

        // Generic reverse proxy standard: first IP in X-Forwarded-For is client
        if let Some(val) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
            if let Some(first) = val.split(',').next() {
                if let Ok(ip) = IpAddr::from_str(first.trim()) {
                    return ip;
                }
            }
        }
    }

    // Direct peer IP, or fallback to localhost in local/mock testing environments
    peer_addr
        .map(|addr| addr.ip())
        .unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))
}

/// In-memory sliding-window rate limiter for IP addresses and wallet addresses.
#[derive(Clone)]
pub struct RateLimiter {
    clock: Arc<dyn Clock>,
    ip_records: Arc<Mutex<HashMap<IpAddr, Vec<i64>>>>,
    wallet_records: Arc<Mutex<HashMap<String, Vec<i64>>>>,
}

impl RateLimiter {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            ip_records: Arc::new(Mutex::new(HashMap::new())),
            wallet_records: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Checks and updates the request count for a given IP address within a 60-second sliding window.
    pub fn check_ip(&self, ip: IpAddr, limit_per_min: u32) -> Result<(), ApiError> {
        let now_sec = self.clock.now().timestamp();
        let window_start = now_sec - 60;

        let mut map = self.ip_records.lock().unwrap();
        let timestamps = map.entry(ip).or_default();
        timestamps.retain(|&ts| ts > window_start);

        if timestamps.len() as u32 >= limit_per_min {
            return Err(ApiError::too_many_requests(
                "RATE_LIMIT_EXCEEDED",
                format!("IP rate limit of {limit_per_min} requests per minute exceeded"),
            ));
        }

        timestamps.push(now_sec);
        Ok(())
    }

    /// Checks and updates the challenge issuance count for a wallet within a 60-second sliding window.
    pub fn check_wallet_challenge(&self, wallet: &str, limit_per_min: u32) -> Result<(), ApiError> {
        let now_sec = self.clock.now().timestamp();
        let window_start = now_sec - 60;

        let mut map = self.wallet_records.lock().unwrap();
        let timestamps = map.entry(wallet.to_string()).or_default();
        timestamps.retain(|&ts| ts > window_start);

        if timestamps.len() as u32 >= limit_per_min {
            return Err(ApiError::too_many_requests(
                "CHALLENGE_RATE_LIMIT_EXCEEDED",
                format!("Challenge issuance rate limit of {limit_per_min} per minute exceeded for wallet"),
            ));
        }

        timestamps.push(now_sec);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::MockClock;
    use axum::http::HeaderValue;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_extract_client_ip_untrusted_proxy() {
        let mut headers = HeaderMap::new();
        headers.insert("fly-client-ip", HeaderValue::from_static("198.51.100.1"));
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.195, 10.0.0.1"),
        );

        let peer: SocketAddr = "192.0.2.1:54321".parse().unwrap();

        // trust_proxy is false -> ignores headers and uses peer
        let ip = extract_client_ip(&headers, Some(peer), false);
        assert_eq!(ip, IpAddr::from_str("192.0.2.1").unwrap());

        // fallback when peer is None
        let ip_fallback = extract_client_ip(&headers, None, false);
        assert_eq!(ip_fallback, IpAddr::from_str("127.0.0.1").unwrap());
    }

    #[test]
    fn test_extract_client_ip_trusted_proxy() {
        let mut headers = HeaderMap::new();
        headers.insert("fly-client-ip", HeaderValue::from_static("198.51.100.1"));
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.195, 10.0.0.1"),
        );

        let peer: SocketAddr = "192.0.2.1:54321".parse().unwrap();

        // trust_proxy is true -> prioritizes Fly-Client-IP
        let ip = extract_client_ip(&headers, Some(peer), true);
        assert_eq!(ip, IpAddr::from_str("198.51.100.1").unwrap());

        // without Fly-Client-IP -> uses first X-Forwarded-For
        let mut headers2 = HeaderMap::new();
        headers2.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.195, 10.0.0.1"),
        );
        let ip2 = extract_client_ip(&headers2, Some(peer), true);
        assert_eq!(ip2, IpAddr::from_str("203.0.113.195").unwrap());
    }

    #[test]
    fn test_rate_limiter_sliding_window_with_mock_clock() {
        let base_time = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
        let clock = Arc::new(MockClock::new(base_time));
        let limiter = RateLimiter::new(clock.clone());

        let ip = IpAddr::from_str("198.51.100.5").unwrap();

        // Allow 3 requests
        assert!(limiter.check_ip(ip, 3).is_ok());
        assert!(limiter.check_ip(ip, 3).is_ok());
        assert!(limiter.check_ip(ip, 3).is_ok());

        // 4th request rejected
        let err = limiter.check_ip(ip, 3).unwrap_err();
        assert_eq!(err.error_code(), "RATE_LIMIT_EXCEEDED");

        // Advance clock by 30 seconds -> still within window -> rejected
        clock.advance(chrono::Duration::seconds(30));
        assert!(limiter.check_ip(ip, 3).is_err());

        // Advance clock another 31 seconds (total 61s from start) -> window slid -> allowed!
        clock.advance(chrono::Duration::seconds(31));
        assert!(limiter.check_ip(ip, 3).is_ok());
    }

    #[test]
    fn test_wallet_challenge_rate_limiting() {
        let base_time = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
        let clock = Arc::new(MockClock::new(base_time));
        let limiter = RateLimiter::new(clock.clone());

        let wallet_a = "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT";
        let wallet_b = "GBJTH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEABC";

        // Wallet A reaches limit of 2
        assert!(limiter.check_wallet_challenge(wallet_a, 2).is_ok());
        assert!(limiter.check_wallet_challenge(wallet_a, 2).is_ok());
        assert_eq!(
            limiter
                .check_wallet_challenge(wallet_a, 2)
                .unwrap_err()
                .error_code(),
            "CHALLENGE_RATE_LIMIT_EXCEEDED"
        );

        // Wallet B is separate and succeeds
        assert!(limiter.check_wallet_challenge(wallet_b, 2).is_ok());

        // After 61 seconds, Wallet A can request again
        clock.advance(chrono::Duration::seconds(61));
        assert!(limiter.check_wallet_challenge(wallet_a, 2).is_ok());
    }
}
