pub mod posts {
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
}

pub mod post_revisions {
    use chrono::{DateTime, Utc};
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, ToSchema)]
    #[sea_orm(table_name = "post_revisions")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub post_id: i32,
        pub title: String,
        pub markdown: String,
        pub html: String,
        pub excerpt: Option<String>,
        pub status: super::posts::PostStatus,
        pub author_id: i32,
        pub created_at: DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::posts::Entity",
            from = "Column::PostId",
            to = "super::posts::Column::Id"
        )]
        Post,
    }

    impl Related<super::posts::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Post.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod terms {
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
}

pub mod post_terms {
    use chrono::{DateTime, Utc};
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, ToSchema)]
    #[sea_orm(table_name = "post_terms")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub post_id: i32,
        pub term_id: i32,
        pub created_at: DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::posts::Entity",
            from = "Column::PostId",
            to = "super::posts::Column::Id"
        )]
        Post,
        #[sea_orm(
            belongs_to = "super::terms::Entity",
            from = "Column::TermId",
            to = "super::terms::Column::Id"
        )]
        Term,
    }

    impl Related<super::posts::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Post.def()
        }
    }

    impl Related<super::terms::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Term.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod comments {
    use chrono::{DateTime, Utc};
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, ToSchema)]
    #[sea_orm(table_name = "comments")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub post_id: i32,
        pub parent_id: Option<i32>,
        pub author_name: String,
        pub author_email: String,
        pub author_url: Option<String>,
        #[serde(skip_serializing)]
        #[schema(ignore)]
        pub ip_hash: Option<String>,
        #[serde(skip_serializing)]
        #[schema(ignore)]
        pub user_agent: Option<String>,
        pub content: String,
        pub status: CommentStatus,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }

    #[derive(
        Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
    )]
    #[serde(rename_all = "snake_case")]
    #[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
    pub enum CommentStatus {
        #[sea_orm(string_value = "pending")]
        Pending,
        #[sea_orm(string_value = "approved")]
        Approved,
        #[sea_orm(string_value = "spam")]
        Spam,
        #[sea_orm(string_value = "trash")]
        Trash,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::posts::Entity",
            from = "Column::PostId",
            to = "super::posts::Column::Id"
        )]
        Post,
    }

    impl Related<super::posts::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Post.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod users {
    use chrono::{DateTime, Utc};
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, ToSchema)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        #[sea_orm(unique)]
        pub username: String,
        #[sea_orm(unique)]
        pub email: String,
        pub password_hash: String,
        pub display_name: String,
        pub role: UserRole,
        pub status: UserStatus,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }

    #[derive(
        Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
    )]
    #[serde(rename_all = "snake_case")]
    #[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
    pub enum UserRole {
        #[sea_orm(string_value = "root")]
        Root,
        #[sea_orm(string_value = "admin")]
        Admin,
        #[sea_orm(string_value = "editor")]
        Editor,
        #[sea_orm(string_value = "author")]
        Author,
    }

    #[derive(
        Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
    )]
    #[serde(rename_all = "snake_case")]
    #[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
    pub enum UserStatus {
        #[sea_orm(string_value = "active")]
        Active,
        #[sea_orm(string_value = "disabled")]
        Disabled,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod options {
    use chrono::{DateTime, Utc};
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "options")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        #[sea_orm(unique)]
        pub key: String,
        pub value: Json,
        pub autoload: bool,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
