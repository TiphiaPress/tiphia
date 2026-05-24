use crate::error::{AppError, AppResult};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub struct MemoryRateLimiter {
    buckets: Arc<Mutex<HashMap<String, Bucket>>>,
}

#[derive(Clone)]
struct Bucket {
    count: u32,
    reset_at: Instant,
}

impl MemoryRateLimiter {
    pub async fn check(&self, key: String, limit: u32, window: Duration) -> AppResult<()> {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().await;
        let bucket = buckets.entry(key).or_insert_with(|| Bucket {
            count: 0,
            reset_at: now + window,
        });

        if now >= bucket.reset_at {
            bucket.count = 0;
            bucket.reset_at = now + window;
        }

        if bucket.count >= limit {
            return Err(AppError::RateLimited);
        }

        bucket.count += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_limiter_blocks_after_limit() {
        let limiter = MemoryRateLimiter::default();
        assert!(
            limiter
                .check("k".to_owned(), 1, Duration::from_secs(60))
                .await
                .is_ok()
        );
        assert!(
            limiter
                .check("k".to_owned(), 1, Duration::from_secs(60))
                .await
                .is_err()
        );
    }
}
