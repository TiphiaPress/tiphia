use crate::{app::AppState, error::AppResult};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

#[path = "options/codec.rs"]
mod codec;
#[path = "options/store.rs"]
mod store;

pub async fn get_json(state: &AppState, key: &str) -> AppResult<Option<Value>> {
    store::find_value(state, key).await
}

pub async fn get_typed<T>(state: &AppState, key: &str) -> AppResult<Option<T>>
where
    T: DeserializeOwned,
{
    get_json(state, key)
        .await?
        .map(codec::from_json)
        .transpose()
}

pub async fn upsert_json(
    state: &AppState,
    key: &str,
    value: Value,
    autoload: bool,
) -> AppResult<()> {
    store::upsert_value(state, key, value, autoload).await
}

pub async fn update_json<F>(
    state: &AppState,
    key: &str,
    autoload: bool,
    update: F,
) -> AppResult<Value>
where
    F: FnOnce(Option<Value>) -> Value,
{
    let current = get_json(state, key).await?;
    let next = update(current);
    upsert_json(state, key, next.clone(), autoload).await?;
    Ok(next)
}

pub async fn upsert_typed<T>(
    state: &AppState,
    key: &str,
    value: &T,
    autoload: bool,
) -> AppResult<()>
where
    T: Serialize,
{
    upsert_json(state, key, codec::to_json(value)?, autoload).await
}
