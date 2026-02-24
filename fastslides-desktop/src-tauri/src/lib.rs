use base64::Engine;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::Cursor;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::{Emitter, Manager};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use url::{form_urlencoded::Serializer as UrlQuerySerializer, Url};

#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

const PROJECT_NAME_PATTERN: &str = r"^[A-Za-z0-9._-]+$";
const DEFAULT_TITLE: &str = "Presentation";
const DEFAULT_SUBTITLE: &str = "Project Overview";
const DEFAULT_DATE_LABEL: &str = "Month YYYY";
const DEFAULT_PREVIEW_BASE_URL: &str = "http://127.0.0.1:34773";
const DEFAULT_AGENT_HOOK_ADDR: &str = "127.0.0.1:38473";
const MENU_EXPORT_SKILL_ID: &str = "menu.export_fastslides_skill";
const MENU_EXPORT_SKILL_EVENT: &str = "fastslides://export-skill";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppConfig {
    #[serde(default)]
    projects_roots: Vec<String>,
    #[serde(default)]
    recent_projects: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ProjectSummary {
    name: String,
    path: String,
    root: String,
    slide_count: usize,
    updated_at: u64,
}

#[derive(Debug, Clone, Serialize)]
struct ProjectDetail {
    name: String,
    path: String,
    root: String,
    page_mdx: String,
    slide_count: usize,
    updated_at: u64,
}

#[derive(Debug, Clone, Serialize)]
struct ValidationReport {
    path: String,
    slide_count: usize,
    assets_checked: usize,
    errors: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AppState {
    config: AppConfig,
    projects: Vec<ProjectSummary>,
}

#[derive(Debug, Deserialize)]
struct PathPayload {
    path: String,
}

#[derive(Debug, Serialize)]
struct HookStatus {
    ok: bool,
    service: String,
}

#[derive(Debug, Serialize)]
struct HookError {
    ok: bool,
    error: String,
}

#[derive(Debug, Serialize)]
struct PreviewUrlResponse {
    ok: bool,
    preview_url: String,
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn project_name_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(PROJECT_NAME_PATTERN).expect("invalid project name regex"))
}

fn slide_start_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?i)<section\s+className=["']slide["']\s*>"#).expect("invalid slide regex")
    })
}

fn markdown_link_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"!\[[^\]]*\]\(([^)]+)\)|\[[^\]]*\]\(([^)]+)\)"#)
            .expect("invalid mdx link regex")
    })
}

fn attr_link_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?:src|href|poster)\s*=\s*["']([^"']+)["']"#)
            .expect("invalid attr link regex")
    })
}

fn word_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"[A-Za-z0-9][A-Za-z0-9'./-]*"#).expect("invalid word regex"))
}

fn bullet_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?m)^\s*(?:[-*+]\s+|\d+\.\s+)"#).expect("invalid bullet regex"))
}

fn import_export_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*(import|export)\s+"#).expect("invalid import/export regex")
    })
}

fn use_client_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*["']use client["']\s*;?\s*$"#).expect("invalid use-client regex")
    })
}

fn html_tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"<[^>]+>"#).expect("invalid html tag regex"))
}

fn frontmatter_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)\A---\s*\n(.*?)\n---\s*(?:\n|$)"#).expect("invalid frontmatter regex")
    })
}

fn frontmatter_line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*([A-Za-z0-9_-]+)\s*:\s*(.*?)\s*$"#)
            .expect("invalid frontmatter line regex")
    })
}

