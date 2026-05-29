use directories::ProjectDirs;
use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    ProjectDirs::from("dev", "michioxd", "mu2vid")
        .unwrap()
        .config_dir()
        .to_path_buf()
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.toml")
}
