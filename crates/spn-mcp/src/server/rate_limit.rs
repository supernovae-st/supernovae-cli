//! Per-API rate limiting using governor.
//!
//! Implements token-bucket rate limiting to prevent API overload and respect
//! upstream rate limits defined in API configurations.

use std::num::NonZeroU32;
use std::sync::Arc;

use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter as GovRateLimiter};

use crate::config::RateLimitConfig;
use crate::error::{Error, Result};

/// Type alias for the governor rate limiter with our configuration.
pub type ApiRateLimiter = GovRateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Create a rate limiter from configuration.
///
/// The limiter uses a token-bucket algorithm with:
/// - `requests_per_minute`: Sustainable rate (tokens refilled per minute)
/// - `burst`: Maximum burst size (bucket capacity)
pub fn create_limiter(config: &RateLimitConfig) -> Arc<ApiRateLimiter> {
    // Use NonZeroU32::MIN (1) as fallback for invalid values
    let rpm = NonZeroU32::new(config.requests_per_minute).unwrap_or(NonZeroU32::MIN);
    let burst = NonZeroU32::new(config.burst).unwrap_or(NonZeroU32::MIN);

    let quota = Quota::per_minute(rpm).allow_burst(burst);
    Arc::new(GovRateLimiter::direct(quota))
}

/// Check if a request is allowed by the rate limiter.
///
/// Returns `Ok(())` if the request is allowed, or an error if rate limited.
pub fn check_limit(limiter: &ApiRateLimiter, api_name: &str) -> Result<()> {
    limiter.check().map_err(|_| {
        Error::Mcp(format!(
            "Rate limit exceeded for API '{}'. Please wait before retrying.",
            api_name
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let config = RateLimitConfig {
            requests_per_minute: 60,
            burst: 5,
        };
        let limiter = create_limiter(&config);

        // Should allow up to burst size immediately
        for _ in 0..5 {
            assert!(check_limit(&limiter, "test").is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let config = RateLimitConfig {
            requests_per_minute: 1,
            burst: 1,
        };
        let limiter = create_limiter(&config);

        // First request should succeed
        assert!(check_limit(&limiter, "test").is_ok());
        // Second request should be blocked (burst exhausted)
        assert!(check_limit(&limiter, "test").is_err());
    }

    #[test]
    fn test_rate_limiter_error_message() {
        let config = RateLimitConfig {
            requests_per_minute: 1,
            burst: 1,
        };
        let limiter = create_limiter(&config);

        // Exhaust the bucket
        let _ = check_limit(&limiter, "my_api");
        let result = check_limit(&limiter, "my_api");

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("my_api"));
        assert!(err_msg.contains("Rate limit exceeded"));
    }

    #[test]
    fn test_rate_limiter_handles_zero_config() {
        // Zero values should fall back to 1
        let config = RateLimitConfig {
            requests_per_minute: 0,
            burst: 0,
        };
        let limiter = create_limiter(&config);

        // Should still allow at least one request
        assert!(check_limit(&limiter, "test").is_ok());
    }

    #[test]
    fn test_create_limiter_high_burst() {
        let config = RateLimitConfig {
            requests_per_minute: 100,
            burst: 10,
        };
        let limiter = create_limiter(&config);

        // Should allow 10 rapid requests (burst)
        for i in 0..10 {
            assert!(
                check_limit(&limiter, "test").is_ok(),
                "Request {} failed",
                i
            );
        }

        // 11th should fail
        assert!(check_limit(&limiter, "test").is_err());
    }
}
