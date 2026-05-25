use crate::error::{AppError, AppResult};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct HookContext {
    db: Option<DatabaseConnection>,
    pub subject: Option<Value>,
    pub meta: BTreeMap<String, Value>,
    pub stopped: bool,
    pub stop_reason: Option<String>,
}

impl HookContext {
    pub fn with_subject<T>(subject: T) -> AppResult<Self>
    where
        T: Serialize,
    {
        Ok(Self {
            db: None,
            subject: Some(
                serde_json::to_value(subject).map_err(|err| AppError::Plugin(err.to_string()))?,
            ),
            meta: BTreeMap::new(),
            stopped: false,
            stop_reason: None,
        })
    }

    pub fn attach_database(&mut self, db: DatabaseConnection) {
        self.db = Some(db);
    }

    pub fn database(&self) -> AppResult<&DatabaseConnection> {
        self.db
            .as_ref()
            .ok_or_else(|| AppError::Plugin("hook context has no database".to_owned()))
    }

    pub fn subject_as<T>(&self) -> AppResult<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.subject
            .clone()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|err| AppError::Plugin(err.to_string()))
    }

    pub fn take_subject<T>(&mut self) -> AppResult<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.subject
            .take()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|err| AppError::Plugin(err.to_string()))
    }

    pub fn replace_subject<T>(&mut self, subject: T) -> AppResult<()>
    where
        T: Serialize,
    {
        self.subject =
            Some(serde_json::to_value(subject).map_err(|err| AppError::Plugin(err.to_string()))?);
        Ok(())
    }

    pub fn insert_meta<T>(&mut self, key: impl Into<String>, value: T) -> AppResult<()>
    where
        T: Serialize,
    {
        self.meta.insert(
            key.into(),
            serde_json::to_value(value).map_err(|err| AppError::Plugin(err.to_string()))?,
        );
        Ok(())
    }

    pub fn meta_as<T>(&self, key: &str) -> AppResult<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.meta
            .get(key)
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|err| AppError::Plugin(err.to_string()))
    }

    pub fn stop(&mut self, reason: impl Into<String>) {
        self.stopped = true;
        self.stop_reason = Some(reason.into());
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    pub fn ensure_not_stopped(&self) -> AppResult<()> {
        if self.stopped {
            return Err(AppError::Plugin(
                self.stop_reason
                    .clone()
                    .unwrap_or_else(|| "hook stopped execution".to_owned()),
            ));
        }

        Ok(())
    }
}
