use crate::{
    error::AppResult,
    migration::{SharedMigration, introspection::column_exists},
};
use async_trait::async_trait;
use sea_orm::sea_query::{Alias, Table};
use sea_orm::{ConnectionTrait, DatabaseTransaction, DbBackend, Schema, Statement};

pub fn core_migrations() -> Vec<SharedMigration> {
    vec![
        Box::new(CreateCoreTables),
        Box::new(AddUserStatus),
        Box::new(CreatePostRevisions),
    ]
}

struct CreateCoreTables;

#[async_trait]
impl crate::migration::Migration for CreateCoreTables {
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
impl crate::migration::Migration for CreatePostRevisions {
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
impl crate::migration::Migration for AddUserStatus {
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
