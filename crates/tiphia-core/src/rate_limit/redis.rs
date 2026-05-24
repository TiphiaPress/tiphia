use crate::error::{AppError, AppResult};
use redis::AsyncCommands;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

pub type RedisRateLimiter = Arc<Mutex<redis::aio::ConnectionManager>>;

pub async fn connect(redis_url: &str) -> AppResult<RedisRateLimiter> {
    let client = redis::Client::open(redis_url)
        .map_err(|err| AppError::RateLimitBackend(err.to_string()))?;
    let manager = client
        .get_connection_manager()
        .await
        .map_err(|err| AppError::RateLimitBackend(err.to_string()))?;
    Ok(Arc::new(Mutex::new(manager)))
}

pub async fn check(
    manager: RedisRateLimiter,
    key: String,
    limit: u32,
    window: Duration,
) -> AppResult<()> {
    let redis_key = format!("tiphia:rate-limit:{key}");
    let mut connection = manager.lock().await;
    let count: u32 = connection
        .incr(&redis_key, 1_u32)
        .await
        .map_err(|err| AppError::RateLimitBackend(err.to_string()))?;

    if count == 1 {
        let _: () = connection
            .expire(&redis_key, window.as_secs() as i64)
            .await
            .map_err(|err| AppError::RateLimitBackend(err.to_string()))?;
    }

    if count > limit {
        return Err(AppError::RateLimited);
    }

    Ok(())
}
