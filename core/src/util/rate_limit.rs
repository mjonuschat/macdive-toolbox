use governor::clock::QuantaClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Jitter, Quota, RateLimiter};
use std::num::NonZeroU32;
use std::time::Duration;

/// Type alias for the rate limiter used throughout core.
pub type ApiRateLimiter = RateLimiter<NotKeyed, InMemoryState, QuantaClock>;

/// Create a rate limiter that allows `requests_per_minute` requests per minute.
///
/// # Panics
///
/// Panics if `requests_per_minute` is zero.
pub fn create_rate_limiter(requests_per_minute: u32) -> ApiRateLimiter {
    RateLimiter::direct(Quota::per_minute(
        NonZeroU32::new(requests_per_minute).expect("rate limit must be > 0"),
    ))
}

/// Wait until a rate limiter permit is available, with jitter to avoid thundering herd.
///
/// Applies up to 100ms of random jitter before the permit becomes available.
pub async fn wait_for_permit(limiter: &ApiRateLimiter) {
    limiter
        .until_ready_with_jitter(Jitter::up_to(Duration::from_millis(100)))
        .await;
}
