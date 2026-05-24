use crate::{
    config::model::FileConfig,
    error::{AppError, AppResult},
};
use std::path::PathBuf;

pub(super) fn load_file_config() -> AppResult<FileConfig> {
    let path = config_path();
    if !path.exists() {
        return Ok(FileConfig::default());
    }

    let content = std::fs::read_to_string(&path)?;
    toml::from_str(&content).map_err(|err| AppError::Config(err.to_string()))
}

fn config_path() -> PathBuf {
    std::env::var("TIPHIA_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("tiphia.toml"))
}
