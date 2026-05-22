use crate::{
    error::{AppError, AppResult},
    plugins::Plugin,
};
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::sea_query::{Alias, Table};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, FromQueryResult,
    QueryOrder, Schema, Set, TransactionTrait,
};
use sea_orm::{DbBackend, Statement};
use sea_orm::{DeriveEntityModel, DeriveRelation, EnumIter, entity::prelude::*};
use tracing::info;

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

    let Some(applied) = Entity::find()
        .order_by_desc(Column::AppliedAt)
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
    Entity::delete_by_id(migration_id.to_owned())
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

fn core_migrations() -> Vec<SharedMigration> {
    vec![
        Box::new(CreateCoreTables),
        Box::new(AddUserStatus),
        Box::new(CreatePostRevisions),
    ]
}

async fn ensure_migration_table(db: &DatabaseConnection) -> AppResult<()> {
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);
    let statement = schema
        .create_table_from_entity(Entity)
        .if_not_exists()
        .to_owned();
    db.execute(backend.build(&statement)).await?;
    Ok(())
}

async fn has_migration_run(db: &DatabaseConnection, id: &str) -> AppResult<bool> {
    Ok(Entity::find_by_id(id.to_owned()).one(db).await?.is_some())
}

async fn record_migration(db: &DatabaseTransaction, id: &str) -> AppResult<()> {
    ActiveModel {
        id: Set(id.to_owned()),
        applied_at: Set(Utc::now()),
    }
    .insert(db)
    .await?;
    Ok(())
}

struct CreateCoreTables;

#[async_trait]
impl Migration for CreateCoreTables {
    fn id(&self) -> &'static str {
        "core:0001:create-core-tables"
    }

    async fn up(&self, db: &DatabaseTransaction) -> AppResult<()> {
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);

        let statements = [
            schema
                .create_table_from_entity(crate::entities::users::Entity)
                .if_not_exists()
                .to_owned(),
            schema
                .create_table_from_entity(crate::entities::posts::Entity)
                .if_not_exists()
                .to_owned(),
            schema
                .create_table_from_entity(crate::entities::comments::Entity)
                .if_not_exists()
                .to_owned(),
            schema
                .create_table_from_entity(crate::entities::terms::Entity)
                .if_not_exists()
                .to_owned(),
            schema
                .create_table_from_entity(crate::entities::post_terms::Entity)
                .if_not_exists()
                .to_owned(),
            schema
                .create_table_from_entity(crate::entities::post_revisions::Entity)
                .if_not_exists()
                .to_owned(),
            schema
                .create_table_from_entity(crate::entities::options::Entity)
                .if_not_exists()
                .to_owned(),
        ];

        for statement in statements {
            db.execute(backend.build(&statement)).await?;
        }

        Ok(())
    }
}

struct CreatePostRevisions;

#[async_trait]
impl Migration for CreatePostRevisions {
    fn id(&self) -> &'static str {
        "core:0003:create-post-revisions"
    }

    async fn up(&self, db: &DatabaseTransaction) -> AppResult<()> {
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        let statement = schema
            .create_table_from_entity(crate::entities::post_revisions::Entity)
            .if_not_exists()
            .to_owned();
        db.execute(backend.build(&statement)).await?;
        Ok(())
    }

    async fn down(&self, db: &DatabaseTransaction) -> AppResult<()> {
        let backend = db.get_database_backend();
        let statement = Table::drop()
            .table(Alias::new("post_revisions"))
            .if_exists()
            .to_owned();
        db.execute(backend.build(&statement)).await?;
        Ok(())
    }
}

struct AddUserStatus;

#[async_trait]
impl Migration for AddUserStatus {
    fn id(&self) -> &'static str {
        "core:0002:add-user-status"
    }

    async fn up(&self, db: &DatabaseTransaction) -> AppResult<()> {
        if column_exists(db, "users", "status").await? {
            return Ok(());
        }

        match db.get_database_backend() {
            DbBackend::Sqlite => {
                db.execute(Statement::from_string(
                    DbBackend::Sqlite,
                    "ALTER TABLE users ADD COLUMN status varchar(32) NOT NULL DEFAULT 'active'",
                ))
                .await?;
            }
            DbBackend::Postgres => {
                db.execute(Statement::from_string(
                    DbBackend::Postgres,
                    "ALTER TABLE users ADD COLUMN status varchar(32) NOT NULL DEFAULT 'active'",
                ))
                .await?;
            }
            DbBackend::MySql => {
                db.execute(Statement::from_string(
                    DbBackend::MySql,
                    "ALTER TABLE users ADD COLUMN status varchar(32) NOT NULL DEFAULT 'active'",
                ))
                .await?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, FromQueryResult)]
struct ExistsRow {
    exists: i32,
}

async fn column_exists(db: &DatabaseTransaction, table: &str, column: &str) -> AppResult<bool> {
    let backend = db.get_database_backend();
    let statement = match backend {
        DbBackend::Sqlite => Statement::from_sql_and_values(
            backend,
            "SELECT COUNT(*) AS \"exists\" FROM pragma_table_info(?) WHERE name = ?",
            [table.into(), column.into()],
        ),
        DbBackend::Postgres => Statement::from_sql_and_values(
            backend,
            "SELECT COUNT(*) AS \"exists\" FROM information_schema.columns WHERE table_name = $1 AND column_name = $2",
            [table.into(), column.into()],
        ),
        DbBackend::MySql => Statement::from_sql_and_values(
            backend,
            "SELECT COUNT(*) AS `exists` FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = ? AND column_name = ?",
            [table.into(), column.into()],
        ),
    };

    Ok(ExistsRow::find_by_statement(statement)
        .one(db)
        .await?
        .map(|row| row.exists > 0)
        .unwrap_or(false))
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "schema_migrations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub applied_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
