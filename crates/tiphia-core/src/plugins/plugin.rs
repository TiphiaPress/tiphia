use crate::{
    error::AppResult,
    migration::SharedMigration,
    plugins::{AdminMenuItem, Hook, HookContext, HookMap, PluginConfigSchema, PluginManifest},
};
use async_trait::async_trait;
use axum::Router;
use sea_orm::DatabaseConnection;

#[async_trait]
pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &'static PluginManifest;

    async fn install(&self, _db: &DatabaseConnection) -> AppResult<()> {
        Ok(())
    }

    fn migrations(&self) -> Vec<SharedMigration> {
        Vec::new()
    }

    fn hooks(&self) -> HookMap {
        HookMap::default()
    }

    fn admin_menu(&self) -> Vec<AdminMenuItem> {
        Vec::new()
    }

    fn config_schema(&self) -> Option<PluginConfigSchema> {
        None
    }

    async fn activate(&self) -> AppResult<()> {
        Ok(())
    }

    async fn handle(&self, _hook: Hook, _context: &mut HookContext) -> AppResult<()> {
        Ok(())
    }

    fn route_prefix(&self) -> Option<&'static str> {
        None
    }

    fn route_router(&self) -> Option<Router<crate::app::AppState>> {
        None
    }

    fn routes(&self, router: Router<crate::app::AppState>) -> Router<crate::app::AppState> {
        router
    }
}
