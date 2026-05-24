use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use sea_orm::DatabaseTransaction;

pub type SharedMigration = Box<dyn Migration>;

#[async_trait]
pub trait Migration: Send + Sync {
    fn id(&self) -> &'static str;
    async fn up(&self, db: &DatabaseTransaction) -> AppResult<()>;

    async fn down(&self, _db: &DatabaseTransaction) -> AppResult<()> {
        Err(AppError::Config(format!(
            "migration {} does not support rollback",
            self.id()
        )))
    }
}
