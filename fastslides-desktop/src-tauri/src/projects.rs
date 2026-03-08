use crate::config::{
    expand_user_path, load_config, normalize_existing_directory,
    normalize_existing_project_directory, normalized_config, path_to_string, save_config,
    AppConfig,
};
use crate::constants::{DEFAULT_DATE_LABEL, DEFAULT_SLIDES_CSS, DEFAULT_SUBTITLE, DEFAULT_TITLE};
use crate::deck::{
    attr_link_re, bullet_re, component_counts_for_slide, extract_frontmatter, extract_slides,
    import_export_re, local_asset_path, markdown_link_re, max_paragraph_words, project_name_re,
    resolve_relative_path, split_class_re, use_client_re, words_in_text,
};
use crate::now_epoch_seconds;
use crate::scene::slide_contract_warnings;
use serde::Serialize;
use std::{collections::HashSet, fs, path::Path, time::UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectSummary {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) root: String,
    pub(crate) slide_count: usize,
    pub(crate) updated_at: u64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectDetail {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) root: String,
    pub(crate) page_mdx: String,
    pub(crate) slide_count: usize,
    pub(crate) updated_at: u64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ValidationReport {
    pub(crate) path: String,
    pub(crate) slide_count: usize,
    pub(crate) assets_checked: usize,
    pub(crate) errors: Vec<String>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AppState {
    pub(crate) config: AppConfig,
    pub(crate) projects: Vec<ProjectSummary>,
}

pub(crate) fn read_page_mdx(project_dir: &Path) -> Result<String, String> {
    let page_path = project_dir.join("page.mdx");
    fs::read_to_string(&page_path)
        .map_err(|error| format!("Failed to read {}: {error}", page_path.display()))
}

pub(crate) fn write_page_mdx(project_dir: &Path, content: &str) -> Result<(), String> {
    let page_path = project_dir.join("page.mdx");
    fs::write(&page_path, content)
        .map_err(|error| format!("Failed to write {}: {error}", page_path.display()))
}

fn slide_count_from_source(source: &str) -> usize {
    extract_slides(source).len()
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
            .unwrap_or_default()
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
  <Canvas cols={{50}} rows={{25}} gap="1px">
    <Area x={{2}} y={{2}} w={{14}} h={{1}}>
      <Kicker>{project}</Kicker>
    </Area>

    <Area x={{2}} y={{4}} w={{46}} h={{4}}>
      <Takeaway>{title}</Takeaway>
    </Area>

    <Area x={{2}} y={{9}} w={{18}} h={{2}}>
      <p>{subtitle}</p>
    </Area>

    <Area x={{2}} y={{12}} w={{22}} h={{2}}>
      <PillRow>
        <Pill>50 x 25 grid</Pill>
        <Pill>structured primitives</Pill>
        <Pill>reviewable output</Pill>
      </PillRow>
    </Area>

    <Area x={{34}} y={{10}} w={{14}} h={{11}}>
      <Callout title="How to use this starter" tone="accent">
        Start with a takeaway title, add a few regions, and keep each slide to one job.
      </Callout>
    </Area>

    <Area x={{2}} y={{23}} w={{12}} h={{1}}>
      <Caption>{date_label}</Caption>
    </Area>
  </Canvas>

</section>

<section className="slide">
  <Canvas cols={{50}} rows={{25}} gap="1px">
    <Area x={{2}} y={{2}} w={{16}} h={{1}}>
      <Kicker>Situation</Kicker>
    </Area>

    <Area x={{2}} y={{4}} w={{46}} h={{4}}>
      <Takeaway>Start from reusable page patterns instead of ad hoc HTML.</Takeaway>
    </Area>

    <Area x={{2}} y={{9}} w={{14}} h={{12}}>
      <Panel title="Challenge" tone="accent">
        <ul>
          <li>Raw layout markup is hard to edit safely.</li>
          <li>Dense slides drift into awkward spacing.</li>
          <li>Review is mostly visual guesswork.</li>
        </ul>
      </Panel>
    </Area>

    <Area x={{18}} y={{9}} w={{14}} h={{12}}>
      <Panel title="2.0 move">
        <ul>
          <li>Use `Canvas` and `Area` for page geometry.</li>
          <li>Use `Panel`, `Callout`, `Metric`, and `Pill` for repeatable structure.</li>
          <li>Keep theme tokens in `slides.css`.</li>
        </ul>
      </Panel>
    </Area>

    <Area x={{34}} y={{9}} w={{14}} h={{12}}>
      <Panel title="Result">
        <ul>
          <li>Smaller type, calmer pages, and clearer hierarchy.</li>
          <li>Decks become easier for agents to mutate and review.</li>
          <li>Preview and export can converge on one scene model.</li>
        </ul>
      </Panel>
    </Area>
  </Canvas>

</section>

<section className="slide">
  <Canvas cols={{50}} rows={{25}} gap="1px">
    <Area x={{2}} y={{2}} w={{18}} h={{1}}>
      <Kicker>Starter kit</Kicker>
    </Area>

    <Area x={{2}} y={{4}} w={{46}} h={{4}}>
      <Takeaway>Every new deck should begin with a small set of stable building blocks.</Takeaway>
    </Area>

    <Area x={{2}} y={{9}} w={{18}} h={{12}}>
      <Panel title="Recommended flow" tone="accent">
        <ol>
          <li>Write the takeaway first.</li>
          <li>Place 2 to 4 regions on the canvas.</li>
          <li>Choose one evidence pattern per slide.</li>
          <li>Run review before approval or export.</li>
        </ol>
      </Panel>
    </Area>

    <Area x={{22}} y={{9}} w={{10}} h={{5}}>
      <Metric label="Grid" value="50x25" hint="Authoring canvas" />
    </Area>

    <Area x={{22}} y={{15}} w={{10}} h={{5}}>
      <Metric label="Blocks" value="12+" hint="Reusable parts" />
    </Area>

    <Area x={{34}} y={{9}} w={{14}} h={{12}}>
      <Callout title="Next step">
        Replace DOM-as-truth rendering with a typed scene compiler so preview and export share the same model.
      </Callout>
    </Area>
  </Canvas>

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
        let component_counts = component_counts_for_slide(slide);
        let human_index = index + 1;

        if !crate::deck::uses_spatial_canvas(&component_counts) {
            errors.push(format!(
                "Slide {human_index} must use the 2.0 spatial layout contract (`Canvas` with `Area` regions)."
            ));
        }
        if split_class_re().is_match(slide) {
            errors.push(format!(
                "Slide {human_index} uses legacy `split` layout. Replace it with `Canvas` and `Area`."
            ));
        }

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
        for warning in slide_contract_warnings(slide) {
            warnings.push(format!("Slide {human_index}: {warning}"));
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

pub(crate) fn get_app_state() -> Result<AppState, String> {
    let state = build_state()?;
    save_config(&state.config)?;
    Ok(state)
}

pub(crate) fn open_project(path: String) -> Result<ProjectDetail, String> {
    let project_path = normalize_existing_project_directory(&path)?;
    let mut config = normalized_config(load_config()?);
    remember_recent_project(&mut config, &project_path);
    save_config(&config)?;
    project_detail_for_path(&config, &project_path)
}

pub(crate) fn add_projects_root(path: String) -> Result<AppState, String> {
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

pub(crate) fn remove_projects_root(path: String) -> Result<AppState, String> {
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

pub(crate) fn remove_project(path: String) -> Result<AppState, String> {
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

    config.pinned_projects.retain(|project| {
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

pub(crate) fn load_project(path: String) -> Result<ProjectDetail, String> {
    let project_path = normalize_existing_project_directory(&path)?;
    let mut config = normalized_config(load_config()?);
    remember_recent_project(&mut config, &project_path);
    save_config(&config)?;
    project_detail_for_path(&config, &project_path)
}

pub(crate) fn save_project(path: String, page_mdx: String) -> Result<ProjectDetail, String> {
    let project_path = normalize_existing_project_directory(&path)?;
    write_page_mdx(&project_path, &page_mdx)?;
    let mut config = normalized_config(load_config()?);
    remember_recent_project(&mut config, &project_path);
    save_config(&config)?;
    project_detail_for_path(&config, &project_path)
}

pub(crate) fn create_project(
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
    fs::write(project_path.join("slides.css"), DEFAULT_SLIDES_CSS)
        .map_err(|error| format!("Failed to create slides.css: {error}"))?;

    let mut config = normalized_config(load_config()?);
    remember_recent_project(&mut config, &project_path);
    save_config(&config)?;
    project_detail_for_path(&config, &project_path)
}

pub(crate) fn toggle_project_pin(path: String) -> Result<AppState, String> {
    let project_path = normalize_existing_project_directory(&path)?;
    let mut config = normalized_config(load_config()?);
    let canonical_str = path_to_string(&project_path);

    if config.pinned_projects.contains(&canonical_str) {
        config
            .pinned_projects
            .retain(|project| project != &canonical_str);
    } else {
        config.pinned_projects.push(canonical_str);
    }

    save_config(&config)?;
    Ok(AppState {
        projects: list_projects(&config),
        config,
    })
}

pub(crate) fn validate_project(path: String) -> Result<ValidationReport, String> {
    validate_project_folder(Path::new(&path))
}

pub(crate) fn read_project_css(path: String) -> Result<String, String> {
    let project_dir = normalize_existing_project_directory(&path)?;
    let css_path = project_dir.join("slides.css");
    if !css_path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(&css_path)
        .map_err(|error| format!("Failed to read {}: {error}", css_path.display()))
}

pub(crate) fn save_project_css(path: String, css: String) -> Result<(), String> {
    let project_dir = normalize_existing_project_directory(&path)?;
    let css_path = project_dir.join("slides.css");
    fs::write(&css_path, &css)
        .map_err(|error| format!("Failed to write {}: {error}", css_path.display()))
}
