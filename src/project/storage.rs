use std::fs;
use std::path::Path;

use super::model::ProjectFile;

pub fn load(path: &Path) -> anyhow::Result<ProjectFile> {
	let text = fs::read_to_string(path)?;
	Ok(toml::from_str(&text)?)
}

pub fn save(path: &Path, project: &ProjectFile) -> anyhow::Result<()> {
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}

	let text = toml::to_string_pretty(project)?;
	fs::write(path, text)?;

	Ok(())
}