fn expand_user_path(raw: &str) -> PathBuf {
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

fn normalize_existing_directory(path_str: &str) -> Result<PathBuf, String> {
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

fn normalize_existing_project_directory(path_str: &str) -> Result<PathBuf, String> {
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

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn load_config() -> Result<AppConfig, String> {
    let config_file = config_file_path()?;
    if !config_file.exists() {
        if let Ok(raw) = env::var("FASTSLIDES_PROJECTS_DIR") {
            if let Ok(path) = normalize_existing_directory(&raw) {
                return Ok(AppConfig {
                    projects_roots: vec![path_to_string(&path)],
                    recent_projects: Vec::new(),
                });
            }
        }
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(&config_file)
        .map_err(|error| format!("Failed to read {}: {error}", config_file.display()))?;

    match serde_json::from_str::<AppConfig>(&content) {
        Ok(config) => Ok(config),
        Err(error) => Err(format!(
            "Invalid config JSON in {}: {error}",
            config_file.display()
        )),
    }
}

fn save_config(config: &AppConfig) -> Result<(), String> {
    let config_file = config_file_path()?;
    let json = serde_json::to_string_pretty(config)
        .map_err(|error| format!("Config serialization failed: {error}"))?;
    fs::write(&config_file, json)
        .map_err(|error| format!("Failed to write {}: {error}", config_file.display()))
}

fn normalized_config(mut config: AppConfig) -> AppConfig {
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

    AppConfig {
        projects_roots: deduped_roots,
        recent_projects: deduped_recent,
    }
}

fn sanitize_markdown_target(raw: &str) -> String {
    let trimmed = raw.trim().trim_matches('<').trim_matches('>');
    if let Some(index) = trimmed.find(' ') {
        return trimmed[..index].to_string();
    }
    trimmed.to_string()
}

fn local_asset_path(raw: &str) -> Option<String> {
    let value = sanitize_markdown_target(raw);
    if value.is_empty() {
        return None;
    }

    let lower = value.to_ascii_lowercase();
    if value.starts_with('#')
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("data:")
        || lower.starts_with("blob:")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
    {
        return None;
    }

    let no_hash = value.split('#').next().unwrap_or_default();
    let no_query = no_hash.split('?').next().unwrap_or_default();
    if no_query.is_empty() {
        return None;
    }

    let normalized = no_query.replace('\\', "/");
    if normalized.starts_with('/') {
        let allowed = normalized == "/assets"
            || normalized == "/images"
            || normalized == "/media"
            || normalized == "/data"
            || normalized.starts_with("/assets/")
            || normalized.starts_with("/images/")
            || normalized.starts_with("/media/")
            || normalized.starts_with("/data/");
        if !allowed {
            return None;
        }
        return Some(normalized.trim_start_matches('/').to_string());
    }

    Some(normalized)
}

fn resolve_relative_path(base_dir: &Path, relative: &str) -> Option<PathBuf> {
    let mut output = base_dir.to_path_buf();

    for component in Path::new(relative).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => output.push(part),
            Component::ParentDir => {
                if !output.pop() {
                    return None;
                }
                if !output.starts_with(base_dir) {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(output)
}

fn mime_type_for_path(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "avif" => "image/avif",
        _ => "application/octet-stream",
    }
}

fn read_page_mdx(project_dir: &Path) -> Result<String, String> {
    let page_path = project_dir.join("page.mdx");
    fs::read_to_string(&page_path)
        .map_err(|error| format!("Failed to read {}: {error}", page_path.display()))
}

fn write_page_mdx(project_dir: &Path, content: &str) -> Result<(), String> {
    let page_path = project_dir.join("page.mdx");
    fs::write(&page_path, content)
        .map_err(|error| format!("Failed to write {}: {error}", page_path.display()))
}

fn slide_count_from_source(source: &str) -> usize {
    slide_start_re().find_iter(source).count()
}

fn extract_slides(source: &str) -> Vec<String> {
    let matches: Vec<_> = slide_start_re().find_iter(source).collect();
    if matches.is_empty() {
        return Vec::new();
    }

    let mut slides = Vec::new();
    for (index, hit) in matches.iter().enumerate() {
        let start = hit.end();
        let explicit_end = source[start..]
            .find("</section>")
            .map(|offset| start + offset);
        let fallback_end = if index + 1 < matches.len() {
            matches[index + 1].start()
        } else {
            source.len()
        };
        let end = explicit_end.unwrap_or(fallback_end);
        slides.push(source[start..end].to_string());
    }
    slides
}

fn words_in_text(text: &str) -> usize {
    let plain = html_tag_re().replace_all(text, " ");
    word_re().find_iter(&plain).count()
}

fn max_paragraph_words(text: &str) -> usize {
    let plain = html_tag_re().replace_all(text, " ");
    let mut max_words = 0usize;
    for paragraph in plain
        .split("\n\n")
        .map(|chunk| chunk.trim())
        .filter(|chunk| !chunk.is_empty())
    {
        let count = word_re().find_iter(paragraph).count();
        if count > max_words {
            max_words = count;
        }
    }
    max_words
}

fn modified_epoch_seconds(path: &Path) -> u64 {
    let modified = fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());
    modified.unwrap_or_else(now_epoch_seconds)
}

fn project_root_for_path(config: &AppConfig, project_path: &Path) -> Option<String> {
    for root in &config.projects_roots {
        let root_path = Path::new(root);
        if project_path.starts_with(root_path) {
            return Some(root.clone());
        }
    }
    None
}

fn project_root_or_parent(config: &AppConfig, project_path: &Path) -> String {
    project_root_for_path(config, project_path).unwrap_or_else(|| {
        project_path
            .parent()
            .map(path_to_string)
            .unwrap_or_else(String::new)
    })
}

fn project_summary_for(config: &AppConfig, project_dir: &Path) -> Option<ProjectSummary> {
    let page_path = project_dir.join("page.mdx");
    if !page_path.exists() || !page_path.is_file() {
        return None;
    }

    let page_source = fs::read_to_string(&page_path).ok()?;
    let slide_count = slide_count_from_source(&page_source);
    let name = project_dir.file_name()?.to_string_lossy().into_owned();

    Some(ProjectSummary {
        name,
        path: path_to_string(project_dir),
        root: project_root_or_parent(config, project_dir),
        slide_count,
        updated_at: modified_epoch_seconds(&page_path),
    })
}

fn list_projects(config: &AppConfig) -> Vec<ProjectSummary> {
    let mut seen_paths = HashSet::<String>::new();
    let mut projects = Vec::<ProjectSummary>::new();

    for project_path in &config.recent_projects {
        if let Ok(canonical_project) = normalize_existing_project_directory(project_path) {
            let canonical_str = path_to_string(&canonical_project);
            if seen_paths.insert(canonical_str) {
                if let Some(summary) = project_summary_for(config, &canonical_project) {
                    projects.push(summary);
                }
            }
        }
    }

    projects.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    projects
}

fn project_detail_for_path(
    config: &AppConfig,
    project_path: &Path,
) -> Result<ProjectDetail, String> {
    let canonical_project = normalize_existing_directory(&path_to_string(project_path))?;
    let page_mdx = read_page_mdx(&canonical_project)?;
    let slide_count = slide_count_from_source(&page_mdx);
    let page_path = canonical_project.join("page.mdx");

    let root = project_root_for_path(config, &canonical_project).unwrap_or_default();
    let name = canonical_project
        .file_name()
        .map(|item| item.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".to_string());

    Ok(ProjectDetail {
        name,
        path: path_to_string(&canonical_project),
        root,
        page_mdx,
        slide_count,
        updated_at: modified_epoch_seconds(&page_path),
    })
}

fn yaml_quote(value: &str) -> String {
    let escaped = value.replace('\\', r#"\\"#).replace('"', r#"\""#);
    format!(r#""{escaped}""#)
}

fn normalize_frontmatter_value(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() >= 2 {
        let first = trimmed.as_bytes()[0] as char;
        let last = trimmed.as_bytes()[trimmed.len() - 1] as char;
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            let inner = &trimmed[1..trimmed.len() - 1];
            let escaped_quote = format!(r#"\{first}"#);
            return inner
                .replace("\\\\", "\\")
                .replace(escaped_quote.as_str(), first.to_string().as_str())
                .trim()
                .to_string();
        }
    }
    trimmed.to_string()
}

fn extract_frontmatter(source: &str) -> (Option<HashMap<String, String>>, String) {
    let Some(captures) = frontmatter_re().captures(source) else {
        return (None, source.to_string());
    };

    let Some(full_match) = captures.get(0) else {
        return (None, source.to_string());
    };
    let block = captures.get(1).map(|item| item.as_str()).unwrap_or_default();

    let mut values = HashMap::<String, String>::new();
    for raw_line in block.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(parsed) = frontmatter_line_re().captures(line) {
            let key = parsed
                .get(1)
                .map(|item| item.as_str().to_ascii_lowercase())
                .unwrap_or_default();
            let value = parsed
                .get(2)
                .map(|item| normalize_frontmatter_value(item.as_str()))
                .unwrap_or_default();
            values.insert(key, value);
        }
    }

    (Some(values), source[full_match.end()..].to_string())
}

fn build_starter_page(project: &str, title: &str, subtitle: &str, date_label: &str) -> String {
    format!(
        r#"---
project: {project}
title: {title}
subtitle: {subtitle}
date: {date_label}
---

<main className="deck">

<section className="slide">

# {title}

<div className="flex flex-col h-full justify-center">
  <div className="text-6xl font-extrabold text-neutral-900 mb-6">{subtitle}</div>
  <div className="text-2xl text-neutral-500">{date_label}</div>
</div>

</section>

<section className="slide">

# Problem

<div className="split">
  <div className="col prose-compact">

  - Current process is fragmented across inboxes and handoffs.
  - Ownership is unclear for time-sensitive messages.
  - Manual triage creates delays and rework.

  </div>
  <div className="col prose-compact">

  - Delays reduce responsiveness and predictability.
  - Teams spend time coordinating instead of resolving.
  - Leadership lacks a clear operational signal.

  </div>
</div>

</section>

<section className="slide">

# Proposal

<div className="split">
  <div className="col prose-compact">

  1. Classify incoming messages by intent and urgency.
  2. Route each message to a clear owner.
  3. Track response timing and outcomes.

  </div>
  <div className="col prose-compact">

  ## Expected Outcome

  - Faster first response
  - Lower coordination overhead
  - Better visibility for management decisions

  </div>
</div>

</section>

</main>
"#,
        project = yaml_quote(project),
        title = yaml_quote(title),
        subtitle = yaml_quote(subtitle),
        date_label = yaml_quote(date_label)
    )
}

fn build_state() -> Result<AppState, String> {
    let config = normalized_config(load_config()?);
    let projects = list_projects(&config);
    Ok(AppState { config, projects })
}

fn remember_recent_project(config: &mut AppConfig, project_path: &Path) {
    let project_path_str = path_to_string(project_path);
    config
        .recent_projects
        .retain(|existing| existing != &project_path_str);
    config.recent_projects.insert(0, project_path_str);

    const MAX_RECENT_PROJECTS: usize = 50;
    if config.recent_projects.len() > MAX_RECENT_PROJECTS {
        config.recent_projects.truncate(MAX_RECENT_PROJECTS);
    }
}

fn validate_project_folder(project_path: &Path) -> Result<ValidationReport, String> {
    let canonical_project = normalize_existing_directory(&path_to_string(project_path))?;
    let page_path = canonical_project.join("page.mdx");
    if !page_path.exists() {
        return Err(format!("Missing page.mdx: {}", page_path.display()));
    }

    let source = read_page_mdx(&canonical_project)?;
    let (frontmatter, body) = extract_frontmatter(&source);
    let mut errors = Vec::<String>::new();
    let mut warnings = Vec::<String>::new();

    if let Some(frontmatter_values) = &frontmatter {
        if frontmatter_values
            .get("project")
            .map(|item| item.trim().is_empty())
            .unwrap_or(true)
        {
            warnings.push("Frontmatter is missing `project`.".to_string());
        }
        if frontmatter_values
            .get("title")
            .map(|item| item.trim().is_empty())
            .unwrap_or(true)
        {
            warnings.push("Frontmatter is missing `title`.".to_string());
        }

        let declared_project = frontmatter_values
            .get("project")
            .map(|item| item.trim())
            .unwrap_or_default();
        let folder_name = canonical_project
            .file_name()
            .map(|item| item.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !declared_project.is_empty() && declared_project != folder_name {
            warnings.push(format!(
                "Frontmatter project `{declared_project}` does not match folder name `{folder_name}`."
            ));
        }
    } else {
        warnings.push(
            "Missing YAML frontmatter in page.mdx. Add metadata block with project/title/subtitle/date."
                .to_string(),
        );
    }

    if import_export_re().is_match(&body) {
        errors.push("Detected import/export statements in page.mdx; runtime decks should be content-only MDX.".to_string());
    }
    if use_client_re().is_match(&body) {
        warnings.push(r#"Found "use client" directive in page.mdx; this is usually unnecessary in runtime-loaded MDX."#.to_string());
    }

    let slides = extract_slides(&body);
    if slides.is_empty() {
        errors.push(r#"No `<section className="slide">` blocks were found."#.to_string());
    }

    for (index, slide) in slides.iter().enumerate() {
        let words = words_in_text(slide);
        let bullets = bullet_re().find_iter(slide).count();
        let paragraph_words = max_paragraph_words(slide);
        let human_index = index + 1;

        if words > 140 {
            warnings.push(format!(
                "Slide {human_index} has {words} words (threshold: 140)."
            ));
        }
        if bullets > 8 {
            warnings.push(format!(
                "Slide {human_index} has {bullets} bullets/list items (threshold: 8)."
            ));
        }
        if paragraph_words > 55 {
            warnings.push(format!(
                "Slide {human_index} has a paragraph with {paragraph_words} words (threshold: 55)."
            ));
        }
    }

    let mut seen = HashSet::<String>::new();
    let mut assets_checked = 0usize;

    for captures in markdown_link_re().captures_iter(&body) {
        let raw = captures
            .get(1)
            .or_else(|| captures.get(2))
            .map(|item| item.as_str())
            .unwrap_or_default();

        if let Some(relative_path) = local_asset_path(raw) {
            if !seen.insert(relative_path.clone()) {
                continue;
            }

            if relative_path == ".." || relative_path.starts_with("../") {
                errors.push(format!("Invalid traversal asset path: {raw}"));
                continue;
            }

            let Some(resolved) = resolve_relative_path(&canonical_project, &relative_path) else {
                errors.push(format!("Asset path escapes project folder: {raw}"));
                continue;
            };
            if !resolved.exists() {
                errors.push(format!(
                    "Missing asset target: {raw} -> {}",
                    resolved.display()
                ));
                continue;
            }
            assets_checked += 1;
        }
    }

    for captures in attr_link_re().captures_iter(&body) {
        let raw = captures
            .get(1)
            .map(|item| item.as_str())
            .unwrap_or_default();

        if let Some(relative_path) = local_asset_path(raw) {
            if !seen.insert(relative_path.clone()) {
                continue;
            }

            if relative_path == ".." || relative_path.starts_with("../") {
                errors.push(format!("Invalid traversal asset path: {raw}"));
                continue;
            }

            let Some(resolved) = resolve_relative_path(&canonical_project, &relative_path) else {
                errors.push(format!("Asset path escapes project folder: {raw}"));
                continue;
            };
            if !resolved.exists() {
                errors.push(format!(
                    "Missing asset target: {raw} -> {}",
                    resolved.display()
                ));
                continue;
            }
            assets_checked += 1;
        }
    }

    Ok(ValidationReport {
        path: path_to_string(&canonical_project),
        slide_count: slides.len(),
        assets_checked,
        errors,
        warnings,
    })
}

#[tauri::command]
fn get_app_state() -> Result<AppState, String> {
    let state = build_state()?;
    save_config(&state.config)?;
    Ok(state)
}

#[tauri::command]
fn open_project(path: String) -> Result<ProjectDetail, String> {
    let project_path = normalize_existing_project_directory(&path)?;
    let mut config = normalized_config(load_config()?);
    remember_recent_project(&mut config, &project_path);
    save_config(&config)?;
    project_detail_for_path(&config, &project_path)
}

#[tauri::command]
fn add_projects_root(path: String) -> Result<AppState, String> {
    let canonical = normalize_existing_directory(&path)?;
    let canonical_str = path_to_string(&canonical);

    let mut config = normalized_config(load_config()?);
    if !config
        .projects_roots
        .iter()
        .any(|root| root == &canonical_str)
    {
        config.projects_roots.push(canonical_str);
    }
    config = normalized_config(config);
    save_config(&config)?;

    Ok(AppState {
        projects: list_projects(&config),
        config,
    })
}

#[tauri::command]
fn remove_projects_root(path: String) -> Result<AppState, String> {
    let mut config = normalized_config(load_config()?);
    let expanded = path_to_string(&expand_user_path(&path));
    let canonical = normalize_existing_directory(&path)
        .ok()
        .map(|item| path_to_string(&item));

    config.projects_roots.retain(|root| {
        let matches_input = root == &expanded;
        let matches_canonical = canonical
            .as_ref()
            .map(|resolved| root == resolved)
            .unwrap_or(false);
        !(matches_input || matches_canonical)
    });
    save_config(&config)?;

    Ok(AppState {
        projects: list_projects(&config),
        config,
    })
}

#[tauri::command]
fn remove_project(path: String) -> Result<AppState, String> {
    let mut config = normalized_config(load_config()?);
    let expanded = path_to_string(&expand_user_path(&path));
    let canonical = normalize_existing_directory(&path)
        .ok()
        .map(|item| path_to_string(&item));

    config.recent_projects.retain(|project| {
        let matches_input = project == &expanded;
        let matches_canonical = canonical
            .as_ref()
            .map(|resolved| project == resolved)
            .unwrap_or(false);
        !(matches_input || matches_canonical)
    });

    save_config(&config)?;
    Ok(AppState {
        projects: list_projects(&config),
        config,
    })
}

#[tauri::command]
fn load_project(path: String) -> Result<ProjectDetail, String> {
    let project_path = normalize_existing_project_directory(&path)?;
    let mut config = normalized_config(load_config()?);
    remember_recent_project(&mut config, &project_path);
    save_config(&config)?;
    project_detail_for_path(&config, &project_path)
}

#[tauri::command]
fn save_project(path: String, page_mdx: String) -> Result<ProjectDetail, String> {
    let project_path = normalize_existing_project_directory(&path)?;
    write_page_mdx(&project_path, &page_mdx)?;
    let mut config = normalized_config(load_config()?);
    remember_recent_project(&mut config, &project_path);
    save_config(&config)?;
    project_detail_for_path(&config, &project_path)
}

#[tauri::command]
fn create_project(
    root: String,
    name: String,
    title: Option<String>,
    subtitle: Option<String>,
    date_label: Option<String>,
) -> Result<ProjectDetail, String> {
    if !project_name_re().is_match(name.as_str()) {
        return Err(
            "Invalid project name. Use letters, numbers, dot, underscore, and dash.".to_string(),
        );
    }

    let root_path = normalize_existing_directory(&root)?;
    let project_path = root_path.join(name.as_str());
    if project_path.exists() {
        return Err(format!(
            "Project folder already exists: {}",
            project_path.display()
        ));
    }

    fs::create_dir_all(project_path.join("images"))
        .map_err(|error| format!("Failed to create images folder: {error}"))?;
    fs::create_dir_all(project_path.join("media"))
        .map_err(|error| format!("Failed to create media folder: {error}"))?;
    fs::create_dir_all(project_path.join("data"))
        .map_err(|error| format!("Failed to create data folder: {error}"))?;

    let starter = build_starter_page(
        name.as_str(),
        title.as_deref().unwrap_or(DEFAULT_TITLE),
        subtitle.as_deref().unwrap_or(DEFAULT_SUBTITLE),
        date_label.as_deref().unwrap_or(DEFAULT_DATE_LABEL),
    );
    write_page_mdx(&project_path, &starter)?;

    let mut config = normalized_config(load_config()?);
    remember_recent_project(&mut config, &project_path);
    save_config(&config)?;
    project_detail_for_path(&config, &project_path)
}

#[tauri::command]
fn validate_project(path: String) -> Result<ValidationReport, String> {
    validate_project_folder(Path::new(&path))
}

#[tauri::command]
fn resolve_project_asset_data_url(project_path: String, raw_src: String) -> Result<String, String> {
    if raw_src.trim().is_empty() || raw_src.trim().starts_with('#') {
        return Ok(raw_src);
    }

    let canonical_project = normalize_existing_project_directory(&project_path)?;
    let Some(relative_path) = local_asset_path(raw_src.as_str()) else {
        return Ok(raw_src);
    };

    if relative_path == ".." || relative_path.starts_with("../") {
        return Err(format!("Invalid traversal asset path: {raw_src}"));
    }

    let Some(resolved_path) = resolve_relative_path(&canonical_project, &relative_path) else {
        return Err(format!("Asset path escapes project folder: {raw_src}"));
    };

    if !resolved_path.exists() {
        return Err(format!(
            "Missing asset target: {} -> {}",
            raw_src,
            resolved_path.display()
        ));
    }

    if !resolved_path.is_file() {
        return Err(format!(
            "Asset target is not a file: {} -> {}",
            raw_src,
            resolved_path.display()
        ));
    }

    let asset_bytes = fs::read(&resolved_path)
        .map_err(|error| format!("Failed to read {}: {error}", resolved_path.display()))?;
    let mime_type = mime_type_for_path(&resolved_path);
    let encoded = base64::engine::general_purpose::STANDARD.encode(asset_bytes);
    Ok(format!("data:{mime_type};base64,{encoded}"))
}

#[tauri::command]
fn open_in_file_manager(path: String) -> Result<(), String> {
    let target = normalize_existing_directory(&path)?;

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut cmd = Command::new("open");
        cmd.arg(&target);
        cmd
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut cmd = Command::new("explorer");
        cmd.arg(&target);
        cmd
    };

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let mut command = {
        let mut cmd = Command::new("xdg-open");
        cmd.arg(&target);
        cmd
    };

    command
        .spawn()
        .map_err(|error| format!("Failed to open {}: {error}", target.display()))?;

    Ok(())
}

fn ensure_zip_destination(path_str: &str) -> PathBuf {
    let path = expand_user_path(path_str);
    let has_zip_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("zip"))
        .unwrap_or(false);
    if has_zip_extension {
        path
    } else {
        path.with_extension("zip")
    }
}

fn resolve_fastslides_skill_directory() -> Result<PathBuf, String> {
    let mut candidates = Vec::<PathBuf>::new();

    if let Ok(explicit) = env::var("FASTSLIDES_SKILL_DIR") {
        let explicit = explicit.trim();
        if !explicit.is_empty() {
            candidates.push(expand_user_path(explicit));
        }
    }

    if let Ok(home) = env::var("HOME") {
        let home_path = PathBuf::from(home);
        candidates.push(home_path.join(".agents").join("skills").join("fastslides"));
        candidates.push(home_path.join(".codex").join("skills").join("fastslides"));
    }

    let mut checked = Vec::<String>::new();
    for candidate in candidates {
        checked.push(path_to_string(&candidate));
        let skill_marker = candidate.join("SKILL.md");
        if candidate.is_dir() && skill_marker.is_file() {
            return candidate
                .canonicalize()
                .map_err(|error| format!("Failed to resolve {}: {error}", candidate.display()));
        }
    }

    Err(format!(
        "Could not locate FastSlides skill folder. Checked: {}",
        checked.join(", ")
    ))
}

#[tauri::command]
fn export_fastslides_skill(destination: String) -> Result<String, String> {
    let skill_dir = resolve_fastslides_skill_directory()?;
    let destination_path = ensure_zip_destination(destination.as_str());

    let parent = destination_path.parent().ok_or_else(|| {
        format!(
            "Destination path has no parent folder: {}",
            destination_path.display()
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create destination folder {}: {error}", parent.display()))?;

    if destination_path.exists() {
        fs::remove_file(&destination_path).map_err(|error| {
            format!(
                "Failed to overwrite existing archive {}: {error}",
                destination_path.display()
            )
        })?;
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("ditto")
            .arg("-c")
            .arg("-k")
            .arg("--sequesterRsrc")
            .arg("--keepParent")
            .arg(&skill_dir)
            .arg(&destination_path)
            .status()
            .map_err(|error| format!("Failed to run ditto for skill export: {error}"))?;

        if !status.success() {
            return Err(format!(
                "Skill archive export failed with status {}.",
                status
            ));
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = skill_dir;
        return Err("Skill export is currently implemented for macOS only.".to_string());
    }

    Ok(path_to_string(&destination_path))
}

fn build_app_menu<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(app)?;

    #[cfg(target_os = "macos")]
    {
        let items = menu.items()?;
        if let Some(app_submenu) = items.first().and_then(|item| item.as_submenu()) {
            let export_item = MenuItem::with_id(
                app,
                MENU_EXPORT_SKILL_ID,
                "Download FastSlides Skill…",
                true,
                None::<&str>,
            )?;
            let separator = PredefinedMenuItem::separator(app)?;
            app_submenu.insert(&separator, 2)?;
            app_submenu.insert(&export_item, 3)?;
        }
    }

    Ok(menu)
}

fn json_response(status_code: u16, payload: impl Serialize) -> Response<Cursor<Vec<u8>>> {
    let body = serde_json::to_string(&payload)
        .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"JSON serialization failed.\"}".to_string());
    let mut response = Response::from_string(body).with_status_code(StatusCode(status_code));

    if let Ok(content_type) = Header::from_bytes("Content-Type", "application/json") {
        response.add_header(content_type);
    }
    if let Ok(access_control) = Header::from_bytes("Access-Control-Allow-Origin", "*") {
        response.add_header(access_control);
    }
    response
}

fn json_error_response(status_code: u16, message: String) -> Response<Cursor<Vec<u8>>> {
    json_response(
        status_code,
        HookError {
            ok: false,
            error: message,
        },
    )
}

fn read_json_body<T: for<'de> Deserialize<'de>>(request: &mut Request) -> Result<T, String> {
    let mut body = String::new();
    request
        .as_reader()
        .read_to_string(&mut body)
        .map_err(|error| format!("Failed to read request body: {error}"))?;

    serde_json::from_str::<T>(&body).map_err(|error| format!("Invalid JSON payload: {error}"))
}

fn preview_base_url() -> String {
    env::var("FASTSLIDES_PREVIEW_URL")
        .ok()
        .or_else(|| env::var("NEXT_PUBLIC_FASTSLIDES_PREVIEW_URL").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PREVIEW_BASE_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

fn agent_hook_addr() -> String {
    env::var("FASTSLIDES_AGENT_HOOK_ADDR")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_AGENT_HOOK_ADDR.to_string())
}

fn build_preview_url_for_path(project_path: &str) -> String {
    let mut serializer = UrlQuerySerializer::new(String::new());
    serializer.append_pair("deckPath", project_path);
    let query = serializer.finish();
    format!("{}/?{query}", preview_base_url())
}

fn handle_agent_hook_request(method: &Method, request_url: &str, request: &mut Request) -> Response<Cursor<Vec<u8>>> {
    let parsed = Url::parse(format!("http://localhost{request_url}").as_str());
    let parsed_url = match parsed {
        Ok(url) => url,
        Err(error) => return json_error_response(400, format!("Invalid request URL: {error}")),
    };

    let path = parsed_url.path();

    match (method, path) {
        (&Method::Get, "/health") => json_response(
            200,
            HookStatus {
                ok: true,
                service: "fastslides-agent-hook".to_string(),
            },
        ),
        (&Method::Get, "/app-state") => match get_app_state() {
            Ok(state) => json_response(200, state),
            Err(error) => json_error_response(500, error),
        },
        (&Method::Get, "/preview-url") => {
            let project_path = parsed_url
                .query_pairs()
                .find_map(|(key, value)| (key == "path").then(|| value.into_owned()))
                .unwrap_or_default();

            if project_path.trim().is_empty() {
                return json_error_response(
                    400,
                    "Missing required query parameter: path".to_string(),
                );
            }

            json_response(
                200,
                PreviewUrlResponse {
                    ok: true,
                    preview_url: build_preview_url_for_path(project_path.as_str()),
                },
            )
        }
        (&Method::Post, "/open-project") => {
            let payload = match read_json_body::<PathPayload>(request) {
                Ok(value) => value,
                Err(error) => return json_error_response(400, error),
            };
            match open_project(payload.path) {
                Ok(detail) => json_response(200, detail),
                Err(error) => json_error_response(400, error),
            }
        }
        (&Method::Post, "/validate-project") => {
            let payload = match read_json_body::<PathPayload>(request) {
                Ok(value) => value,
                Err(error) => return json_error_response(400, error),
            };
            match validate_project(payload.path) {
                Ok(report) => json_response(200, report),
                Err(error) => json_error_response(400, error),
            }
        }
        _ => json_error_response(
            404,
            format!("Unknown endpoint: {:?} {}", method, path),
        ),
    }
}

fn start_agent_hook_server() {
    let bind_addr = agent_hook_addr();
    thread::spawn(move || {
        let server = match Server::http(bind_addr.as_str()) {
            Ok(server) => server,
            Err(error) => {
                log::warn!(
                    "FastSlides agent hook unavailable on {}: {}",
                    bind_addr,
                    error
                );
                return;
            }
        };

        log::info!("FastSlides agent hook listening on http://{}", bind_addr);

        for mut request in server.incoming_requests() {
            let method = request.method().clone();
            let request_url = request.url().to_string();
            let response = handle_agent_hook_request(&method, request_url.as_str(), &mut request);
            let _ = request.respond(response);
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    start_agent_hook_server();

    tauri::Builder::default()
        .menu(|app| build_app_menu(app))
        .on_menu_event(|app, event| {
            if event.id() == MENU_EXPORT_SKILL_ID {
                if let Err(error) = app.emit(MENU_EXPORT_SKILL_EVENT, ()) {
                    log::warn!("Failed to emit skill export menu event: {}", error);
                }
            }
        })
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = apply_vibrancy(&window, NSVisualEffectMaterial::Sidebar, None, Some(10.0));
                }
            }

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            add_projects_root,
            create_project,
            get_app_state,
            load_project,
            open_project,
            open_in_file_manager,
            export_fastslides_skill,
            remove_project,
            remove_projects_root,
            save_project,
            resolve_project_asset_data_url,
            validate_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
