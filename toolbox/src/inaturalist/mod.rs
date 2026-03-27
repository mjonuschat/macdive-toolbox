use macdive_toolbox_core::util::rate_limit::{ApiRateLimiter, create_rate_limiter};
use std::sync::LazyLock;

mod helpers;
pub(crate) mod types;

pub use helpers::*;
pub use types::*;

static INAT_API_LIMIT: LazyLock<ApiRateLimiter> = LazyLock::new(|| create_rate_limiter(60));
