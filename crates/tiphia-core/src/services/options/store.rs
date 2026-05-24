use crate::{
    app::AppState,
    entities::options,
    error::{AppResult, validation_on_unique},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::Value;

pub async fn find_value(state: &AppState, key: &str) -> AppResult<Option<Value>> {
    Ok(options::Entity::find()
        .filter(options::Column::Key.eq(key))
        .one(&state.db)
        .await?
        .map(|option| option.value))
}

pub async fn upsert_value(
    state: &AppState,
    key: &str,
    value: Value,
    autoload: bool,
) -> AppResult<()> {
    let now = Utc::now();
    if let Some(existing) = options::Entity::find()
        .filter(options::Column::Key.eq(key))
        .one(&state.db)
        .await?
    {
        let mut model: options::ActiveModel = existing.into();
        model.value = Set(value);
        model.autoload = Set(autoload);
        model.updated_at = Set(now);
        model.update(&state.db).await?;
    } else {
        options::ActiveModel {
            key: Set(key.to_owned()),
            value: Set(value),
            autoload: Set(autoload),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&state.db)
        .await
        .map_err(|err| validation_on_unique(err, "option already exists"))?;
    }
    Ok(())
}
