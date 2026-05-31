use std::path::Path;

pub const MAX_RECENT_PROJECTS: usize = 10;

pub fn add_recent_project(recent_projects: &mut Vec<String>, path: &Path) {
    let path = path.to_string_lossy().to_string();
    recent_projects.retain(|item| !item.eq_ignore_ascii_case(&path));
    recent_projects.insert(0, path);
    recent_projects.truncate(MAX_RECENT_PROJECTS);
}
