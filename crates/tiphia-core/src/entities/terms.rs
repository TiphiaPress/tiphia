use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, ToSchema)]
#[sea_orm(table_name = "terms")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub term_type: TermType,
    pub parent_id: Option<i32>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum TermType {
    #[sea_orm(string_value = "category")]
    Category,
    #[sea_orm(string_value = "tag")]
    Tag,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::post_terms::Entity")]
    PostTerms,
}

impl Related<super::post_terms::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PostTerms.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
