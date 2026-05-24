use crate::error::AppResult;
use sea_orm::{ConnectionTrait, DatabaseTransaction, DbBackend, FromQueryResult, Statement};

#[derive(Debug, FromQueryResult)]
struct ExistsRow {
    exists: i32,
}

pub async fn column_exists(db: &DatabaseTransaction, table: &str, column: &str) -> AppResult<bool> {
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
