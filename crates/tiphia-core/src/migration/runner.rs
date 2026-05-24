use crate::{
    error::{AppError, AppResult},
    migration::{SharedMigration, core::core_migrations, entity, schema::ensure_migration_table},
    plugins::Plugin,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, QueryOrder, Set, TransactionTrait,
};
use tracing::info;

pub async fn run_core_migrations(db: &DatabaseConnection) -> AppResult<()> {
    ensure_migration_table(db).await?;
    run_migrations(db, core_migrations()).await
}

pub async fn run_plugin_migrations(db: &DatabaseConnection, plugin: &dyn Plugin) -> AppResult<()> {
    ensure_migration_table(db).await?;
    run_migrations(db, plugin.migrations()).await
}

pub async fn run_migrations(
    db: &DatabaseConnection,
    migrations: Vec<SharedMigration>,
) -> AppResult<()> {
    for migration in migrations {
        if has_migration_run(db, migration.id()).await? {
            continue;
        }

        let transaction = db.begin().await?;
        migration.up(&transaction).await.map_err(|err| {
            AppError::Config(format!("migration {} failed: {err}", migration.id()))
        })?;
        record_migration(&transaction, migration.id())
            .await
            .map_err(|err| {
                AppError::Config(format!(
                    "failed to record migration {}: {err}",
                    migration.id()
                ))
            })?;
        transaction.commit().await.map_err(|err| {
            AppError::Config(format!(
                "failed to commit migration {}: {err}",
                migration.id()
            ))
        })?;
        info!(migration = migration.id(), "migration applied");
    }

    Ok(())
}

pub async fn rollback_last_migration(
    db: &DatabaseConnection,
    migrations: Vec<SharedMigration>,
) -> AppResult<Option<String>> {
    ensure_migration_table(db).await?;

    let Some(applied) = entity::Entity::find()
        .order_by_desc(entity::Column::AppliedAt)
        .one(db)
        .await?
    else {
        return Ok(None);
    };

    let migration = migrations
        .into_iter()
        .find(|migration| migration.id() == applied.id)
        .ok_or_else(|| {
            AppError::Config(format!(
                "migration {} is applied but not present in provided migration list",
                applied.id
            ))
        })?;
    let migration_id = migration.id();

    let transaction = db.begin().await?;
    migration.down(&transaction).await.map_err(|err| {
        AppError::Config(format!("rollback migration {migration_id} failed: {err}"))
    })?;
    entity::Entity::delete_by_id(migration_id.to_owned())
        .exec(&transaction)
        .await
        .map_err(|err| {
            AppError::Config(format!(
                "failed to remove migration record {migration_id}: {err}"
            ))
        })?;
    transaction.commit().await.map_err(|err| {
        AppError::Config(format!(
            "failed to commit rollback migration {migration_id}: {err}"
        ))
    })?;

    info!(migration = migration_id, "migration rolled back");
    Ok(Some(migration_id.to_owned()))
}

async fn has_migration_run(db: &DatabaseConnection, id: &str) -> AppResult<bool> {
    Ok(entity::Entity::find_by_id(id.to_owned())
        .one(db)
        .await?
        .is_some())
}

async fn record_migration(db: &sea_orm::DatabaseTransaction, id: &str) -> AppResult<()> {
    entity::ActiveModel {
        id: Set(id.to_owned()),
        applied_at: Set(Utc::now()),
    }
    .insert(db)
    .await?;
    Ok(())
}
