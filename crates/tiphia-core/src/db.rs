use crate::{config::DatabaseConfig, error::AppResult, migration::run_core_migrations};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

pub async fn connect_database(config: &DatabaseConfig) -> AppResult<DatabaseConnection> {
    let mut options = ConnectOptions::new(config.url.clone());
    options
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect_timeout(config.connect_timeout)
        .acquire_timeout(config.acquire_timeout);

    let db = Database::connect(options).await?;
    run_core_migrations(&db).await?;
    Ok(db)
}
