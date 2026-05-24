use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub author: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminMenuItem {
    pub label: &'static str,
    pub path: &'static str,
    pub icon: Option<&'static str>,
    pub order: i32,
}
