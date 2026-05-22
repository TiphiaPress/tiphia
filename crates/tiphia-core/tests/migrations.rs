mod support;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use sea_orm::{ConnectionTrait, DatabaseTransaction, Statement};
use tiphia_core::{
    AppResult,
    entities::users,
    migration::{self, Migration},
};

#[tokio::test]
async fn core_migrations_are_idempotent() {
    let db = support::database().await;

    migration::run_core_migrations(&db)
        .await
        .expect("rerun migrations");

    let applied = migration::Entity::find()
        .filter(migration::Column::Id.eq("core:0001:create-core-tables"))
        .one(&db)
        .await
        .expect("query migration")
        .expect("migration exists");
    assert_eq!(applied.id, "core:0001:create-core-tables");

    let user_columns_exist = users::Entity::find().one(&db).await.is_ok();
    assert!(user_columns_exist);
}

#[tokio::test]
async fn rollback_last_migration_runs_down_and_removes_record() {
    let db = support::database().await;

    migration::run_migrations(&db, vec![Box::new(CreateTemporaryTable)])
        .await
        .expect("run temp migration");
    assert!(temporary_table_exists(&db).await);

    let rolled_back = migration::rollback_last_migration(&db, vec![Box::new(CreateTemporaryTable)])
        .await
        .expect("rollback temp migration");

    assert_eq!(
        rolled_back.as_deref(),
        Some("test:0001:create-temporary-table")
    );
    assert!(!temporary_table_exists(&db).await);
}

struct CreateTemporaryTable;

#[async_trait]
impl Migration for CreateTemporaryTable {
    fn id(&self) -> &'static str {
        "test:0001:create-temporary-table"
    }

    async fn up(&self, db: &DatabaseTransaction) -> AppResult<()> {
        db.execute(Statement::from_string(
            db.get_database_backend(),
            "CREATE TABLE temporary_migration_test (id integer primary key)",
        ))
        .await?;
        Ok(())
    }

    async fn down(&self, db: &DatabaseTransaction) -> AppResult<()> {
        db.execute(Statement::from_string(
            db.get_database_backend(),
            "DROP TABLE temporary_migration_test",
        ))
        .await?;
        Ok(())
    }
}

async fn temporary_table_exists(db: &sea_orm::DatabaseConnection) -> bool {
    db.execute(Statement::from_string(
        db.get_database_backend(),
        "SELECT 1 FROM temporary_migration_test LIMIT 1",
    ))
    .await
    .is_ok()
}
