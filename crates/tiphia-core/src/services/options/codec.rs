use crate::error::{AppError, AppResult};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

pub fn from_json<T>(value: Value) -> AppResult<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value).map_err(|err| AppError::Validation(err.to_string()))
}

pub fn to_json<T>(value: &T) -> AppResult<Value>
where
    T: Serialize,
{
    serde_json::to_value(value).map_err(|err| AppError::Validation(err.to_string()))
}
