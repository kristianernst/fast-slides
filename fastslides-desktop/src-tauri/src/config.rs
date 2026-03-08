use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct AppConfig {
    #[serde(default)]
    pub(crate) projects_roots: Vec<String>,
    #[serde(default)]
    pub(crate) recent_projects: Vec<String>,
    #[serde(default)]
    pub(crate) pinned_projects: Vec<String>,
}

pub(crate) fn expand_user_path(raw: &str) -> PathBuf {
    if let Some(remainder) = raw.strip_prefix("~/") {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home).join(remainder);
        }
    }
    PathBuf::from(raw)
}

fn ensure_fastslides_home() -> Result<PathBuf, String> {
    let root = if let Ok(explicit) = env::var("FASTSLIDES_HOME") {
        expand_user_path(&explicit)
    } else if let Ok(home) = env::var("HOME") {
        PathBuf::from(home).join(".fastslides")
    } else {
        return Err("Unable to resolve FASTSLIDES_HOME or HOME.".to_string());
    };

    fs::create_dir_all(&root)
        .map_err(|error| format!("Failed to create config folder {}: {error}", root.display()))?;
    Ok(root)
}

fn config_file_path() -> Result<PathBuf, String> {
    Ok(ensure_fastslides_home()?.join("config.json"))
}

pub(crate) fn normalize_existing_directory(path_str: &str) -> Result<PathBuf, String> {
    let expanded = expand_user_path(path_str);
    if !expanded.exists() {
        return Err(format!("Path does not exist: {}", expanded.display()));
    }
    if !expanded.is_dir() {
        return Err(format!("Path is not a directory: {}", expanded.display()));
    }
    expanded
        .canonicalize()
        .map_err(|error| format!("Failed to canonicalize {}: {error}", expanded.display()))
}

pub(crate) fn normalize_existing_project_directory(path_str: &str) -> Result<PathBuf, String> {
    let project_dir = normalize_existing_directory(path_str)?;
    let page_path = project_dir.join("page.mdx");
    if !page_path.exists() || !page_path.is_file() {
        return Err(format!(
            "Project folder must contain page.mdx: {}",
            page_path.display()
        ));
    }
    Ok(project_dir)
}

pub(crate) fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub(crate) fn load_config() -> Result<AppConfig, String> {
    let config_file = config_file_path()?;
    if !config_file.exists() {
        if let Ok(raw) = env::var("FASTSLIDES_PROJECTS_DIR") {
            if let Ok(path) = normalize_existing_directory(&raw) {
                return Ok(AppConfig {
                    projects_roots: vec![path_to_string(&path)],
                    recent_projects: Vec::new(),
                    pinned_projects: Vec::new(),
                });
            }
        }
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(&config_file)
        .map_err(|error| format!("Failed to read {}: {error}", config_file.display()))?;

    serde_json::from_str::<AppConfig>(&content)
        .map_err(|error| format!("Invalid config JSON in {}: {error}", config_file.display()))
}

pub(crate) fn save_config(config: &AppConfig) -> Result<(), String> {
    let config_file = config_file_path()?;
    let json = serde_json::to_string_pretty(config)
        .map_err(|error| format!("Config serialization failed: {error}"))?;
    fs::write(&config_file, json)
        .map_err(|error| format!("Failed to write {}: {error}", config_file.display()))
}

pub(crate) fn normalized_config(mut config: AppConfig) -> AppConfig {
    let mut deduped_roots = Vec::<String>::new();
    let mut seen_roots = HashSet::<String>::new();
    for root in config.projects_roots.drain(..) {
        if let Ok(canonical) = normalize_existing_directory(&root) {
            let canonical_str = path_to_string(&canonical);
            if seen_roots.insert(canonical_str.clone()) {
                deduped_roots.push(canonical_str);
            }
        }
    }

    let mut deduped_recent = Vec::<String>::new();
    let mut seen_recent = HashSet::<String>::new();
    for project in config.recent_projects.drain(..) {
        if let Ok(canonical) = normalize_existing_project_directory(&project) {
            let canonical_str = path_to_string(&canonical);
            if seen_recent.insert(canonical_str.clone()) {
                deduped_recent.push(canonical_str);
            }
        }
    }

    let mut deduped_pinned = Vec::<String>::new();
    let mut seen_pinned = HashSet::<String>::new();
    for project in config.pinned_projects.drain(..) {
        if let Ok(canonical) = normalize_existing_project_directory(&project) {
            let canonical_str = path_to_string(&canonical);
            if seen_pinned.insert(canonical_str.clone()) {
                deduped_pinned.push(canonical_str);
            }
        }
    }

    AppConfig {
        projects_roots: deduped_roots,
        recent_projects: deduped_recent,
        pinned_projects: deduped_pinned,
    }
}
