#[path = "migration/core.rs"]
mod core;
#[path = "migration/entity.rs"]
pub mod entity;
#[path = "migration/introspection.rs"]
mod introspection;
#[path = "migration/trait.rs"]
mod migration_trait;
#[path = "migration/runner.rs"]
mod runner;
#[path = "migration/schema.rs"]
mod schema;

pub use entity::{ActiveModel, Column, Entity, Model, Relation};
pub use migration_trait::{Migration, SharedMigration};
pub use runner::{
    rollback_last_migration, run_core_migrations, run_migrations, run_plugin_migrations,
};
