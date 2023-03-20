use std::path::PathBuf;

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

static INATURALIST_CACHE: Lazy<sled::Db> = Lazy::new(|| {
    let cache_dir = dirs::cache_dir()
        .map(|p| p.join(PathBuf::from("MacDive Buddy")))
        .expect("Could not determine cache directory");

    std::fs::create_dir_all(&cache_dir).expect("Could not create cache directory");

    sled::open(cache_dir.join("iNaturalist")).expect("Failed to open iNaturalist Cache")
});