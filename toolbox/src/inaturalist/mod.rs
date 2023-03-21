use governor::clock::QuantaClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use nonzero_ext::nonzero;
use once_cell::sync::Lazy;

mod helpers;
pub(crate) mod types;

pub use helpers::*;
pub use types::*;

static INAT_API_LIMIT: Lazy<RateLimiter<NotKeyed, InMemoryState, QuantaClock>> =
    Lazy::new(|| RateLimiter::direct(Quota::per_minute(nonzero!(60u32))));
