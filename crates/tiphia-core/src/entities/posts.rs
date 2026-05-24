use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, ToSchema)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub slug: String,
    pub title: String,
    pub markdown: String,
    pub html: String,
    pub excerpt: Option<String>,
    pub status: PostStatus,
    pub post_type: PostType,
    pub author_id: i32,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum PostStatus {
    #[sea_orm(string_value = "draft")]
    Draft,
    #[sea_orm(string_value = "pending_review")]
    PendingReview,
    #[sea_orm(string_value = "published")]
    Published,
    #[sea_orm(string_value = "scheduled")]
    Scheduled,
    #[sea_orm(string_value = "archived")]
    Archived,
}

#[derive(
    Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum PostType {
    #[sea_orm(string_value = "post")]
    Post,
    #[sea_orm(string_value = "page")]
    Page,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::comments::Entity")]
    Comments,
    #[sea_orm(has_many = "super::post_terms::Entity")]
    PostTerms,
}

impl Related<super::comments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Comments.def()
    }
}

impl Related<super::post_terms::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PostTerms.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
