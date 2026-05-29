use std::fs;

use super::{app::AppConfig, paths::config_file};

pub fn load() -> AppConfig {
    let path = config_file();

    let Ok(text) = fs::read_to_string(path) else {
        return AppConfig::default();
    };

    toml::from_str(&text).unwrap_or_default()
}

pub fn save(config: &AppConfig) -> anyhow::Result<()> {
    let path = config_file();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let text = toml::to_string_pretty(config)?;

    fs::write(path, text)?;

    Ok(())
}
