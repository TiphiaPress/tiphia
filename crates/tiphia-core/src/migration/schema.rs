use crate::{error::AppResult, migration::entity::Entity};
use sea_orm::{ConnectionTrait, DatabaseConnection, Schema};

pub async fn ensure_migration_table(db: &DatabaseConnection) -> AppResult<()> {
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);
    let statement = schema
        .create_table_from_entity(Entity)
        .if_not_exists()
        .to_owned();
    db.execute(backend.build(&statement)).await?;
    Ok(())
}
