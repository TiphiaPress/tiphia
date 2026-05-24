use std::sync::Arc;

#[path = "plugins/config.rs"]
pub mod config;
#[path = "plugins/context.rs"]
mod context;
#[path = "plugins/hook.rs"]
mod hook;
#[path = "plugins/manifest.rs"]
mod manifest;
#[path = "plugins/plugin.rs"]
mod plugin;
#[path = "plugins/registry.rs"]
mod registry;
#[path = "plugins/schema.rs"]
mod schema;

pub use config::{
    PLUGIN_OPTION_PREFIX, default_plugin_state, ensure_plugin_config, load_plugin_config,
    load_plugin_config_with, merge_json_config, plugin_config_key, plugin_state_key,
};
pub use context::HookContext;
pub use hook::{Hook, HookMap};
pub use manifest::{AdminMenuItem, PluginManifest};
pub use plugin::Plugin;
pub use registry::{PluginRegistry, PluginRegistryBuilder};
pub use schema::{PluginConfigField, PluginConfigFieldType, PluginConfigSchema};

pub type SharedPlugin = Arc<dyn Plugin>;
