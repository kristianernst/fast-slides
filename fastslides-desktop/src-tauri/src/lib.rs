use axum::{
    extract::Request as AxumRequest, middleware, middleware::Next, response::IntoResponse, Router,
};
use base64::Engine;
use regex::Regex;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs;
use std::io::Cursor;
use std::net::{TcpStream, ToSocketAddrs};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, Runtime};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use tokio_util::sync::CancellationToken;
use url::{form_urlencoded::Serializer as UrlQuerySerializer, Url};

#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

const PROJECT_NAME_PATTERN: &str = r"^[A-Za-z0-9._-]+$";
const DEFAULT_TITLE: &str = "Presentation";
const DEFAULT_SUBTITLE: &str = "Project Overview";
const DEFAULT_DATE_LABEL: &str = "Month YYYY";
const DEFAULT_PREVIEW_BASE_URL: &str = "http://127.0.0.1:1420";
const DEFAULT_AGENT_HOOK_ADDR: &str = "127.0.0.1:38473";
const DEFAULT_MCP_SERVER_ADDR: &str = "127.0.0.1:38474";
const DEFAULT_MCP_SERVER_PATH: &str = "/mcp";
const FASTSLIDES_MCP_STATEFUL_MODE: bool = false;
const MCP_TRAY_ICON_ID: &str = "fastslides.mcp.server";
const MENU_OPEN_SETTINGS_ID: &str = "menu.open_settings";
const MENU_OPEN_SETTINGS_EVENT: &str = "fastslides://open-settings";
const MENU_OPEN_MAIN_WINDOW_ID: &str = "menu.open_main_window";
const MENU_INSTALL_CODEX_MCP_ID: &str = "menu.install_codex_mcp";
const MENU_EXPORT_SKILL_ID: &str = "menu.export_fastslides_skill";
const MENU_EXPORT_SKILL_EVENT: &str = "fastslides://export-skill";
const SCENE_SESSION_EVENT_NAME: &str = "fastslides://scene-session-event";
const STRUCTURED_COMPONENT_NAMES: &[&str] = &[
    "Stack", "Row", "Grid", "Canvas", "Area", "Card", "Panel", "Callout", "Metric", "Caption",
    "Kicker", "Takeaway", "Chart", "PillRow", "Pill", "Quote", "Rule",
];
const LAYOUT_COMPONENT_NAMES: &[&str] = &["Stack", "Row", "Grid", "Canvas", "Area", "PillRow"];
const SURFACE_COMPONENT_NAMES: &[&str] = &["Card", "Panel", "Callout", "Quote"];
const DEFAULT_SLIDES_CSS: &str = "\
/* ═══ slides.css ═══
   Per-project slide design tokens.
   Edit the custom properties below to customise slide appearance.
   Changes are picked up by the preview whenever settings are saved
   or the project is re-selected. Agents can also edit this file
   directly on disk.
*/

:root {
  /* ── Layout ── */
  --slide-bg: #f5f1e8;
  --slide-border: rgba(30, 42, 56, 0.1);
  --slide-radius: 14px;
  --slide-padding: 36px;
  --slide-layout-gap: 18px;
  --slide-card-bg: rgba(255, 255, 255, 0.72);
  --slide-card-border: rgba(30, 42, 56, 0.1);
  --slide-card-radius: 12px;
  --slide-card-padding: 16px;

  /* ── Typography ── */
  --slide-font-family: \"IBM Plex Sans\", \"Inter\", system-ui, sans-serif;
  --slide-heading-font: \"Iowan Old Style\", \"Georgia\", serif;
  --slide-code-font: \"Fira Code\", monospace;
  --slide-meta-font: var(--slide-code-font);
  --slide-meta-size: 0.72rem;

  /* ── Colors ── */
  --slide-fg: #16212b;
  --slide-h1-color: #13202c;
  --slide-h2-color: #223446;
  --slide-h3-color: #395166;
  --slide-body-color: #425467;
  --slide-meta-color: rgba(66, 84, 103, 0.82);
  --slide-accent: #1f7a78;
  --slide-link-color: var(--slide-accent);
  --slide-code-bg: rgba(23, 32, 43, 0.06);

  /* ── Palette (charts / diagrams / highlights) ── */
  --slide-palette-1: #1f7a78;
  --slide-palette-2: #739e82;
  --slide-palette-3: #d7a65d;
  --slide-palette-4: #ba6b6f;
  --slide-palette-5: #5973a9;
}
";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppConfig {
    #[serde(default)]
    projects_roots: Vec<String>,
    #[serde(default)]
    recent_projects: Vec<String>,
    #[serde(default)]
    pinned_projects: Vec<String>,
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

#[derive(Debug, Deserialize)]
struct ProjectCssPayload {
    path: String,
    css: String,
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

#[derive(Debug, Deserialize)]
struct SlideCapturePayload {
    path: String,
    slide: Option<usize>,
    output_dir: Option<String>,
    headed: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SaveComponentPayload {
    name: String,
    family: String,
    summary: String,
    tags: Option<Vec<String>>,
    mdx: String,
    notes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
struct SlideCaptureResponse {
    ok: bool,
    path: String,
    slide: usize,
    output_dir: String,
    image_path: String,
    preview_url: String,
}

#[derive(Debug, Clone, Serialize)]
struct SaveComponentResponse {
    ok: bool,
    component: ComponentCatalogEntry,
    library_path: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CodexInstallStatus {
    Installed,
    Updated,
    Unchanged,
}

#[derive(Debug, Clone, Serialize)]
struct CodexMcpInstallResponse {
    ok: bool,
    status: CodexInstallStatus,
    config_path: String,
    server_name: String,
    url: String,
}

#[derive(Debug, Clone, Serialize)]
struct NamedCount {
    name: String,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct DeckOutlineEntry {
    index: usize,
    title: String,
}

#[derive(Debug, Clone, Serialize)]
struct SlideAnalysis {
    index: usize,
    title: String,
    archetype: String,
    words: usize,
    bullets: usize,
    max_paragraph_words: usize,
    components: Vec<NamedCount>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ProjectAnalysis {
    path: String,
    slide_count: usize,
    has_project_css: bool,
    outline: Vec<DeckOutlineEntry>,
    components: Vec<NamedCount>,
    archetypes: Vec<NamedCount>,
    warnings: Vec<String>,
    slides: Vec<SlideAnalysis>,
}

#[derive(Debug, Clone, Serialize)]
struct DesignSystemRegistry {
    version: String,
    philosophy: Vec<String>,
    default_frame: DesignFrameSpec,
    primitives: Vec<PrimitiveSpec>,
    compositions: Vec<CompositionSpec>,
    recipes: Vec<RecipeSpec>,
    sections: Vec<SectionSpec>,
}

#[derive(Debug, Clone, Serialize)]
struct DesignFrameSpec {
    cols: usize,
    rows: usize,
    header_rows: usize,
    body_rows: usize,
    footer_rows: usize,
    body_slices: Vec<BodySliceSpec>,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct BodySliceSpec {
    name: String,
    min_rows: usize,
    preferred_rows: usize,
    purpose: String,
}

#[derive(Debug, Clone, Serialize)]
struct AreaSizeSpec {
    cols: usize,
    rows: usize,
}

#[derive(Debug, Clone, Serialize)]
struct SlotSpec {
    name: String,
    accepts: Vec<String>,
    min: usize,
    max: usize,
    note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct PrimitiveSpec {
    name: String,
    purpose: String,
    variants: Vec<String>,
    min_area: Option<AreaSizeSpec>,
    preferred_area: Option<AreaSizeSpec>,
    allowed_parents: Vec<String>,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CompositionSpec {
    name: String,
    purpose: String,
    variants: Vec<String>,
    min_area: AreaSizeSpec,
    preferred_area: AreaSizeSpec,
    source_primitives: Vec<String>,
    slots: Vec<SlotSpec>,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RecipeSpec {
    name: String,
    summary: String,
    frame: DesignFrameSpec,
    compositions: Vec<String>,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SectionSpec {
    name: String,
    summary: String,
    recipes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DesignTemplate {
    kind: String,
    name: String,
    mdx: String,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ComponentCatalog {
    version: String,
    items: Vec<ComponentCatalogEntry>,
}

#[derive(Debug, Clone, Serialize)]
struct ComponentCatalogEntry {
    name: String,
    family: String,
    kind: String,
    scope: String,
    summary: String,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedComponentRecord {
    name: String,
    family: String,
    summary: String,
    tags: Vec<String>,
    mdx: String,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ProjectScene {
    path: String,
    project: Option<String>,
    title: Option<String>,
    subtitle: Option<String>,
    date: Option<String>,
    deck_class_name: Option<String>,
    slide_count: usize,
    slides: Vec<SceneSlide>,
}

#[derive(Debug, Clone, Serialize)]
struct ProjectSceneManifest {
    path: String,
    project: Option<String>,
    title: Option<String>,
    subtitle: Option<String>,
    date: Option<String>,
    deck_class_name: Option<String>,
    slide_count: usize,
    slides: Vec<SceneSlideManifest>,
}

#[derive(Debug, Clone, Serialize)]
struct ProjectSceneSessionHandle {
    session_id: String,
    path: String,
    slide_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ProjectSceneSessionEvent {
    session_id: String,
    sequence: u64,
    #[serde(flatten)]
    payload: ProjectSceneSessionEventPayload,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum ProjectSceneSessionEventPayload {
    Manifest {
        scene: ProjectSceneManifest,
    },
    SlideReady {
        slide: SceneSlide,
    },
    SlideError {
        index: usize,
        error: String,
    },
    Complete {
        ready_count: usize,
        error_count: usize,
    },
}

#[derive(Debug, Clone, Serialize)]
struct SceneSlide {
    index: usize,
    title: String,
    layout: SceneLayout,
    nodes: Vec<SceneNode>,
    source_mdx: String,
}

#[derive(Debug, Clone, Serialize)]
struct SceneSlideManifest {
    index: usize,
    title: String,
    layout: SceneLayout,
}

#[derive(Debug, Clone, Serialize)]
struct SceneLayout {
    kind: String,
    cols: Option<usize>,
    rows: Option<usize>,
    gap: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SceneChartDatum {
    label: String,
    value: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum SceneNode {
    Canvas {
        cols: usize,
        rows: usize,
        gap: Option<String>,
        class_name: Option<String>,
        children: Vec<SceneNode>,
        source_mdx: String,
    },
    Area {
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        layer: Option<usize>,
        gap: Option<String>,
        align: Option<String>,
        justify: Option<String>,
        class_name: Option<String>,
        children: Vec<SceneNode>,
        source_mdx: String,
    },
    LayoutGroup {
        component: String,
        cols: Option<usize>,
        gap: Option<String>,
        align: Option<String>,
        justify: Option<String>,
        nowrap: Option<bool>,
        class_name: Option<String>,
        children: Vec<SceneNode>,
        source_mdx: String,
    },
    Surface {
        component: String,
        tone: Option<String>,
        title: Option<String>,
        kicker: Option<String>,
        subtitle: Option<String>,
        foot: Option<String>,
        attribution: Option<String>,
        class_name: Option<String>,
        children: Vec<SceneNode>,
        source_mdx: String,
    },
    Metric {
        label: Option<String>,
        value: Option<String>,
        hint: Option<String>,
        class_name: Option<String>,
        source_mdx: String,
    },
    Chart {
        chart_type: String,
        title: Option<String>,
        tone: Option<String>,
        value_suffix: Option<String>,
        highlight: Option<String>,
        data: Vec<SceneChartDatum>,
        class_name: Option<String>,
        source_mdx: String,
    },
    Text {
        role: String,
        text: String,
        level: Option<u8>,
        class_name: Option<String>,
    },
    List {
        ordered: bool,
        items: Vec<String>,
    },
    Media {
        media_kind: String,
        src: String,
        alt: Option<String>,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Pill {
        tone: Option<String>,
        text: String,
        class_name: Option<String>,
    },
    Rule {
        class_name: Option<String>,
    },
    Arrow {
        direction: Option<String>,
        tone: Option<String>,
        label: Option<String>,
        class_name: Option<String>,
        source_mdx: String,
    },
    Raw {
        format: String,
        text: String,
    },
}

#[derive(Debug, Clone, Copy)]
struct CanvasFrame {
    cols: usize,
    rows: usize,
}

#[derive(Debug, Clone, Copy)]
struct AreaFrame {
    w: usize,
    h: usize,
}

impl AreaFrame {
    fn size_label(self) -> String {
        format!("{} x {}", self.w, self.h)
    }
}

#[derive(Debug, Clone)]
struct McpServerStatus {
    running: bool,
    url: String,
    error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct McpPathParams {
    path: String,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct McpSceneSlideParams {
    path: String,
    index: usize,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct McpDesignTemplateParams {
    name: String,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct McpSaveComponentParams {
    name: String,
    family: String,
    summary: String,
    tags: Option<Vec<String>>,
    mdx: String,
    notes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct McpSlideCaptureParams {
    path: String,
    slide: Option<usize>,
    output_dir: Option<String>,
    headed: Option<bool>,
}

#[derive(Debug, Clone)]
struct ProjectSceneSource {
    path: String,
    project: Option<String>,
    title: Option<String>,
    subtitle: Option<String>,
    date: Option<String>,
    deck_class_name: Option<String>,
    slides: Vec<String>,
}

static SCENE_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
struct FastSlidesMcpServer {
    tool_router: ToolRouter<Self>,
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn next_scene_session_id() -> String {
    let next = SCENE_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("scene-session-{}-{next}", now_epoch_seconds())
}

fn preferred_scene_session_worker_count() -> usize {
    let available = thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4);
    available.clamp(2, 6)
}

fn prioritize_scene_slide_indices(total: usize, start_index: usize) -> Vec<usize> {
    if total == 0 {
        return Vec::new();
    }

    let clamped_start = start_index.min(total - 1);
    let mut ordered = Vec::<usize>::with_capacity(total);

    for offset in 0..total {
        let forward = clamped_start + offset;
        if forward < total {
            ordered.push(forward);
        }

        if offset > 0 {
            let backward = clamped_start.saturating_sub(offset);
            if backward < clamped_start {
                ordered.push(backward);
            }
        }

        if ordered.len() >= total {
            break;
        }
    }

    ordered.truncate(total);
    ordered
}

fn build_project_scene_manifest_from_source(source: &ProjectSceneSource) -> ProjectSceneManifest {
    ProjectSceneManifest {
        path: source.path.clone(),
        project: source.project.clone(),
        title: source.title.clone(),
        subtitle: source.subtitle.clone(),
        date: source.date.clone(),
        deck_class_name: source.deck_class_name.clone(),
        slide_count: source.slides.len(),
        slides: source
            .slides
            .iter()
            .enumerate()
            .map(|(index, slide)| build_scene_slide_manifest(slide, index))
            .collect(),
    }
}

fn try_build_scene_slide(slides: &[String], index: usize) -> Result<SceneSlide, String> {
    let slide = slides.get(index).ok_or_else(|| {
        format!(
            "Slide {} is out of range for this deck ({} slides).",
            index + 1,
            slides.len()
        )
    })?;

    catch_unwind(AssertUnwindSafe(|| build_scene_slide(slide, index))).map_err(|panic| match panic
        .downcast_ref::<String>(
    ) {
        Some(message) => message.clone(),
        None => match panic.downcast_ref::<&str>() {
            Some(message) => (*message).to_string(),
            None => format!("Slide {} panicked during scene compilation.", index + 1),
        },
    })
}

fn emit_project_scene_session_events<F>(
    source: ProjectSceneSource,
    priority_index: usize,
    worker_count: usize,
    emit: &mut F,
) -> Result<(), String>
where
    F: FnMut(ProjectSceneSessionEventPayload),
{
    emit(ProjectSceneSessionEventPayload::Manifest {
        scene: build_project_scene_manifest_from_source(&source),
    });

    let slide_count = source.slides.len();
    if slide_count == 0 {
        emit(ProjectSceneSessionEventPayload::Complete {
            ready_count: 0,
            error_count: 0,
        });
        return Ok(());
    }

    let ordered_indices = prioritize_scene_slide_indices(slide_count, priority_index);
    let primary_index = ordered_indices[0];
    let mut ready_count = 0usize;
    let mut error_count = 0usize;

    match try_build_scene_slide(&source.slides, primary_index) {
        Ok(slide) => {
            ready_count += 1;
            emit(ProjectSceneSessionEventPayload::SlideReady { slide });
        }
        Err(error) => {
            error_count += 1;
            emit(ProjectSceneSessionEventPayload::SlideError {
                index: primary_index,
                error,
            });
        }
    }

    let remaining: Vec<usize> = ordered_indices.into_iter().skip(1).collect();
    if remaining.is_empty() {
        emit(ProjectSceneSessionEventPayload::Complete {
            ready_count,
            error_count,
        });
        return Ok(());
    }

    let slides = Arc::new(source.slides);
    let queue = Arc::new(Mutex::new(VecDeque::from(remaining.clone())));
    let (tx, rx) = mpsc::channel::<(usize, Result<SceneSlide, String>)>();
    let worker_total = remaining.len().min(worker_count.max(1));
    let mut handles = Vec::<thread::JoinHandle<()>>::with_capacity(worker_total);

    for _ in 0..worker_total {
        let tx = tx.clone();
        let queue = Arc::clone(&queue);
        let slides = Arc::clone(&slides);
        handles.push(thread::spawn(move || loop {
            let next_index = {
                let mut pending = match queue.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                pending.pop_front()
            };

            let Some(index) = next_index else {
                break;
            };

            let result = try_build_scene_slide(slides.as_ref(), index);
            if tx.send((index, result)).is_err() {
                break;
            }
        }));
    }
    drop(tx);

    for _ in 0..remaining.len() {
        let (index, result) = rx
            .recv()
            .map_err(|error| format!("Scene session worker channel failed: {error}"))?;
        match result {
            Ok(slide) => {
                ready_count += 1;
                emit(ProjectSceneSessionEventPayload::SlideReady { slide });
            }
            Err(error) => {
                error_count += 1;
                emit(ProjectSceneSessionEventPayload::SlideError { index, error });
            }
        }
    }

    for handle in handles {
        let _ = handle.join();
    }

    emit(ProjectSceneSessionEventPayload::Complete {
        ready_count,
        error_count,
    });
    Ok(())
}

#[cfg(test)]
fn collect_project_scene_session_events(
    source: ProjectSceneSource,
    priority_index: usize,
    worker_count: usize,
) -> Result<Vec<ProjectSceneSessionEventPayload>, String> {
    let mut events = Vec::<ProjectSceneSessionEventPayload>::new();
    emit_project_scene_session_events(source, priority_index, worker_count, &mut |event| {
        events.push(event);
    })?;
    Ok(events)
}

impl FastSlidesMcpServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl FastSlidesMcpServer {
    #[tool(
        name = "health",
        description = "Check FastSlides desktop and hook health."
    )]
    fn tool_health(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&HookStatus {
            ok: true,
            service: "fastslides-agent-hook".to_string(),
        })
        .map_err(|error| format!("Failed to serialize health response: {error}"))
    }

    #[tool(
        name = "get_app_state",
        description = "Get current FastSlides app state and tracked projects."
    )]
    fn tool_get_app_state(&self) -> Result<String, String> {
        let state = get_app_state()?;
        serde_json::to_string_pretty(&state)
            .map_err(|error| format!("Failed to serialize app state: {error}"))
    }

    #[tool(
        name = "get_design_system",
        description = "Return the FastSlides base design-system registry: frame defaults, primitive contracts, compositions, recipes, and section flows."
    )]
    fn tool_get_design_system(&self) -> Result<String, String> {
        let registry = get_design_system()?;
        serde_json::to_string_pretty(&registry)
            .map_err(|error| format!("Failed to serialize design-system registry: {error}"))
    }

    #[tool(
        name = "get_component_catalog",
        description = "Return the FastSlides component phonebook: builtin primitives, compositions, recipes, patterns, and saved custom snippets."
    )]
    fn tool_get_component_catalog(&self) -> Result<String, String> {
        let catalog = get_component_catalog()?;
        serde_json::to_string_pretty(&catalog)
            .map_err(|error| format!("Failed to serialize component catalog: {error}"))
    }

    #[tool(
        name = "get_component_template",
        description = "Return canonical MDX for one FastSlides component, pattern, composition, recipe, or saved custom snippet."
    )]
    fn tool_get_component_template(
        &self,
        Parameters(params): Parameters<McpDesignTemplateParams>,
    ) -> Result<String, String> {
        let template = get_component_template(params.name)?;
        serde_json::to_string_pretty(&template)
            .map_err(|error| format!("Failed to serialize component template: {error}"))
    }

    #[tool(
        name = "save_component_template",
        description = "Save a reusable custom FastSlides component snippet into the local component phonebook for later agent lookup."
    )]
    fn tool_save_component_template(
        &self,
        Parameters(params): Parameters<McpSaveComponentParams>,
    ) -> Result<String, String> {
        let saved = save_component_template(SaveComponentPayload {
            name: params.name,
            family: params.family,
            summary: params.summary,
            tags: params.tags,
            mdx: params.mdx,
            notes: params.notes,
        })?;
        serde_json::to_string_pretty(&saved)
            .map_err(|error| format!("Failed to serialize saved component: {error}"))
    }

    #[tool(
        name = "get_composition_template",
        description = "Return canonical MDX for one reusable FastSlides composition cluster. Paste the result inside a Canvas."
    )]
    fn tool_get_composition_template(
        &self,
        Parameters(params): Parameters<McpDesignTemplateParams>,
    ) -> Result<String, String> {
        let template = get_composition_template(params.name)?;
        serde_json::to_string_pretty(&template)
            .map_err(|error| format!("Failed to serialize composition template: {error}"))
    }

    #[tool(
        name = "get_recipe_template",
        description = "Return canonical MDX for one full-slide FastSlides recipe. Use recipes as the default generation target before raw areas."
    )]
    fn tool_get_recipe_template(
        &self,
        Parameters(params): Parameters<McpDesignTemplateParams>,
    ) -> Result<String, String> {
        let template = get_recipe_template(params.name)?;
        serde_json::to_string_pretty(&template)
            .map_err(|error| format!("Failed to serialize recipe template: {error}"))
    }

    #[tool(
        name = "open_project",
        description = "Open a FastSlides project by absolute path."
    )]
    fn tool_open_project(
        &self,
        Parameters(params): Parameters<McpPathParams>,
    ) -> Result<String, String> {
        let detail = open_project(params.path)?;
        serde_json::to_string_pretty(&detail)
            .map_err(|error| format!("Failed to serialize project detail: {error}"))
    }

    #[tool(
        name = "validate_project",
        description = "Validate a FastSlides project by absolute path."
    )]
    fn tool_validate_project(
        &self,
        Parameters(params): Parameters<McpPathParams>,
    ) -> Result<String, String> {
        let report = validate_project(params.path)?;
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("Failed to serialize validation report: {error}"))
    }

    #[tool(
        name = "get_project_outline",
        description = "Return slide titles and indices for a FastSlides project."
    )]
    fn tool_get_project_outline(
        &self,
        Parameters(params): Parameters<McpPathParams>,
    ) -> Result<String, String> {
        let analysis = analyze_project(params.path)?;
        serde_json::to_string_pretty(&serde_json::json!({
            "path": analysis.path,
            "slide_count": analysis.slide_count,
            "outline": analysis.outline,
        }))
        .map_err(|error| format!("Failed to serialize project outline: {error}"))
    }

    #[tool(
        name = "analyze_project",
        description = "Analyze deck structure, inferred slide archetypes, component usage, and review findings for a FastSlides project."
    )]
    fn tool_analyze_project(
        &self,
        Parameters(params): Parameters<McpPathParams>,
    ) -> Result<String, String> {
        let analysis = analyze_project(params.path)?;
        serde_json::to_string_pretty(&analysis)
            .map_err(|error| format!("Failed to serialize project analysis: {error}"))
    }

    #[tool(
        name = "compile_project_scene",
        description = "Compile a FastSlides project into a typed scene graph for custom rendering experiments."
    )]
    fn tool_compile_project_scene(
        &self,
        Parameters(params): Parameters<McpPathParams>,
    ) -> Result<String, String> {
        let scene = compile_project_scene(params.path)?;
        serde_json::to_string_pretty(&scene)
            .map_err(|error| format!("Failed to serialize project scene: {error}"))
    }

    #[tool(
        name = "compile_project_scene_manifest",
        description = "Compile only deck metadata and slide manifest for progressive scene rendering."
    )]
    fn tool_compile_project_scene_manifest(
        &self,
        Parameters(params): Parameters<McpPathParams>,
    ) -> Result<String, String> {
        let manifest = compile_project_scene_manifest(params.path)?;
        serde_json::to_string_pretty(&manifest)
            .map_err(|error| format!("Failed to serialize scene manifest: {error}"))
    }

    #[tool(
        name = "compile_project_scene_slide",
        description = "Compile a single slide scene node tree by index for progressive preview rendering."
    )]
    fn tool_compile_project_scene_slide(
        &self,
        Parameters(params): Parameters<McpSceneSlideParams>,
    ) -> Result<String, String> {
        let slide = compile_project_scene_slide(params.path, params.index)?;
        serde_json::to_string_pretty(&slide)
            .map_err(|error| format!("Failed to serialize scene slide: {error}"))
    }

    #[tool(
        name = "preview_url",
        description = "Build the browser workspace preview URL for a project path."
    )]
    fn tool_preview_url(
        &self,
        Parameters(params): Parameters<McpPathParams>,
    ) -> Result<String, String> {
        let project_path = params.path.trim();
        if project_path.is_empty() {
            return Err("Missing required parameter: path".to_string());
        }

        serde_json::to_string_pretty(&PreviewUrlResponse {
            ok: true,
            preview_url: build_preview_url_for_path(project_path),
        })
        .map_err(|error| format!("Failed to serialize preview URL response: {error}"))
    }

    #[tool(
        name = "ensure_preview",
        description = "Check if the browser workspace preview is reachable for a project path."
    )]
    fn tool_ensure_preview(
        &self,
        Parameters(params): Parameters<McpPathParams>,
    ) -> Result<String, String> {
        let project_path = params.path.trim();
        if project_path.is_empty() {
            return Err("Missing required parameter: path".to_string());
        }

        let preview_url = build_preview_url_for_path(project_path);
        let reachable = is_preview_url_reachable(preview_url.as_str());
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": reachable,
            "path": project_path,
            "preview_url": preview_url,
            "reachable": reachable,
        }))
        .map_err(|error| format!("Failed to serialize ensure_preview result: {error}"))
    }

    #[tool(
        name = "capture_slide_image",
        description = "Capture a PNG screenshot for a specific slide in the browser workspace preview."
    )]
    fn tool_capture_slide_image(
        &self,
        Parameters(params): Parameters<McpSlideCaptureParams>,
    ) -> Result<String, String> {
        let capture = capture_slide_image(
            params.path,
            params.slide,
            params.output_dir,
            params.headed,
        )?;
        serde_json::to_string_pretty(&capture)
            .map_err(|error| format!("Failed to serialize slide capture response: {error}"))
    }

    #[tool(
        name = "get_compile_errors",
        description = "Validate project and return compile/validation errors with best-effort line references."
    )]
    fn tool_get_compile_errors(
        &self,
        Parameters(params): Parameters<McpPathParams>,
    ) -> Result<String, String> {
        let report = validate_project(params.path)?;
        let errors: Vec<_> = report
            .errors
            .iter()
            .map(|message| {
                let (line, column) = parse_line_column(message);
                serde_json::json!({
                    "message": message,
                    "line": line,
                    "column": column,
                })
            })
            .collect();

        let warnings: Vec<_> = report
            .warnings
            .iter()
            .map(|message| {
                let (line, column) = parse_line_column(message);
                serde_json::json!({
                    "message": message,
                    "line": line,
                    "column": column,
                })
            })
            .collect();

        serde_json::to_string_pretty(&serde_json::json!({
            "path": report.path,
            "error_count": errors.len(),
            "warning_count": warnings.len(),
            "errors": errors,
            "warnings": warnings,
        }))
        .map_err(|error| format!("Failed to serialize compile errors: {error}"))
    }
}

#[tool_handler]
impl ServerHandler for FastSlidesMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "FastSlides Desktop MCP server. Use tools to inspect app state and open/validate project folders."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
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

fn markdown_heading_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s{0,3}#{1,3}\s+(.+?)\s*$"#).expect("invalid heading regex")
    })
}

fn html_heading_capture_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?is)<h[1-3][^>]*>(.*?)</h[1-3]>"#).expect("invalid html heading regex")
    })
}

fn takeaway_capture_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?is)<Takeaway\b[^>]*>(.*?)</Takeaway>"#)
            .expect("invalid takeaway heading regex")
    })
}

fn mdx_component_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"<(Stack|Row|Grid|Canvas|Area|Card|Panel|Callout|Metric|Chart|Caption|Kicker|Takeaway|PillRow|Pill|Quote|Rule|Arrow)\b"#,
        )
            .expect("invalid mdx component regex")
    })
}

fn split_class_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"className\s*=\s*["'][^"']*\bsplit\b[^"']*["']"#)
            .expect("invalid split class regex")
    })
}

fn image_ref_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?i)<img\b|!\[[^\]]*\]\("#).expect("invalid image regex"))
}

fn video_ref_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?i)<video\b|poster\s*="#).expect("invalid video regex"))
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
                    pinned_projects: Vec::new(),
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

fn clean_heading_text(raw: &str) -> String {
    let without_tags = html_tag_re().replace_all(raw, " ");
    without_tags
        .replace("**", " ")
        .replace("__", " ")
        .replace('`', " ")
        .replace('*', " ")
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn slide_title_for(slide: &str, index: usize) -> String {
    if let Some(captures) = markdown_heading_re().captures(slide) {
        if let Some(raw_title) = captures
            .get(1)
            .map(|item| clean_heading_text(item.as_str()))
        {
            if !raw_title.is_empty() {
                return raw_title;
            }
        }
    }

    if let Some(captures) = takeaway_capture_re().captures(slide) {
        if let Some(raw_title) = captures
            .get(1)
            .map(|item| clean_heading_text(item.as_str()))
        {
            if !raw_title.is_empty() {
                return raw_title;
            }
        }
    }

    if let Some(captures) = html_heading_capture_re().captures(slide) {
        if let Some(raw_title) = captures
            .get(1)
            .map(|item| clean_heading_text(item.as_str()))
        {
            if !raw_title.is_empty() {
                return raw_title;
            }
        }
    }

    format!("Slide {}", index + 1)
}

fn increment_count(counts: &mut HashMap<String, usize>, key: &str, amount: usize) {
    *counts.entry(key.to_string()).or_insert(0) += amount;
}

fn add_count_if_present(counts: &mut HashMap<String, usize>, key: &str, amount: usize) {
    if amount > 0 {
        increment_count(counts, key, amount);
    }
}

fn sorted_named_counts(counts: HashMap<String, usize>) -> Vec<NamedCount> {
    let mut entries: Vec<_> = counts
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(name, count)| NamedCount { name, count })
        .collect();

    entries.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.name.cmp(&right.name))
    });
    entries
}

fn component_counts_for_slide(slide: &str) -> HashMap<String, usize> {
    let mut counts = HashMap::<String, usize>::new();

    for captures in mdx_component_re().captures_iter(slide) {
        if let Some(name) = captures.get(1).map(|item| item.as_str()) {
            increment_count(&mut counts, name, 1);
        }
    }

    let mermaid_count = slide.matches("```mermaid").count();
    add_count_if_present(&mut counts, "mermaid", mermaid_count);
    let code_fence_count = slide.matches("```").count() / 2;
    add_count_if_present(&mut counts, "code", code_fence_count.saturating_sub(mermaid_count));
    add_count_if_present(&mut counts, "image", image_ref_re().find_iter(slide).count());
    add_count_if_present(&mut counts, "video", video_ref_re().find_iter(slide).count());

    counts
}

fn uses_spatial_canvas(component_counts: &HashMap<String, usize>) -> bool {
    component_counts.get("Canvas").copied().unwrap_or(0) > 0
        && component_counts.get("Area").copied().unwrap_or(0) > 0
}

fn inferred_archetype(
    slide: &str,
    words: usize,
    bullets: usize,
    component_counts: &HashMap<String, usize>,
) -> String {
    let has = |name: &str| component_counts.get(name).copied().unwrap_or(0) > 0;

    if has("mermaid") {
        return "diagram".to_string();
    }
    if has("Chart") {
        return "chart".to_string();
    }
    if has("code") {
        return "code-demo".to_string();
    }
    if has("Metric") {
        return "metrics".to_string();
    }
    if has("Canvas") && has("Area") {
        return "spatial-canvas".to_string();
    }
    if has("Quote") {
        return "quote".to_string();
    }
    if has("Grid") && has("Card") {
        return "card-grid".to_string();
    }
    if SURFACE_COMPONENT_NAMES
        .iter()
        .any(|name| *name != "Quote" && has(name))
    {
        return "structured-brief".to_string();
    }
    if has("Row") && words > 45 {
        return "comparison".to_string();
    }
    if (has("image") || has("video")) && words <= 90 {
        return "visual-explainer".to_string();
    }
    if words <= 40 && bullets == 0 {
        return "hero".to_string();
    }
    if bullets >= 3 {
        return "bullet-brief".to_string();
    }
    if slide.contains("<Card")
        || slide.contains("<Panel")
        || slide.contains("<Callout")
        || slide.contains("<Metric")
    {
        return "structured-brief".to_string();
    }
    "narrative".to_string()
}

fn size(cols: usize, rows: usize) -> AreaSizeSpec {
    AreaSizeSpec { cols, rows }
}

fn named_spec_list<T>(items: Vec<T>, name_of: impl Fn(&T) -> &str) -> Vec<String> {
    items.into_iter()
        .map(|item| name_of(&item).to_string())
        .collect()
}

fn default_design_frame() -> DesignFrameSpec {
    DesignFrameSpec {
        cols: 50,
        rows: 25,
        header_rows: 2,
        body_rows: 21,
        footer_rows: 2,
        body_slices: vec![
            BodySliceSpec {
                name: "hero".to_string(),
                min_rows: 4,
                preferred_rows: 4,
                purpose: "Primary conclusion or title band.".to_string(),
            },
            BodySliceSpec {
                name: "primary".to_string(),
                min_rows: 8,
                preferred_rows: 9,
                purpose: "Main exhibit or first composition row.".to_string(),
            },
            BodySliceSpec {
                name: "secondary".to_string(),
                min_rows: 5,
                preferred_rows: 5,
                purpose: "Supporting comparison, KPIs, or commentary.".to_string(),
            },
            BodySliceSpec {
                name: "note".to_string(),
                min_rows: 2,
                preferred_rows: 3,
                purpose: "Caption, source note, or operator footer within the body band."
                    .to_string(),
            },
        ],
        notes: vec![
            "Keep chrome thin. Header and footer should frame the page, not consume it."
                .to_string(),
            "Treat the body as the design surface. Compose it with 2 to 4 deliberate modules."
                .to_string(),
        ],
    }
}

fn design_system_registry() -> DesignSystemRegistry {
    let default_frame = default_design_frame();

    let primitives = vec![
        PrimitiveSpec {
            name: "Kicker".to_string(),
            purpose: "Thin meta label that names the story without competing with the main point."
                .to_string(),
            variants: vec!["default".to_string()],
            min_area: Some(size(10, 1)),
            preferred_area: Some(size(14, 1)),
            allowed_parents: vec!["Area".to_string(), "Panel".to_string()],
            notes: vec!["Use as top-line chrome only.".to_string()],
        },
        PrimitiveSpec {
            name: "Takeaway".to_string(),
            purpose: "Main conclusion band. This is the strongest sentence on the slide."
                .to_string(),
            variants: vec![
                "compact".to_string(),
                "balanced".to_string(),
                "hero".to_string(),
            ],
            min_area: Some(size(36, 4)),
            preferred_area: Some(size(44, 4)),
            allowed_parents: vec!["Area".to_string()],
            notes: vec![
                "Prefer one sentence.".to_string(),
                "Use the full hero band before shrinking the title into a narrow left column."
                    .to_string(),
            ],
        },
        PrimitiveSpec {
            name: "Panel".to_string(),
            purpose: "Structured evidence block for lists, code, tables, or mixed narrative."
                .to_string(),
            variants: vec![
                "compact".to_string(),
                "standard".to_string(),
                "roomy".to_string(),
            ],
            min_area: Some(size(14, 9)),
            preferred_area: Some(size(18, 11)),
            allowed_parents: vec!["Area".to_string(), "Grid".to_string()],
            notes: vec!["Use this as the default evidence container.".to_string()],
        },
        PrimitiveSpec {
            name: "Metric".to_string(),
            purpose: "Compact KPI tile for one value, one label, and a small hint.".to_string(),
            variants: vec![
                "compact".to_string(),
                "standard".to_string(),
                "feature".to_string(),
            ],
            min_area: Some(size(10, 5)),
            preferred_area: Some(size(11, 5)),
            allowed_parents: vec!["Area".to_string(), "Grid".to_string()],
            notes: vec![
                "Do not use long narrative copy inside a metric.".to_string(),
                "Three-up scorecards need around 30 columns of total width.".to_string(),
            ],
        },
        PrimitiveSpec {
            name: "Chart".to_string(),
            purpose: "Single-series analytical chart that renders in the slide theme without raw SVG handwork."
                .to_string(),
            variants: vec!["bar".to_string(), "trend".to_string()],
            min_area: Some(size(18, 8)),
            preferred_area: Some(size(30, 11)),
            allowed_parents: vec!["Area".to_string(), "Panel".to_string()],
            notes: vec![
                "Keep charts focused: one series, 3 to 8 points.".to_string(),
                "Use a panel only when the chart also needs a narrative title or footnote."
                    .to_string(),
            ],
        },
        PrimitiveSpec {
            name: "Rule".to_string(),
            purpose: "Thin divider or anchor line that adds structure without another box."
                .to_string(),
            variants: vec!["default".to_string()],
            min_area: Some(size(8, 1)),
            preferred_area: Some(size(18, 1)),
            allowed_parents: vec!["Area".to_string()],
            notes: vec!["Use for rhythm, grouping, or section breaks.".to_string()],
        },
        PrimitiveSpec {
            name: "Arrow".to_string(),
            purpose: "Directional mark that connects two regions or indicates flow."
                .to_string(),
            variants: vec![
                "right".to_string(),
                "left".to_string(),
                "down".to_string(),
            ],
            min_area: Some(size(8, 2)),
            preferred_area: Some(size(14, 2)),
            allowed_parents: vec!["Area".to_string()],
            notes: vec![
                "Prefer one arrow over many.".to_string(),
                "Use to connect modules, not decorate them.".to_string(),
            ],
        },
        PrimitiveSpec {
            name: "Callout".to_string(),
            purpose: "Interpretive rail that explains why the evidence matters.".to_string(),
            variants: vec![
                "compact".to_string(),
                "rail".to_string(),
                "standard".to_string(),
            ],
            min_area: Some(size(13, 9)),
            preferred_area: Some(size(14, 10)),
            allowed_parents: vec!["Area".to_string()],
            notes: vec![
                "Keep it short enough to read in one glance.".to_string(),
                "If it grows, widen the rail or turn it into a panel.".to_string(),
            ],
        },
        PrimitiveSpec {
            name: "Caption".to_string(),
            purpose: "Thin note or source line that should live near the bottom of the body band."
                .to_string(),
            variants: vec!["default".to_string()],
            min_area: Some(size(12, 1)),
            preferred_area: Some(size(18, 1)),
            allowed_parents: vec!["Area".to_string()],
            notes: vec!["Footer copy should be quiet and low-height.".to_string()],
        },
    ];

    let compositions = vec![
        CompositionSpec {
            name: "TakeawayRail".to_string(),
            purpose: "One takeaway band with one interpretation rail.".to_string(),
            variants: vec!["standard".to_string(), "executive".to_string()],
            min_area: size(50, 16),
            preferred_area: size(50, 19),
            source_primitives: vec![
                "Kicker".to_string(),
                "Takeaway".to_string(),
                "Callout".to_string(),
                "Caption".to_string(),
            ],
            slots: vec![
                SlotSpec {
                    name: "takeaway".to_string(),
                    accepts: vec!["Takeaway".to_string()],
                    min: 1,
                    max: 1,
                    note: Some("Primary conclusion band.".to_string()),
                },
                SlotSpec {
                    name: "rail".to_string(),
                    accepts: vec!["Callout".to_string()],
                    min: 1,
                    max: 1,
                    note: Some("Interpretive commentary.".to_string()),
                },
            ],
            notes: vec!["Default opener pattern.".to_string()],
        },
        CompositionSpec {
            name: "MetricStrip".to_string(),
            purpose: "Two to four metrics in a controlled scorecard row.".to_string(),
            variants: vec![
                "compact".to_string(),
                "standard".to_string(),
                "executive".to_string(),
            ],
            min_area: size(24, 5),
            preferred_area: size(30, 6),
            source_primitives: vec!["Metric".to_string(), "Caption".to_string()],
            slots: vec![SlotSpec {
                name: "items".to_string(),
                accepts: vec!["Metric".to_string()],
                min: 2,
                max: 4,
                note: Some("Fewer metrics are stronger than many tiny ones.".to_string()),
            }],
            notes: vec!["Default scorecard module.".to_string()],
        },
        CompositionSpec {
            name: "ExhibitCommentary".to_string(),
            purpose: "One evidence panel paired with one commentary rail.".to_string(),
            variants: vec!["standard".to_string(), "chart".to_string()],
            min_area: size(50, 14),
            preferred_area: size(50, 17),
            source_primitives: vec![
                "Panel".to_string(),
                "Chart".to_string(),
                "Callout".to_string(),
                "Caption".to_string(),
            ],
            slots: vec![
                SlotSpec {
                    name: "exhibit".to_string(),
                    accepts: vec!["Panel".to_string(), "Chart".to_string()],
                    min: 1,
                    max: 1,
                    note: Some("Table, code, chart, or structured evidence.".to_string()),
                },
                SlotSpec {
                    name: "commentary".to_string(),
                    accepts: vec!["Callout".to_string()],
                    min: 1,
                    max: 1,
                    note: Some("Short interpretation.".to_string()),
                },
            ],
            notes: vec!["Best for one main exhibit per slide.".to_string()],
        },
        CompositionSpec {
            name: "ThreeUpPanels".to_string(),
            purpose: "Three parallel panels for compare, problem/move/outcome, or options."
                .to_string(),
            variants: vec!["standard".to_string(), "compare".to_string()],
            min_area: size(44, 10),
            preferred_area: size(46, 11),
            source_primitives: vec!["Panel".to_string()],
            slots: vec![SlotSpec {
                name: "panels".to_string(),
                accepts: vec!["Panel".to_string()],
                min: 3,
                max: 3,
                note: Some("Parallel structure only.".to_string()),
            }],
            notes: vec!["Use when symmetry is the point.".to_string()],
        },
        CompositionSpec {
            name: "KpiPair".to_string(),
            purpose: "Two stacked metrics beside a supporting note or exhibit.".to_string(),
            variants: vec!["stacked".to_string(), "rail".to_string()],
            min_area: size(20, 11),
            preferred_area: size(22, 11),
            source_primitives: vec![
                "Metric".to_string(),
                "Callout".to_string(),
                "Caption".to_string(),
            ],
            slots: vec![
                SlotSpec {
                    name: "metrics".to_string(),
                    accepts: vec!["Metric".to_string()],
                    min: 2,
                    max: 2,
                    note: Some("Two related KPIs only.".to_string()),
                },
                SlotSpec {
                    name: "note".to_string(),
                    accepts: vec!["Callout".to_string(), "Caption".to_string()],
                    min: 0,
                    max: 1,
                    note: Some("Optional interpretation or source line.".to_string()),
                },
            ],
            notes: vec!["Use for side modules, not the whole story.".to_string()],
        },
    ];

    let recipes = vec![
        RecipeSpec {
            name: "takeaway_plus_rail".to_string(),
            summary: "Default opening slide with one conclusion and one side interpretation."
                .to_string(),
            frame: default_frame.clone(),
            compositions: vec!["TakeawayRail".to_string()],
            notes: vec![
                "Thin header and footer; most rows belong to the body.".to_string(),
            ],
        },
        RecipeSpec {
            name: "scorecard_with_note".to_string(),
            summary: "One takeaway band, one metric strip, and one note rail.".to_string(),
            frame: default_frame.clone(),
            compositions: vec!["MetricStrip".to_string(), "TakeawayRail".to_string()],
            notes: vec!["Good for status and executive summaries.".to_string()],
        },
        RecipeSpec {
            name: "exhibit_left_commentary_right".to_string(),
            summary: "Primary evidence on the left with commentary on the right.".to_string(),
            frame: default_frame.clone(),
            compositions: vec!["ExhibitCommentary".to_string()],
            notes: vec!["The most reliable analytic-slide pattern.".to_string()],
        },
        RecipeSpec {
            name: "three_up_compare".to_string(),
            summary: "Three parallel panels under one takeaway.".to_string(),
            frame: default_frame.clone(),
            compositions: vec!["ThreeUpPanels".to_string()],
            notes: vec!["Use when the slide needs equal-weight comparison.".to_string()],
        },
        RecipeSpec {
            name: "kpi_pair_with_exhibit".to_string(),
            summary: "Two KPIs paired with a supporting panel or note.".to_string(),
            frame: default_frame.clone(),
            compositions: vec!["KpiPair".to_string(), "ExhibitCommentary".to_string()],
            notes: vec!["Useful for dashboard-like pages with one side story.".to_string()],
        },
    ];

    let sections = vec![
        SectionSpec {
            name: "Situation -> Evidence -> Recommendation".to_string(),
            summary: "Classic consulting arc with one opener, one proof slide, and one action page."
                .to_string(),
            recipes: vec![
                "takeaway_plus_rail".to_string(),
                "exhibit_left_commentary_right".to_string(),
                "scorecard_with_note".to_string(),
            ],
        },
        SectionSpec {
            name: "Problem -> Options -> Decision".to_string(),
            summary: "Use a compare slide between context and chosen path.".to_string(),
            recipes: vec![
                "takeaway_plus_rail".to_string(),
                "three_up_compare".to_string(),
                "scorecard_with_note".to_string(),
            ],
        },
        SectionSpec {
            name: "Dashboard -> Insight -> Operator Note".to_string(),
            summary: "KPI-heavy flow for product and operational reviews.".to_string(),
            recipes: vec![
                "scorecard_with_note".to_string(),
                "kpi_pair_with_exhibit".to_string(),
                "takeaway_plus_rail".to_string(),
            ],
        },
    ];

    DesignSystemRegistry {
        version: "2.0-base".to_string(),
        philosophy: vec![
            "Use few expressive modules instead of many decorative variants.".to_string(),
            "Keep header and footer thin so the body owns the page.".to_string(),
            "Compose slides from modules first; use raw areas only as an escape hatch.".to_string(),
        ],
        default_frame,
        primitives,
        compositions,
        recipes,
        sections,
    }
}

fn known_primitive_names() -> Vec<String> {
    named_spec_list(design_system_registry().primitives, |primitive| &primitive.name)
}

fn known_composition_names() -> Vec<String> {
    named_spec_list(design_system_registry().compositions, |composition| &composition.name)
}

fn known_recipe_names() -> Vec<String> {
    named_spec_list(design_system_registry().recipes, |recipe| &recipe.name)
}

fn primitive_template(name: &str) -> Result<DesignTemplate, String> {
    match name.trim() {
        "Kicker" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Kicker".to_string(),
            mdx: r#"<Area x={2} y={2} w={14} h={1}>
  <Kicker>Section label</Kicker>
</Area>"#
                .to_string(),
            notes: vec!["Use as quiet header chrome only.".to_string()],
        }),
        "Takeaway" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Takeaway".to_string(),
            mdx: r#"<Area x={2} y={4} w={46} h={4}>
  <Takeaway>Replace this with the single conclusion for the slide.</Takeaway>
</Area>"#
                .to_string(),
            notes: vec!["Use the full hero band by default.".to_string()],
        }),
        "Panel" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Panel".to_string(),
            mdx: r#"<Area x={2} y={10} w={18} h={11}>
  <Panel title="Evidence" tone="accent">
    Replace with one structured block of evidence.
  </Panel>
</Area>"#
                .to_string(),
            notes: vec!["Use as the default evidence container.".to_string()],
        }),
        "Metric" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Metric".to_string(),
            mdx: r#"<Area x={2} y={11} w={10} h={5}>
  <Metric label="Metric" value="42%" hint="Short note" />
</Area>"#
                .to_string(),
            notes: vec!["Keep values short and hints quiet.".to_string()],
        }),
        "Chart" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Chart".to_string(),
            mdx: r#"<Area x={2} y={10} w={30} h={11}>
  <Chart
    type="bar"
    title="Exhibit"
    data="Option A:72;Option B:54;Option C:39"
    suffix="%"
    highlight="Option A"
  />
</Area>"#
                .to_string(),
            notes: vec!["Use one chart per exhibit area.".to_string()],
        }),
        "Rule" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Rule".to_string(),
            mdx: r#"<Area x={2} y={20} w={18} h={1}>
  <Rule />
</Area>"#
                .to_string(),
            notes: vec!["Good for separating evidence rows or notes.".to_string()],
        }),
        "Arrow" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Arrow".to_string(),
            mdx: r#"<Area x={22} y={13} w={12} h={2}>
  <Arrow direction="right" label="Flow of work" tone="accent" />
</Area>"#
                .to_string(),
            notes: vec!["Use to connect modules or show sequence.".to_string()],
        }),
        "Callout" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Callout".to_string(),
            mdx: r#"<Area x={34} y={10} w={14} h={10}>
  <Callout title="Interpretation" tone="accent">
    Explain why the evidence matters.
  </Callout>
</Area>"#
                .to_string(),
            notes: vec!["Keep it short enough to read in one glance.".to_string()],
        }),
        "Caption" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Caption".to_string(),
            mdx: r#"<Area x={2} y={23} w={18} h={1}>
  <Caption>Source or operator note</Caption>
</Area>"#
                .to_string(),
            notes: vec!["Use as low-height footer copy inside the body band.".to_string()],
        }),
        _ => Err(format!(
            "Unknown primitive `{}`. Available primitives: {}.",
            name,
            known_primitive_names().join(", ")
        )),
    }
}

fn component_pattern_entries() -> Vec<ComponentCatalogEntry> {
    vec![
        ComponentCatalogEntry {
            name: "ImageFigure".to_string(),
            family: "media".to_string(),
            kind: "pattern".to_string(),
            scope: "builtin".to_string(),
            summary: "One image with a quiet caption below it.".to_string(),
            tags: vec!["image".to_string(), "caption".to_string()],
        },
        ComponentCatalogEntry {
            name: "LogoStrip".to_string(),
            family: "media".to_string(),
            kind: "pattern".to_string(),
            scope: "builtin".to_string(),
            summary: "Three to five visual references in one row.".to_string(),
            tags: vec!["image".to_string(), "row".to_string(), "reference".to_string()],
        },
        ComponentCatalogEntry {
            name: "BarChartCommentary".to_string(),
            family: "exhibit".to_string(),
            kind: "pattern".to_string(),
            scope: "builtin".to_string(),
            summary: "One bar chart with one interpretation rail.".to_string(),
            tags: vec!["chart".to_string(), "bar".to_string(), "callout".to_string()],
        },
        ComponentCatalogEntry {
            name: "TrendChartCommentary".to_string(),
            family: "exhibit".to_string(),
            kind: "pattern".to_string(),
            scope: "builtin".to_string(),
            summary: "One trend chart with one interpretation rail.".to_string(),
            tags: vec!["chart".to_string(), "trend".to_string(), "callout".to_string()],
        },
        ComponentCatalogEntry {
            name: "ArrowBridge".to_string(),
            family: "mark".to_string(),
            kind: "pattern".to_string(),
            scope: "builtin".to_string(),
            summary: "Use one arrow to bridge two modules.".to_string(),
            tags: vec!["arrow".to_string(), "flow".to_string(), "connection".to_string()],
        },
    ]
}

fn component_pattern_template(name: &str) -> Option<DesignTemplate> {
    match name.trim() {
        "ImageFigure" => Some(DesignTemplate {
            kind: "pattern".to_string(),
            name: "ImageFigure".to_string(),
            mdx: r#"<Area x={2} y={10} w={24} h={10}>
  ![Replace with figure](./assets/figure.png)
</Area>

<Area x={2} y={21} w={24} h={1}>
  <Caption>Replace with figure caption</Caption>
</Area>"#
                .to_string(),
            notes: vec!["Replace the image path with a real project asset.".to_string()],
        }),
        "LogoStrip" => Some(DesignTemplate {
            kind: "pattern".to_string(),
            name: "LogoStrip".to_string(),
            mdx: r#"<Area x={2} y={11} w={46} h={5}>
  <Row gap="sm" align="stretch">
    ![Reference one](./assets/ref-1.png)
    ![Reference two](./assets/ref-2.png)
    ![Reference three](./assets/ref-3.png)
  </Row>
</Area>"#
                .to_string(),
            notes: vec!["Use this for logos, interfaces, or visual references.".to_string()],
        }),
        "BarChartCommentary" => Some(DesignTemplate {
            kind: "pattern".to_string(),
            name: "BarChartCommentary".to_string(),
            mdx: r#"<Area x={2} y={10} w={30} h={11}>
  <Chart
    type="bar"
    title="Exhibit"
    data="Option A:72;Option B:54;Option C:39"
    suffix="%"
    highlight="Option A"
  />
</Area>

<Area x={34} y={10} w={14} h={11}>
  <Callout title="Interpretation">
    Explain the one thing the audience should learn from the chart.
  </Callout>
</Area>"#
                .to_string(),
            notes: vec!["This is the default chart pattern.".to_string()],
        }),
        "TrendChartCommentary" => Some(DesignTemplate {
            kind: "pattern".to_string(),
            name: "TrendChartCommentary".to_string(),
            mdx: r#"<Area x={2} y={10} w={30} h={11}>
  <Chart
    type="trend"
    title="Trend"
    data="Baseline:42;Telemetry:58;Memory:71;Eval loop:84"
    highlight="Eval loop"
  />
</Area>

<Area x={34} y={10} w={14} h={11}>
  <Callout title="Interpretation">
    Explain what is compounding and why it matters.
  </Callout>
</Area>"#
                .to_string(),
            notes: vec!["Use for one directional story only.".to_string()],
        }),
        "ArrowBridge" => Some(DesignTemplate {
            kind: "pattern".to_string(),
            name: "ArrowBridge".to_string(),
            mdx: r#"<Area x={16} y={13} w={18} h={2}>
  <Arrow direction="right" label="Connect the modules" tone="accent" />
</Area>"#
                .to_string(),
            notes: vec!["Use between two modules, not as standalone decoration.".to_string()],
        }),
        _ => None,
    }
}

fn composition_template(name: &str) -> Result<DesignTemplate, String> {
    match name.trim() {
        "TakeawayRail" => Ok(DesignTemplate {
            kind: "composition".to_string(),
            name: "TakeawayRail".to_string(),
            mdx: r#"<Area x={2} y={2} w={14} h={1}>
  <Kicker>Section</Kicker>
</Area>

<Area x={2} y={4} w={46} h={4}>
  <Takeaway>Replace this with the main conclusion for the slide.</Takeaway>
</Area>

<Area x={34} y={10} w={14} h={11}>
  <Callout title="Interpretation" tone="accent">
    Explain why the takeaway matters in one short paragraph.
  </Callout>
</Area>

<Area x={2} y={23} w={12} h={1}>
  <Caption>Source or operator note</Caption>
</Area>"#
                .to_string(),
            notes: vec![
                "Paste this inside a Canvas.".to_string(),
                "Treat it as the default opener cluster.".to_string(),
            ],
        }),
        "MetricStrip" => Ok(DesignTemplate {
            kind: "composition".to_string(),
            name: "MetricStrip".to_string(),
            mdx: r#"<Area x={2} y={11} w={30} h={6}>
  <Grid cols={3} gap="sm">
    <Metric label="Metric A" value="12%" hint="Short supporting note" />
    <Metric label="Metric B" value="3.4x" hint="Short supporting note" />
    <Metric label="Metric C" value="24d" hint="Short supporting note" />
  </Grid>
</Area>"#
                .to_string(),
            notes: vec![
                "Use two to four metrics only.".to_string(),
                "If values get long, widen the strip or reduce columns.".to_string(),
            ],
        }),
        "ExhibitCommentary" => Ok(DesignTemplate {
            kind: "composition".to_string(),
            name: "ExhibitCommentary".to_string(),
            mdx: r#"<Area x={2} y={10} w={30} h={11}>
  <Panel title="Exhibit">
    | Workflow | Owner | Signal |
    | --- | --- | --- |
    | Discovery | PM | Growing usage |
    | Execution | Ops | High automation fit |
    | Review | Lead | Needs human override |
  </Panel>
</Area>

<Area x={34} y={10} w={14} h={11}>
  <Callout title="Interpretation">
    Explain the one thing the audience should take away from the exhibit.
  </Callout>
</Area>"#
                .to_string(),
            notes: vec![
                "Best when there is one clear exhibit.".to_string(),
                "Do not split the story across multiple equal-weight charts.".to_string(),
            ],
        }),
        "ThreeUpPanels" => Ok(DesignTemplate {
            kind: "composition".to_string(),
            name: "ThreeUpPanels".to_string(),
            mdx: r#"<Area x={2} y={10} w={14} h={12}>
  <Panel title="Column one" tone="accent">
    Replace with the first parallel point.
  </Panel>
</Area>

<Area x={18} y={10} w={14} h={12}>
  <Panel title="Column two">
    Replace with the second parallel point.
  </Panel>
</Area>

<Area x={34} y={10} w={14} h={12}>
  <Panel title="Column three">
    Replace with the third parallel point.
  </Panel>
</Area>"#
                .to_string(),
            notes: vec![
                "Use only when all three columns deserve equal weight.".to_string(),
            ],
        }),
        "KpiPair" => Ok(DesignTemplate {
            kind: "composition".to_string(),
            name: "KpiPair".to_string(),
            mdx: r#"<Area x={23} y={10} w={10} h={5}>
  <Metric label="KPI one" value="42%" hint="Short note" />
</Area>

<Area x={23} y={16} w={10} h={5}>
  <Metric label="KPI two" value="18d" hint="Short note" />
</Area>

<Area x={35} y={10} w={13} h={11}>
  <Callout title="Operator note">
    Add the interpretation, risk, or decision implied by the two KPIs.
  </Callout>
</Area>"#
                .to_string(),
            notes: vec![
                "Use for side modules, not the main story.".to_string(),
            ],
        }),
        _ => Err(format!(
            "Unknown composition `{}`. Available compositions: {}.",
            name,
            known_composition_names().join(", ")
        )),
    }
}

fn recipe_template(name: &str) -> Result<DesignTemplate, String> {
    match name.trim() {
        "takeaway_plus_rail" => Ok(DesignTemplate {
            kind: "recipe".to_string(),
            name: "takeaway_plus_rail".to_string(),
            mdx: format!(
                "<section className=\"slide\">\n  <Canvas cols={{50}} rows={{25}} gap=\"1px\">\n{}\n  </Canvas>\n</section>",
                composition_template("TakeawayRail")?.mdx
            ),
            notes: vec![
                "Default opener recipe.".to_string(),
                "Body-first frame: keep header/footer thin.".to_string(),
            ],
        }),
        "scorecard_with_note" => Ok(DesignTemplate {
            kind: "recipe".to_string(),
            name: "scorecard_with_note".to_string(),
            mdx: r#"<section className="slide">
  <Canvas cols={50} rows={25} gap="1px">
    <Area x={2} y={2} w={14} h={1}>
      <Kicker>Status</Kicker>
    </Area>

    <Area x={2} y={4} w={46} h={4}>
      <Takeaway>Replace this with one conclusion supported by a small scorecard.</Takeaway>
    </Area>

    <Area x={2} y={11} w={30} h={6}>
      <Grid cols={3} gap="sm">
        <Metric label="Metric A" value="12%" hint="Short note" />
        <Metric label="Metric B" value="3.4x" hint="Short note" />
        <Metric label="Metric C" value="24d" hint="Short note" />
      </Grid>
    </Area>

    <Area x={34} y={11} w={14} h={8}>
      <Callout title="Interpretation" tone="accent">
        Explain the scorecard in one sentence.
      </Callout>
    </Area>

    <Area x={2} y={23} w={16} h={1}>
      <Caption>Optional source note</Caption>
    </Area>
  </Canvas>
</section>"#
                .to_string(),
            notes: vec!["Use for executive progress and dashboard slides.".to_string()],
        }),
        "exhibit_left_commentary_right" => Ok(DesignTemplate {
            kind: "recipe".to_string(),
            name: "exhibit_left_commentary_right".to_string(),
            mdx: r#"<section className="slide">
  <Canvas cols={50} rows={25} gap="1px">
    <Area x={2} y={2} w={14} h={1}>
      <Kicker>Evidence</Kicker>
    </Area>

    <Area x={2} y={4} w={46} h={4}>
      <Takeaway>Replace this with the single conclusion the exhibit should prove.</Takeaway>
    </Area>

    <Area x={2} y={10} w={30} h={11}>
      <Chart
        type="bar"
        title="Exhibit"
        data="Option A:72;Option B:54;Option C:39"
        suffix="%"
        highlight="Option A"
      />
    </Area>

    <Area x={34} y={10} w={14} h={11}>
      <Callout title="Interpretation">
        Explain why the exhibit matters and what decision it supports.
      </Callout>
    </Area>

    <Area x={2} y={23} w={16} h={1}>
      <Caption>Optional source note</Caption>
    </Area>
  </Canvas>
</section>"#
                .to_string(),
            notes: vec!["This should be the default analytical recipe.".to_string()],
        }),
        "three_up_compare" => Ok(DesignTemplate {
            kind: "recipe".to_string(),
            name: "three_up_compare".to_string(),
            mdx: r#"<section className="slide">
  <Canvas cols={50} rows={25} gap="1px">
    <Area x={2} y={2} w={14} h={1}>
      <Kicker>Compare</Kicker>
    </Area>

    <Area x={2} y={4} w={46} h={4}>
      <Takeaway>Replace with the one conclusion that frames all three columns.</Takeaway>
    </Area>

    <Area x={2} y={11} w={14} h={11}>
      <Panel title="Column one" tone="accent">
        Replace with the first parallel point.
      </Panel>
    </Area>

    <Area x={18} y={11} w={14} h={11}>
      <Panel title="Column two">
        Replace with the second parallel point.
      </Panel>
    </Area>

    <Area x={34} y={11} w={14} h={11}>
      <Panel title="Column three">
        Replace with the third parallel point.
      </Panel>
    </Area>
  </Canvas>
</section>"#
                .to_string(),
            notes: vec!["Use only when symmetry is the story.".to_string()],
        }),
        "kpi_pair_with_exhibit" => Ok(DesignTemplate {
            kind: "recipe".to_string(),
            name: "kpi_pair_with_exhibit".to_string(),
            mdx: r#"<section className="slide">
  <Canvas cols={50} rows={25} gap="1px">
    <Area x={2} y={2} w={14} h={1}>
      <Kicker>Dashboard</Kicker>
    </Area>

    <Area x={2} y={4} w={46} h={4}>
      <Takeaway>Replace with the conclusion the KPIs and exhibit should prove.</Takeaway>
    </Area>

    <Area x={2} y={10} w={18} h={11}>
      <Chart
        type="bar"
        title="Supporting exhibit"
        data="Segment A:47;Segment B:38;Segment C:29;Segment D:21"
        suffix="%"
        highlight="Segment A"
      />
    </Area>

    <Area x={23} y={10} w={10} h={5}>
      <Metric label="KPI one" value="42%" hint="Short note" />
    </Area>

    <Area x={23} y={16} w={10} h={5}>
      <Metric label="KPI two" value="18d" hint="Short note" />
    </Area>

    <Area x={34} y={10} w={14} h={11}>
      <Callout title="Interpretation">
        Explain the decision or risk implied by the exhibit and KPIs.
      </Callout>
    </Area>
  </Canvas>
</section>"#
                .to_string(),
            notes: vec!["Good for operational or product reviews.".to_string()],
        }),
        _ => Err(format!(
            "Unknown recipe `{}`. Available recipes: {}.",
            name,
            known_recipe_names().join(", ")
        )),
    }
}

fn primitive_family(name: &str) -> &'static str {
    match name {
        "Kicker" | "Takeaway" | "Caption" => "narrative",
        "Panel" | "Callout" | "Metric" => "container",
        "Chart" => "exhibit",
        "Rule" | "Arrow" => "mark",
        _ => "primitive",
    }
}

fn composition_family(name: &str) -> &'static str {
    match name {
        "TakeawayRail" => "narrative",
        "MetricStrip" | "KpiPair" => "scorecard",
        "ExhibitCommentary" => "exhibit",
        "ThreeUpPanels" => "compare",
        _ => "composition",
    }
}

fn component_catalog_path() -> Result<PathBuf, String> {
    if let Ok(explicit) = env::var("FASTSLIDES_COMPONENT_LIBRARY_PATH") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return Ok(expand_user_path(trimmed));
        }
    }
    let home = env::var("HOME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| "Could not resolve HOME for component library.".to_string())?;
    Ok(home.join(".fastslides").join("component-library.json"))
}

fn load_saved_components_from(path: &Path) -> Result<Vec<SavedComponentRecord>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read component library {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("Failed to parse component library {}: {error}", path.display()))
}

fn save_component_to_path(path: &Path, payload: SaveComponentPayload) -> Result<SaveComponentResponse, String> {
    let name = payload.name.trim().to_string();
    let family = payload.family.trim().to_string();
    let summary = payload.summary.trim().to_string();
    let mdx = payload.mdx.trim().to_string();
    if name.is_empty() || family.is_empty() || summary.is_empty() || mdx.is_empty() {
        return Err("Saved components require non-empty name, family, summary, and mdx.".to_string());
    }

    let parent = path.parent().ok_or_else(|| {
        format!(
            "Component library path has no parent directory: {}",
            path.display()
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Failed to create component library directory {}: {error}",
            parent.display()
        )
    })?;

    let mut records = load_saved_components_from(path)?;
    let record = SavedComponentRecord {
        name: name.clone(),
        family: family.clone(),
        summary: summary.clone(),
        tags: payload.tags.unwrap_or_default(),
        mdx,
        notes: payload.notes.unwrap_or_default(),
    };

    if let Some(existing) = records
        .iter_mut()
        .find(|existing| existing.name.eq_ignore_ascii_case(&name))
    {
        *existing = record.clone();
    } else {
        records.push(record.clone());
    }
    records.sort_by(|left, right| {
        left.family
            .cmp(&right.family)
            .then_with(|| left.name.cmp(&right.name))
    });
    let content = serde_json::to_string_pretty(&records)
        .map_err(|error| format!("Failed to serialize component library: {error}"))?;
    fs::write(path, content)
        .map_err(|error| format!("Failed to write component library {}: {error}", path.display()))?;

    Ok(SaveComponentResponse {
        ok: true,
        component: ComponentCatalogEntry {
            name: record.name,
            family: record.family,
            kind: "saved-component".to_string(),
            scope: "saved".to_string(),
            summary: record.summary,
            tags: record.tags,
        },
        library_path: path_to_string(path),
    })
}

fn component_catalog_entries() -> Result<Vec<ComponentCatalogEntry>, String> {
    let registry = design_system_registry();
    let mut entries = Vec::<ComponentCatalogEntry>::new();

    entries.extend(registry.primitives.into_iter().map(|primitive| ComponentCatalogEntry {
        name: primitive.name.clone(),
        family: primitive_family(&primitive.name).to_string(),
        kind: "primitive".to_string(),
        scope: "builtin".to_string(),
        summary: primitive.purpose,
        tags: primitive.variants,
    }));
    entries.extend(registry.compositions.into_iter().map(|composition| ComponentCatalogEntry {
        name: composition.name.clone(),
        family: composition_family(&composition.name).to_string(),
        kind: "composition".to_string(),
        scope: "builtin".to_string(),
        summary: composition.purpose,
        tags: composition.source_primitives,
    }));
    entries.extend(registry.recipes.into_iter().map(|recipe| ComponentCatalogEntry {
        name: recipe.name,
        family: "recipe".to_string(),
        kind: "recipe".to_string(),
        scope: "builtin".to_string(),
        summary: recipe.summary,
        tags: recipe.compositions,
    }));
    entries.extend(component_pattern_entries());

    if let Ok(path) = component_catalog_path() {
        entries.extend(load_saved_components_from(&path)?.into_iter().map(|record| {
            ComponentCatalogEntry {
                name: record.name,
                family: record.family,
                kind: "saved-component".to_string(),
                scope: "saved".to_string(),
                summary: record.summary,
                tags: record.tags,
            }
        }));
    }

    entries.sort_by(|left, right| {
        left.family
            .cmp(&right.family)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(entries)
}

fn component_template(name: &str) -> Result<DesignTemplate, String> {
    primitive_template(name)
        .or_else(|_| composition_template(name))
        .or_else(|_| recipe_template(name))
        .or_else(|_| {
            component_pattern_template(name).ok_or_else(|| {
                format!("Unknown component `{name}`.")
            })
        })
        .or_else(|_| {
            let path = component_catalog_path()?;
            let record = load_saved_components_from(&path)?
                .into_iter()
                .find(|component| component.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| {
                    format!(
                        "Unknown component `{}`. Query the component catalog for available built-ins and saved snippets.",
                        name
                    )
                })?;
            Ok(DesignTemplate {
                kind: "saved-component".to_string(),
                name: record.name,
                mdx: record.mdx,
                notes: record.notes,
            })
        })
}

fn build_component_catalog() -> Result<ComponentCatalog, String> {
    Ok(ComponentCatalog {
        version: "1.0".to_string(),
        items: component_catalog_entries()?,
    })
}

fn push_contract_warning(
    seen: &mut HashSet<String>,
    warnings: &mut Vec<String>,
    message: String,
) {
    if seen.insert(message.clone()) {
        warnings.push(message);
    }
}

fn scene_nodes_text_length(nodes: &[SceneNode]) -> usize {
    nodes.iter()
        .map(|node| match node {
            SceneNode::Canvas { children, .. }
            | SceneNode::Area { children, .. }
            | SceneNode::LayoutGroup { children, .. }
            | SceneNode::Surface { children, .. } => scene_nodes_text_length(children),
            SceneNode::Metric {
                label, value, hint, ..
            } => label.as_deref().unwrap_or_default().chars().count()
                + value.as_deref().unwrap_or_default().chars().count()
                + hint.as_deref().unwrap_or_default().chars().count(),
            SceneNode::Chart {
                title,
                data,
                value_suffix,
                ..
            } => {
                title.as_deref().unwrap_or_default().chars().count()
                    + value_suffix.as_deref().unwrap_or_default().chars().count()
                    + data.iter().map(|item| item.label.chars().count()).sum::<usize>()
            }
            SceneNode::Text { text, .. } => text.chars().count(),
            SceneNode::List { items, .. } => items.iter().map(|item| item.chars().count()).sum(),
            SceneNode::Media { alt, .. } => alt
                .as_deref()
                .unwrap_or_default()
                .chars()
                .count(),
            SceneNode::Arrow { label, .. } => label
                .as_deref()
                .unwrap_or_default()
                .chars()
                .count(),
            SceneNode::CodeBlock { code, .. } | SceneNode::Raw { text: code, .. } => {
                code.chars().count()
            }
            SceneNode::Pill { text, .. } => text.chars().count(),
            SceneNode::Rule { .. } => 0,
        })
        .sum()
}

fn metric_node_summary(node: &SceneNode) -> Option<(usize, usize, usize)> {
    match node {
        SceneNode::Metric {
            label, value, hint, ..
        } => Some((
            label.as_deref().unwrap_or_default().chars().count(),
            value.as_deref().unwrap_or_default().chars().count(),
            hint.as_deref().unwrap_or_default().chars().count(),
        )),
        _ => None,
    }
}

fn estimate_takeaway_rows(text: &str, area_cols: usize) -> f32 {
    let text_len = clean_scene_text(text).chars().count() as f32;
    if text_len <= 0.0 {
        return 0.0;
    }
    let chars_per_line = (area_cols as f32 * 1.05).max(18.0);
    let line_count = (text_len / chars_per_line).ceil().max(1.0);
    1.0 + (line_count * 1.55)
}

fn estimate_callout_rows(title: Option<&str>, body_len: usize, area_cols: usize) -> f32 {
    let title_len = title.unwrap_or_default().chars().count();
    let total_len = title_len + body_len;
    if total_len == 0 {
        return 0.0;
    }
    let chars_per_line = (area_cols as f32 * 1.25).max(16.0);
    let line_count = (total_len as f32 / chars_per_line).ceil().max(1.0);
    2.6 + (line_count * 1.15)
}

fn maybe_warn_takeaway_contract(
    area: AreaFrame,
    text: &str,
    seen: &mut HashSet<String>,
    warnings: &mut Vec<String>,
) {
    let required_rows = estimate_takeaway_rows(text, area.w);
    if required_rows > area.h as f32 + 0.25 {
        push_contract_warning(
            seen,
            warnings,
            format!(
                "Takeaway is too dense for a {} hero area. Give it more rows or shorten the sentence.",
                area.size_label()
            ),
        );
    }
}

fn maybe_warn_metric_contract(
    area: AreaFrame,
    value_len: usize,
    hint_len: usize,
    grid_cols: Option<usize>,
    seen: &mut HashSet<String>,
    warnings: &mut Vec<String>,
) {
    if grid_cols.is_some() || value_len == 0 {
        return;
    }

    let narrow_value = area.w <= 10 && value_len >= 10;
    let dense_copy = area.h <= 5 && hint_len > area.w.saturating_mul(2);
    if narrow_value || dense_copy {
        push_contract_warning(
            seen,
            warnings,
            format!(
                "Metric is too dense for a {} tile. Use a shorter value, more columns, or a compact scorecard recipe.",
                area.size_label()
            ),
        );
    }
}

fn maybe_warn_metric_grid_contract(
    area: AreaFrame,
    cols: usize,
    children: &[SceneNode],
    seen: &mut HashSet<String>,
    warnings: &mut Vec<String>,
) {
    if cols < 2 {
        return;
    }

    let metrics: Vec<_> = children.iter().filter_map(metric_node_summary).collect();
    if metrics.len() < 2 {
        return;
    }

    let card_cols = area.w as f32 / cols as f32;
    let longest_value = metrics.iter().map(|(_, value_len, _)| *value_len).max().unwrap_or(0);
    let longest_hint = metrics.iter().map(|(_, _, hint_len)| *hint_len).max().unwrap_or(0);
    let values_too_long = longest_value as f32 > card_cols * 0.95;
    let narrow_cards = card_cols <= 10.5 && longest_value >= 8;
    let dense_hints = area.h <= 10 && longest_hint > (card_cols as usize).saturating_mul(3);

    if values_too_long || narrow_cards || dense_hints {
        push_contract_warning(
            seen,
            warnings,
            format!(
                "{}-up metric grid is too tight inside a {} area. Use wider cards, fewer columns, or shorter metric values.",
                cols,
                area.size_label()
            ),
        );
    }
}

fn maybe_warn_callout_contract(
    area: AreaFrame,
    title: Option<&str>,
    children: &[SceneNode],
    seen: &mut HashSet<String>,
    warnings: &mut Vec<String>,
) {
    let required_rows = estimate_callout_rows(title, scene_nodes_text_length(children), area.w);
    if required_rows > area.h as f32 + 0.25 {
        push_contract_warning(
            seen,
            warnings,
            format!(
                "Callout copy is too dense for a {} rail. Widen the rail or reduce the copy.",
                area.size_label()
            ),
        );
    }
}

fn maybe_warn_chart_contract(
    area: AreaFrame,
    chart_type: &str,
    points: usize,
    seen: &mut HashSet<String>,
    warnings: &mut Vec<String>,
) {
    let min_cols = if chart_type.eq_ignore_ascii_case("trend") { 20 } else { 18 };
    let too_small = area.w < min_cols || area.h < 8;
    let too_many_points = points > 8 && area.w < 32;
    let too_few_points = points < 2;
    if too_small || too_many_points || too_few_points {
        push_contract_warning(
            seen,
            warnings,
            format!(
                "Chart is too constrained for a {} area. Give it at least {} x 8 and keep the series concise.",
                area.size_label(),
                min_cols
            ),
        );
    }
}

fn collect_scene_contract_warnings(
    nodes: &[SceneNode],
    canvas: Option<CanvasFrame>,
    area: Option<AreaFrame>,
    grid_cols: Option<usize>,
    seen: &mut HashSet<String>,
    warnings: &mut Vec<String>,
) {
    for node in nodes {
        match node {
            SceneNode::Canvas {
                cols,
                rows,
                children,
                ..
            } => collect_scene_contract_warnings(
                children,
                Some(CanvasFrame {
                    cols: *cols,
                    rows: *rows,
                }),
                None,
                None,
                seen,
                warnings,
            ),
            SceneNode::Area {
                x,
                y,
                w,
                h,
                children,
                ..
            } => {
                let area_frame = AreaFrame {
                    w: *w,
                    h: *h,
                };
                if let Some(canvas_frame) = canvas {
                    let right_edge = x.saturating_add(*w).saturating_sub(1);
                    let bottom_edge = y.saturating_add(*h).saturating_sub(1);
                    if right_edge > canvas_frame.cols || bottom_edge > canvas_frame.rows {
                        push_contract_warning(
                            seen,
                            warnings,
                            format!(
                                "Area at ({}, {}) with size {} exceeds the {} x {} canvas bounds.",
                                x,
                                y,
                                area_frame.size_label(),
                                canvas_frame.cols,
                                canvas_frame.rows
                            ),
                        );
                    }
                }
                collect_scene_contract_warnings(
                    children,
                    canvas,
                    Some(area_frame),
                    None,
                    seen,
                    warnings,
                );
            }
            SceneNode::LayoutGroup {
                component,
                cols,
                children,
                ..
            } => {
                let next_grid_cols = if component == "Grid" {
                    Some(
                        cols.unwrap_or_else(|| {
                            children
                                .iter()
                                .filter(|child| matches!(child, SceneNode::Metric { .. }))
                                .count()
                                .clamp(1, 4)
                        }),
                    )
                } else {
                    grid_cols
                };

                if component == "Grid" {
                    if let (Some(area_frame), Some(resolved_cols)) = (area, next_grid_cols) {
                        maybe_warn_metric_grid_contract(
                            area_frame,
                            resolved_cols,
                            children,
                            seen,
                            warnings,
                        );
                    }
                }

                collect_scene_contract_warnings(
                    children,
                    canvas,
                    area,
                    next_grid_cols,
                    seen,
                    warnings,
                );
            }
            SceneNode::Surface {
                component,
                title,
                children,
                ..
            } => {
                if component == "Callout" {
                    if let Some(area_frame) = area {
                        maybe_warn_callout_contract(
                            area_frame,
                            title.as_deref(),
                            children,
                            seen,
                            warnings,
                        );
                    }
                }

                collect_scene_contract_warnings(
                    children, canvas, area, grid_cols, seen, warnings,
                );
            }
            SceneNode::Metric { value, hint, .. } => {
                if let Some(area_frame) = area {
                    maybe_warn_metric_contract(
                        area_frame,
                        value.as_deref().unwrap_or_default().chars().count(),
                        hint.as_deref().unwrap_or_default().chars().count(),
                        grid_cols,
                        seen,
                        warnings,
                    );
                }
            }
            SceneNode::Chart {
                chart_type, data, ..
            } => {
                if let Some(area_frame) = area {
                    maybe_warn_chart_contract(
                        area_frame,
                        chart_type,
                        data.len(),
                        seen,
                        warnings,
                    );
                }
            }
            SceneNode::Text { role, text, .. } => {
                if role == "takeaway" {
                    if let Some(area_frame) = area {
                        maybe_warn_takeaway_contract(area_frame, text, seen, warnings);
                    }
                }
            }
            SceneNode::List { .. }
            | SceneNode::Media { .. }
            | SceneNode::CodeBlock { .. }
            | SceneNode::Pill { .. }
            | SceneNode::Rule { .. }
            | SceneNode::Arrow { .. }
            | SceneNode::Raw { .. } => {}
        }
    }
}

fn slide_contract_warnings(slide: &str) -> Vec<String> {
    let mut warnings = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    let nodes = compile_slide_nodes(slide);
    collect_scene_contract_warnings(&nodes, None, None, None, &mut seen, &mut warnings);
    warnings
}

fn slide_review_warnings(
    slide: &str,
    title: &str,
    words: usize,
    bullets: usize,
    paragraph_words: usize,
    component_counts: &HashMap<String, usize>,
) -> Vec<String> {
    let mut warnings = Vec::<String>::new();

    if title.starts_with("Slide ") {
        warnings.push("Slide has no explicit heading.".to_string());
    }
    if !uses_spatial_canvas(component_counts) {
        warnings.push(
            "Slide must use the 2.0 spatial contract: one Canvas with Area regions.".to_string(),
        );
    }
    if words > 110 {
        warnings.push(format!("High density: {words} words."));
    }
    if bullets > 6 {
        warnings.push(format!("Too many list items: {bullets}."));
    }
    if paragraph_words > 45 {
        warnings.push(format!("Longest paragraph is {paragraph_words} words."));
    }
    if split_class_re().is_match(slide) {
        warnings.push(
            "Legacy `split` layout is no longer supported. Replace it with Canvas and Area."
                .to_string(),
        );
    }
    warnings.extend(slide_contract_warnings(slide));

    warnings
}

fn build_project_analysis(project_path: &Path) -> Result<ProjectAnalysis, String> {
    let canonical_project = normalize_existing_project_directory(&path_to_string(project_path))?;
    let source = read_page_mdx(&canonical_project)?;
    let (_, body) = extract_frontmatter(&source);
    let slides = extract_slides(&body);

    let mut outline = Vec::<DeckOutlineEntry>::new();
    let mut slide_analyses = Vec::<SlideAnalysis>::new();
    let mut project_components = HashMap::<String, usize>::new();
    let mut archetype_counts = HashMap::<String, usize>::new();
    let mut warnings = Vec::<String>::new();
    let mut non_spatial_slide_total = 0usize;

    for (index, slide) in slides.iter().enumerate() {
        let title = slide_title_for(slide, index);
        let words = words_in_text(slide);
        let bullets = bullet_re().find_iter(slide).count();
        let paragraph_words = max_paragraph_words(slide);
        let component_counts = component_counts_for_slide(slide);
        if !uses_spatial_canvas(&component_counts) {
            non_spatial_slide_total += 1;
        }
        let archetype = inferred_archetype(slide, words, bullets, &component_counts);
        let slide_warnings = slide_review_warnings(
            slide,
            &title,
            words,
            bullets,
            paragraph_words,
            &component_counts,
        );

        outline.push(DeckOutlineEntry {
            index,
            title: title.clone(),
        });

        for (name, count) in &component_counts {
            increment_count(&mut project_components, name.as_str(), *count);
        }
        increment_count(&mut archetype_counts, archetype.as_str(), 1);

        slide_analyses.push(SlideAnalysis {
            index,
            title,
            archetype,
            words,
            bullets,
            max_paragraph_words: paragraph_words,
            components: sorted_named_counts(component_counts),
            warnings: slide_warnings,
        });
    }

    let has_project_css = canonical_project.join("slides.css").exists();
    if !has_project_css {
        warnings.push(
            "Project has no `slides.css`; 2.0 decks should carry project-level theme tokens."
                .to_string(),
        );
    }

    let structured_component_total = STRUCTURED_COMPONENT_NAMES
        .iter()
        .map(|name| project_components.get(*name).copied().unwrap_or(0))
        .sum::<usize>();
    if structured_component_total == 0 && !slides.is_empty() {
        warnings.push(
            "Deck does not use FastSlides 2.0 primitives; move slides to Canvas and Area."
                .to_string(),
        );
    }

    if non_spatial_slide_total > 0 {
        warnings.push(format!(
            "{non_spatial_slide_total} slide(s) are not on the 2.0 spatial canvas contract."
        ));
    }

    if slides.len() >= 4 {
        if let Some(dominant) = archetype_counts
            .iter()
            .max_by(|left, right| left.1.cmp(right.1).then_with(|| right.0.cmp(left.0)))
        {
            if dominant.0 != "spatial-canvas" && *dominant.1 * 100 / slides.len() >= 75 {
                warnings.push(format!(
                    "Most slides resolve to `{}`. Consider more archetype variety for stronger pacing.",
                    dominant.0
                ));
            }
        }
    }

    let slides_with_findings = slide_analyses
        .iter()
        .filter(|slide| !slide.warnings.is_empty())
        .count();
    if slides_with_findings > 0 {
        warnings.push(format!(
            "{slides_with_findings} slide(s) have density or structure findings."
        ));
    }

    Ok(ProjectAnalysis {
        path: path_to_string(&canonical_project),
        slide_count: slides.len(),
        has_project_css,
        outline,
        components: sorted_named_counts(project_components),
        archetypes: sorted_named_counts(archetype_counts),
        warnings,
        slides: slide_analyses,
    })
}

#[derive(Debug, Clone)]
struct ComponentBlock {
    name: String,
    start: usize,
    end: usize,
    attrs: HashMap<String, String>,
    inner: Option<String>,
    source: String,
}

#[derive(Debug, Clone)]
struct CodeFenceBlock {
    start: usize,
    end: usize,
    language: Option<String>,
    code: String,
}

fn scene_attr_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"([A-Za-z_][A-Za-z0-9_-]*)\s*=\s*(?:"([^"]*)"|'([^']*)'|\{([^{}]*)\})"#)
            .expect("invalid scene attr regex")
    })
}

fn markdown_heading_capture_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s{0,3}(#{1,3})\s+(.+?)\s*$"#)
            .expect("invalid markdown heading capture regex")
    })
}

fn html_heading_block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?is)<h([1-3])[^>]*>(.*?)</h[1-3]>"#)
            .expect("invalid html heading block regex")
    })
}

fn markdown_list_item_capture_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*((?:[-*+])|(?:\d+\.))\s+(.+?)\s*$"#)
            .expect("invalid markdown list capture regex")
    })
}

fn html_list_block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?is)<(ul|ol)[^>]*>(.*?)</(?:ul|ol)>"#)
            .expect("invalid html list block regex")
    })
}

fn html_list_item_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?is)<li[^>]*>(.*?)</li>"#).expect("invalid html list item regex")
    })
}

fn code_fence_block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)```([A-Za-z0-9_+-]+)?\n(.*?)\n?```"#).expect("invalid code fence regex")
    })
}

fn markdown_image_capture_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"!\[([^\]]*)\]\(([^)]+)\)"#).expect("invalid markdown image regex")
    })
}

fn html_image_block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?is)<img\b([^>]*?)>"#).expect("invalid html image regex"))
}

fn html_video_block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?is)<video\b([^>]*?)>"#).expect("invalid html video regex"))
}

fn find_tag_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut index = start + 1;
    let mut quote = None::<u8>;
    let mut brace_depth = 0usize;

    while index < bytes.len() {
        let current = bytes[index];
        if let Some(active_quote) = quote {
            if current == active_quote && bytes.get(index.saturating_sub(1)) != Some(&b'\\') {
                quote = None;
            }
        } else {
            match current {
                b'"' | b'\'' => quote = Some(current),
                b'{' => brace_depth += 1,
                b'}' => brace_depth = brace_depth.saturating_sub(1),
                b'>' if brace_depth == 0 => return Some(index),
                _ => {}
            }
        }
        index += 1;
    }

    None
}

fn is_self_closing_tag(source: &str, tag_start: usize, tag_end: usize) -> bool {
    let raw = source[tag_start + 1..tag_end].trim_end();
    raw.ends_with('/')
}

fn component_starts_at(source: &str, start: usize, name: &str) -> bool {
    let Some(rest) = source.get(start..) else {
        return false;
    };
    if !rest.starts_with('<') || rest.starts_with("</") {
        return false;
    }
    let expected = format!("<{name}");
    if !rest.starts_with(expected.as_str()) {
        return false;
    }
    rest.chars()
        .nth(expected.chars().count())
        .map(|ch| ch.is_whitespace() || ch == '>' || ch == '/')
        .unwrap_or(true)
}

fn parse_component_attrs(raw: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::<String, String>::new();
    for captures in scene_attr_re().captures_iter(raw) {
        let Some(name) = captures.get(1).map(|item| item.as_str().to_string()) else {
            continue;
        };
        let raw_value = captures
            .get(2)
            .or_else(|| captures.get(3))
            .or_else(|| captures.get(4))
            .map(|item| item.as_str())
            .unwrap_or_default();
        let value = normalize_frontmatter_value(raw_value);
        if !value.is_empty() {
            attrs.insert(name, value);
        }
    }
    attrs
}

fn attr_text(attrs: &HashMap<String, String>, name: &str) -> Option<String> {
    attrs
        .get(name)
        .cloned()
        .filter(|value| !value.trim().is_empty())
}

fn attr_usize(attrs: &HashMap<String, String>, name: &str) -> Option<usize> {
    attrs
        .get(name)
        .and_then(|value| value.trim().parse::<usize>().ok())
}

fn attr_bool(attrs: &HashMap<String, String>, name: &str) -> Option<bool> {
    attrs.get(name).and_then(|value| {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    })
}

fn attr_class_name(attrs: &HashMap<String, String>) -> Option<String> {
    attr_text(attrs, "className")
}

fn parse_chart_value(raw: &str) -> Option<f32> {
    let normalized = raw.trim().trim_end_matches('%').replace(',', "");
    if normalized.is_empty() {
        return None;
    }
    normalized.parse::<f32>().ok()
}

fn parse_chart_data(raw: &str) -> Vec<SceneChartDatum> {
    raw.split([';', '\n'])
        .filter_map(|entry| {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                return None;
            }
            let (label, value) = trimmed
                .split_once(':')
                .or_else(|| trimmed.split_once('='))
                .unwrap_or((trimmed, ""));
            let label = label.trim().to_string();
            let value = parse_chart_value(value)?;
            if label.is_empty() {
                return None;
            }
            Some(SceneChartDatum { label, value })
        })
        .collect()
}

fn build_component_block(source: &str, start: usize, name: &str) -> Option<ComponentBlock> {
    let tag_end = find_tag_end(source, start)?;
    let attrs_start = start + 1 + name.len();
    let attrs_source = if is_self_closing_tag(source, start, tag_end) {
        source[attrs_start..tag_end]
            .trim_end()
            .strip_suffix('/')
            .unwrap_or(source[attrs_start..tag_end].trim_end())
            .trim()
    } else {
        source[attrs_start..tag_end].trim()
    };
    let attrs = parse_component_attrs(attrs_source);

    if is_self_closing_tag(source, start, tag_end) {
        return Some(ComponentBlock {
            name: name.to_string(),
            start,
            end: tag_end + 1,
            attrs,
            inner: None,
            source: source[start..tag_end + 1].to_string(),
        });
    }

    let open_pattern = format!("<{name}");
    let close_pattern = format!("</{name}");
    let mut depth = 1usize;
    let mut cursor = tag_end + 1;

    while cursor < source.len() {
        let next_open = source[cursor..]
            .find(open_pattern.as_str())
            .map(|offset| cursor + offset)
            .filter(|position| component_starts_at(source, *position, name));
        let next_close = source[cursor..]
            .find(close_pattern.as_str())
            .map(|offset| cursor + offset);

        match (next_open, next_close) {
            (Some(open_start), Some(close_start)) if close_start < open_start => {
                let close_end = source[close_start..]
                    .find('>')
                    .map(|offset| close_start + offset)?;
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let inner = source[tag_end + 1..close_start].to_string();
                    return Some(ComponentBlock {
                        name: name.to_string(),
                        start,
                        end: close_end + 1,
                        attrs,
                        inner: Some(inner),
                        source: source[start..close_end + 1].to_string(),
                    });
                }
                cursor = close_end + 1;
            }
            (None, Some(close_start)) => {
                let close_end = source[close_start..]
                    .find('>')
                    .map(|offset| close_start + offset)?;
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let inner = source[tag_end + 1..close_start].to_string();
                    return Some(ComponentBlock {
                        name: name.to_string(),
                        start,
                        end: close_end + 1,
                        attrs,
                        inner: Some(inner),
                        source: source[start..close_end + 1].to_string(),
                    });
                }
                cursor = close_end + 1;
            }
            (Some(open_start), _) => {
                let open_end = find_tag_end(source, open_start)?;
                if !is_self_closing_tag(source, open_start, open_end) {
                    depth += 1;
                }
                cursor = open_end + 1;
            }
            (None, None) => break,
        }
    }

    None
}

fn next_component_block(source: &str, start: usize, names: &[&str]) -> Option<ComponentBlock> {
    let mut cursor = start;
    while let Some(offset) = source[cursor..].find('<') {
        let candidate = cursor + offset;
        for name in names {
            if component_starts_at(source, candidate, name) {
                if let Some(block) = build_component_block(source, candidate, name) {
                    return Some(block);
                }
            }
        }
        cursor = candidate + 1;
    }
    None
}

fn extract_component_blocks(source: &str, names: &[&str]) -> Vec<ComponentBlock> {
    let mut blocks = Vec::<ComponentBlock>::new();
    let mut cursor = 0usize;
    while let Some(block) = next_component_block(source, cursor, names) {
        cursor = block.end;
        blocks.push(block);
    }
    blocks
}

fn extract_code_fence_blocks(source: &str) -> Vec<CodeFenceBlock> {
    code_fence_block_re()
        .captures_iter(source)
        .filter_map(|captures| {
            let full = captures.get(0)?;
            let language = captures
                .get(1)
                .map(|item| item.as_str().trim().to_string())
                .filter(|value| !value.is_empty());
            let code = captures
                .get(2)
                .map(|item| item.as_str().trim_end().to_string())
                .unwrap_or_default();
            Some(CodeFenceBlock {
                start: full.start(),
                end: full.end(),
                language,
                code,
            })
        })
        .collect()
}

fn clean_scene_text(raw: &str) -> String {
    let with_breaks = raw
        .replace("<br />", "\n")
        .replace("<br/>", "\n")
        .replace("<br>", "\n")
        .replace("</p>", "\n")
        .replace("</div>", "\n")
        .replace("</section>", "\n");
    let without_tags = html_tag_re().replace_all(&with_breaks, " ");
    without_tags
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn compile_component_node(block: &ComponentBlock) -> SceneNode {
    match block.name.as_str() {
        "Stack" | "Row" | "Grid" | "PillRow" => SceneNode::LayoutGroup {
            component: block.name.clone(),
            cols: attr_usize(&block.attrs, "cols"),
            gap: attr_text(&block.attrs, "gap"),
            align: attr_text(&block.attrs, "align"),
            justify: attr_text(&block.attrs, "justify"),
            nowrap: attr_bool(&block.attrs, "nowrap"),
            class_name: attr_class_name(&block.attrs),
            children: block
                .inner
                .as_deref()
                .map(compile_fragment_nodes)
                .unwrap_or_default(),
            source_mdx: block.source.clone(),
        },
        "Card" | "Panel" | "Callout" | "Quote" => SceneNode::Surface {
            component: block.name.clone(),
            tone: attr_text(&block.attrs, "tone"),
            title: attr_text(&block.attrs, "title"),
            kicker: attr_text(&block.attrs, "kicker"),
            subtitle: attr_text(&block.attrs, "subtitle"),
            foot: attr_text(&block.attrs, "foot"),
            attribution: attr_text(&block.attrs, "attribution"),
            class_name: attr_class_name(&block.attrs),
            children: block
                .inner
                .as_deref()
                .map(compile_fragment_nodes)
                .unwrap_or_default(),
            source_mdx: block.source.clone(),
        },
        "Metric" => {
            let fallback_value = block
                .inner
                .as_deref()
                .map(clean_scene_text)
                .filter(|value| !value.is_empty());
            SceneNode::Metric {
                label: attr_text(&block.attrs, "label"),
                value: attr_text(&block.attrs, "value").or(fallback_value),
                hint: attr_text(&block.attrs, "hint"),
                class_name: attr_class_name(&block.attrs),
                source_mdx: block.source.clone(),
            }
        }
        "Chart" => {
            let inline_data = block
                .inner
                .as_deref()
                .map(clean_scene_text)
                .filter(|value| !value.is_empty());
            SceneNode::Chart {
                chart_type: attr_text(&block.attrs, "type").unwrap_or_else(|| "bar".to_string()),
                title: attr_text(&block.attrs, "title"),
                tone: attr_text(&block.attrs, "tone"),
                value_suffix: attr_text(&block.attrs, "suffix")
                    .or_else(|| attr_text(&block.attrs, "valueSuffix")),
                highlight: attr_text(&block.attrs, "highlight"),
                data: attr_text(&block.attrs, "data")
                    .or_else(|| attr_text(&block.attrs, "items"))
                    .or(inline_data)
                    .map(|value| parse_chart_data(&value))
                    .unwrap_or_default(),
                class_name: attr_class_name(&block.attrs),
                source_mdx: block.source.clone(),
            }
        }
        "Caption" => SceneNode::Text {
            role: "caption".to_string(),
            text: block
                .inner
                .as_deref()
                .map(clean_scene_text)
                .unwrap_or_default(),
            level: None,
            class_name: attr_class_name(&block.attrs),
        },
        "Kicker" => SceneNode::Text {
            role: "kicker".to_string(),
            text: block
                .inner
                .as_deref()
                .map(clean_scene_text)
                .unwrap_or_default(),
            level: None,
            class_name: attr_class_name(&block.attrs),
        },
        "Takeaway" => {
            let level = attr_text(&block.attrs, "as").and_then(|value| match value.trim() {
                "h1" => Some(1),
                "h2" => Some(2),
                "h3" => Some(3),
                _ => None,
            });
            SceneNode::Text {
                role: "takeaway".to_string(),
                text: block
                    .inner
                    .as_deref()
                    .map(clean_scene_text)
                    .unwrap_or_default(),
                level,
                class_name: attr_class_name(&block.attrs),
            }
        }
        "Pill" => SceneNode::Pill {
            tone: attr_text(&block.attrs, "tone"),
            text: block
                .inner
                .as_deref()
                .map(clean_scene_text)
                .unwrap_or_default(),
            class_name: attr_class_name(&block.attrs),
        },
        "Rule" => SceneNode::Rule {
            class_name: attr_class_name(&block.attrs),
        },
        "Arrow" => SceneNode::Arrow {
            direction: attr_text(&block.attrs, "direction"),
            tone: attr_text(&block.attrs, "tone"),
            label: attr_text(&block.attrs, "label").or_else(|| {
                block.inner
                    .as_deref()
                    .map(clean_scene_text)
                    .filter(|value| !value.is_empty())
            }),
            class_name: attr_class_name(&block.attrs),
            source_mdx: block.source.clone(),
        },
        _ => SceneNode::Raw {
            format: "mdx".to_string(),
            text: block.source.clone(),
        },
    }
}

fn extract_media_nodes(fragment: &str) -> Vec<SceneNode> {
    let mut nodes = Vec::<SceneNode>::new();

    for captures in markdown_image_capture_re().captures_iter(fragment) {
        let src = captures
            .get(2)
            .map(|item| sanitize_markdown_target(item.as_str()))
            .unwrap_or_default();
        if src.is_empty() {
            continue;
        }
        let alt = captures
            .get(1)
            .map(|item| item.as_str().trim().to_string())
            .filter(|value| !value.is_empty());
        nodes.push(SceneNode::Media {
            media_kind: "image".to_string(),
            src,
            alt,
        });
    }

    for captures in html_image_block_re().captures_iter(fragment) {
        let attrs = captures
            .get(1)
            .map(|item| parse_component_attrs(item.as_str()))
            .unwrap_or_default();
        let Some(src) = attr_text(&attrs, "src") else {
            continue;
        };
        nodes.push(SceneNode::Media {
            media_kind: "image".to_string(),
            src,
            alt: attr_text(&attrs, "alt"),
        });
    }

    for captures in html_video_block_re().captures_iter(fragment) {
        let attrs = captures
            .get(1)
            .map(|item| parse_component_attrs(item.as_str()))
            .unwrap_or_default();
        let Some(src) = attr_text(&attrs, "src").or_else(|| attr_text(&attrs, "poster")) else {
            continue;
        };
        nodes.push(SceneNode::Media {
            media_kind: "video".to_string(),
            src,
            alt: None,
        });
    }

    nodes
}

fn extract_heading_nodes(fragment: &str) -> (Vec<SceneNode>, String) {
    let mut nodes = Vec::<SceneNode>::new();

    for captures in html_heading_block_re().captures_iter(fragment) {
        let level = captures
            .get(1)
            .and_then(|item| item.as_str().parse::<u8>().ok());
        let text = captures
            .get(2)
            .map(|item| clean_heading_text(item.as_str()))
            .unwrap_or_default();
        if !text.is_empty() {
            nodes.push(SceneNode::Text {
                role: "heading".to_string(),
                text,
                level,
                class_name: None,
            });
        }
    }

    let without_html = html_heading_block_re()
        .replace_all(fragment, "\n")
        .to_string();

    for captures in markdown_heading_capture_re().captures_iter(&without_html) {
        let level = captures.get(1).map(|item| item.as_str().len() as u8);
        let text = captures
            .get(2)
            .map(|item| clean_heading_text(item.as_str()))
            .unwrap_or_default();
        if !text.is_empty() {
            nodes.push(SceneNode::Text {
                role: "heading".to_string(),
                text,
                level,
                class_name: None,
            });
        }
    }

    let remainder = markdown_heading_capture_re()
        .replace_all(&without_html, "\n")
        .to_string();

    (nodes, remainder)
}

fn extract_list_nodes(fragment: &str) -> (Vec<SceneNode>, String) {
    let mut nodes = Vec::<SceneNode>::new();

    for captures in html_list_block_re().captures_iter(fragment) {
        let ordered = captures
            .get(1)
            .map(|item| item.as_str().eq_ignore_ascii_case("ol"))
            .unwrap_or(false);
        let body = captures
            .get(2)
            .map(|item| item.as_str())
            .unwrap_or_default();
        let items: Vec<_> = html_list_item_re()
            .captures_iter(body)
            .filter_map(|item| item.get(1).map(|entry| clean_scene_text(entry.as_str())))
            .filter(|item| !item.is_empty())
            .collect();
        if !items.is_empty() {
            nodes.push(SceneNode::List { ordered, items });
        }
    }

    let without_html_lists = html_list_block_re().replace_all(fragment, "\n").to_string();
    let mut markdown_items = Vec::<String>::new();
    let mut ordered = true;
    for captures in markdown_list_item_capture_re().captures_iter(&without_html_lists) {
        let marker = captures
            .get(1)
            .map(|item| item.as_str())
            .unwrap_or_default();
        if !marker.ends_with('.') {
            ordered = false;
        }
        let item_text = captures
            .get(2)
            .map(|item| clean_scene_text(item.as_str()))
            .unwrap_or_default();
        if !item_text.is_empty() {
            markdown_items.push(item_text);
        }
    }
    if !markdown_items.is_empty() {
        nodes.push(SceneNode::List {
            ordered,
            items: markdown_items,
        });
    }

    let remainder = markdown_list_item_capture_re()
        .replace_all(&without_html_lists, "\n")
        .to_string();

    (nodes, remainder)
}

fn compile_plain_fragment_nodes(fragment: &str) -> Vec<SceneNode> {
    let trimmed = fragment.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut nodes = extract_media_nodes(trimmed);

    let without_media = markdown_image_capture_re()
        .replace_all(trimmed, " ")
        .to_string();
    let without_media = html_image_block_re()
        .replace_all(&without_media, " ")
        .to_string();
    let without_media = html_video_block_re()
        .replace_all(&without_media, " ")
        .to_string();

    let (mut heading_nodes, after_headings) = extract_heading_nodes(&without_media);
    nodes.append(&mut heading_nodes);

    let (mut list_nodes, after_lists) = extract_list_nodes(&after_headings);
    nodes.append(&mut list_nodes);

    let text = clean_scene_text(&after_lists);
    if !text.is_empty() {
        nodes.push(SceneNode::Text {
            role: "paragraph".to_string(),
            text,
            level: None,
            class_name: None,
        });
    }

    if nodes.is_empty() {
        return vec![SceneNode::Raw {
            format: "mdx".to_string(),
            text: trimmed.to_string(),
        }];
    }

    nodes
}

fn compile_loose_nodes(fragment: &str) -> Vec<SceneNode> {
    let trimmed = fragment.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if should_preserve_raw_html_fragment(trimmed) {
        return vec![SceneNode::Raw {
            format: "html".to_string(),
            text: trimmed.to_string(),
        }];
    }

    let code_blocks = extract_code_fence_blocks(trimmed);
    if code_blocks.is_empty() {
        return compile_plain_fragment_nodes(trimmed);
    }

    let mut nodes = Vec::<SceneNode>::new();
    let mut cursor = 0usize;
    for block in code_blocks {
        nodes.extend(compile_plain_fragment_nodes(&trimmed[cursor..block.start]));
        nodes.push(SceneNode::CodeBlock {
            language: block.language,
            code: block.code,
        });
        cursor = block.end;
    }
    nodes.extend(compile_plain_fragment_nodes(&trimmed[cursor..]));
    nodes
}

fn compile_fragment_nodes(fragment: &str) -> Vec<SceneNode> {
    let trimmed = fragment.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let block_names = [
        "Stack", "Row", "Grid", "Card", "Panel", "Metric", "Chart", "Caption", "Kicker",
        "Takeaway", "Callout", "PillRow", "Pill", "Quote", "Rule",
    ];
    let blocks = extract_component_blocks(trimmed, &block_names);
    if blocks.is_empty() {
        return compile_loose_nodes(trimmed);
    }

    let mut nodes = Vec::<SceneNode>::new();
    let mut cursor = 0usize;
    for block in blocks {
        nodes.extend(compile_loose_nodes(&trimmed[cursor..block.start]));
        nodes.push(compile_component_node(&block));
        cursor = block.end;
    }
    nodes.extend(compile_loose_nodes(&trimmed[cursor..]));
    nodes
}

fn compile_canvas_node(block: &ComponentBlock) -> SceneNode {
    let cols = attr_usize(&block.attrs, "cols").unwrap_or(50);
    let rows = attr_usize(&block.attrs, "rows").unwrap_or(25);
    let gap = attr_text(&block.attrs, "gap");
    let class_name = attr_class_name(&block.attrs);
    let inner = block.inner.as_deref().unwrap_or_default();
    let area_blocks = extract_component_blocks(inner, &["Area"]);
    let children = if area_blocks.is_empty() {
        compile_fragment_nodes(inner)
    } else {
        let mut nodes = Vec::<SceneNode>::new();
        let mut cursor = 0usize;
        for area_block in &area_blocks {
            nodes.extend(compile_fragment_nodes(&inner[cursor..area_block.start]));
            nodes.push(compile_area_node(area_block));
            cursor = area_block.end;
        }
        nodes.extend(compile_fragment_nodes(&inner[cursor..]));
        nodes
    };

    SceneNode::Canvas {
        cols,
        rows,
        gap,
        class_name,
        children,
        source_mdx: block.source.clone(),
    }
}

fn compile_area_node(block: &ComponentBlock) -> SceneNode {
    let inner = block.inner.as_deref().unwrap_or_default();
    SceneNode::Area {
        x: attr_usize(&block.attrs, "x").unwrap_or(1),
        y: attr_usize(&block.attrs, "y").unwrap_or(1),
        w: attr_usize(&block.attrs, "w").unwrap_or(1),
        h: attr_usize(&block.attrs, "h").unwrap_or(1),
        layer: attr_usize(&block.attrs, "layer"),
        gap: attr_text(&block.attrs, "gap"),
        align: attr_text(&block.attrs, "align"),
        justify: attr_text(&block.attrs, "justify"),
        class_name: attr_class_name(&block.attrs),
        children: compile_fragment_nodes(inner),
        source_mdx: block.source.clone(),
    }
}

fn infer_scene_layout(slide: &str) -> SceneLayout {
    if let Some(canvas_block) = next_component_block(slide, 0, &["Canvas"]) {
        return SceneLayout {
            kind: "canvas".to_string(),
            cols: attr_usize(&canvas_block.attrs, "cols").or(Some(50)),
            rows: attr_usize(&canvas_block.attrs, "rows").or(Some(25)),
            gap: attr_text(&canvas_block.attrs, "gap"),
        };
    }

    let component_counts = component_counts_for_slide(slide);
    let kind = if component_counts.get("Grid").copied().unwrap_or(0) > 0 {
        "grid"
    } else if component_counts.get("Row").copied().unwrap_or(0) > 0 {
        "row"
    } else if component_counts.get("Stack").copied().unwrap_or(0) > 0 {
        "stack"
    } else if LAYOUT_COMPONENT_NAMES
        .iter()
        .any(|name| component_counts.get(*name).copied().unwrap_or(0) > 0)
    {
        "structured-layout"
    } else {
        "flow"
    };

    SceneLayout {
        kind: kind.to_string(),
        cols: None,
        rows: None,
        gap: None,
    }
}

fn compile_slide_nodes(slide: &str) -> Vec<SceneNode> {
    if let Some(canvas_block) = next_component_block(slide, 0, &["Canvas"]) {
        let mut nodes = Vec::<SceneNode>::new();
        nodes.extend(compile_fragment_nodes(&slide[..canvas_block.start]));
        nodes.push(compile_canvas_node(&canvas_block));
        nodes.extend(compile_fragment_nodes(&slide[canvas_block.end..]));
        return nodes;
    }
    compile_fragment_nodes(slide)
}

fn deck_class_name_from_body(body: &str) -> Option<String> {
    let main_open = Regex::new(r#"(?is)<main(?P<attrs>[^>]*)>"#).unwrap();
    let captures = main_open.captures(body)?;
    let attrs = captures.name("attrs")?.as_str();
    let parsed = parse_component_attrs(attrs);
    let class_name = attr_text(&parsed, "className").or_else(|| attr_text(&parsed, "class"))?;
    let normalized = class_name
        .split_whitespace()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn should_preserve_raw_html_fragment(fragment: &str) -> bool {
    if fragment.contains("```") {
        return false;
    }

    if fragment.contains("className=") || fragment.contains("class=") {
        return true;
    }

    let html_block_tags = [
        "<table",
        "<thead",
        "<tbody",
        "<tr",
        "<td",
        "<th",
        "<figure",
        "<figcaption",
    ];
    fragment.trim_start().starts_with('<')
        && html_block_tags.iter().any(|tag| fragment.contains(tag))
}

fn validate_scene_slide_contract(slide: &str, index: usize) -> Result<(), String> {
    let component_counts = component_counts_for_slide(slide);
    if !uses_spatial_canvas(&component_counts) {
        return Err(format!(
            "Slide {} is not on the 2.0 spatial contract. Use one Canvas with Area regions before compiling a scene.",
            index + 1
        ));
    }
    if split_class_re().is_match(slide) {
        return Err(format!(
            "Slide {} still uses legacy `split` layout. Replace it with Canvas and Area before compiling a scene.",
            index + 1
        ));
    }
    Ok(())
}

fn load_project_scene_source(project_path: &Path) -> Result<ProjectSceneSource, String> {
    let canonical_project = normalize_existing_project_directory(&path_to_string(project_path))?;
    let source = read_page_mdx(&canonical_project)?;
    let (frontmatter, body) = extract_frontmatter(&source);
    let slides = extract_slides(&body);
    let metadata = frontmatter.unwrap_or_default();

    for (index, slide) in slides.iter().enumerate() {
        validate_scene_slide_contract(slide, index)?;
    }

    Ok(ProjectSceneSource {
        path: path_to_string(&canonical_project),
        project: metadata.get("project").cloned(),
        title: metadata.get("title").cloned(),
        subtitle: metadata.get("subtitle").cloned(),
        date: metadata.get("date").cloned(),
        deck_class_name: deck_class_name_from_body(&body),
        slides,
    })
}

fn build_scene_slide(slide: &str, index: usize) -> SceneSlide {
    SceneSlide {
        index,
        title: slide_title_for(slide, index),
        layout: infer_scene_layout(slide),
        nodes: compile_slide_nodes(slide),
        source_mdx: slide.trim().to_string(),
    }
}

fn build_scene_slide_manifest(slide: &str, index: usize) -> SceneSlideManifest {
    SceneSlideManifest {
        index,
        title: slide_title_for(slide, index),
        layout: infer_scene_layout(slide),
    }
}

fn build_project_scene(project_path: &Path) -> Result<ProjectScene, String> {
    let ProjectSceneSource {
        path,
        project,
        title,
        subtitle,
        date,
        deck_class_name,
        slides,
    } = load_project_scene_source(project_path)?;

    let slide_count = slides.len();
    let compiled_slides = slides
        .iter()
        .enumerate()
        .map(|(index, slide)| build_scene_slide(slide, index))
        .collect();

    Ok(ProjectScene {
        path,
        project,
        title,
        subtitle,
        date,
        deck_class_name,
        slide_count,
        slides: compiled_slides,
    })
}

fn build_project_scene_manifest(project_path: &Path) -> Result<ProjectSceneManifest, String> {
    let ProjectSceneSource {
        path,
        project,
        title,
        subtitle,
        date,
        deck_class_name,
        slides,
    } = load_project_scene_source(project_path)?;

    let slide_count = slides.len();
    let compiled_slides = slides
        .iter()
        .enumerate()
        .map(|(index, slide)| build_scene_slide_manifest(slide, index))
        .collect();

    Ok(ProjectSceneManifest {
        path,
        project,
        title,
        subtitle,
        date,
        deck_class_name,
        slide_count,
        slides: compiled_slides,
    })
}

fn build_project_scene_slide(project_path: &Path, index: usize) -> Result<SceneSlide, String> {
    let ProjectSceneSource { slides, .. } = load_project_scene_source(project_path)?;
    let slide = slides.get(index).ok_or_else(|| {
        format!(
            "Slide {} is out of range for this deck ({} slides).",
            index + 1,
            slides.len()
        )
    })?;
    Ok(build_scene_slide(slide, index))
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
    let block = captures
        .get(1)
        .map(|item| item.as_str())
        .unwrap_or_default();

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
      <Metric label="Grid" value="50 x 25" hint="Spacious authoring canvas" />
    </Area>

    <Area x={{22}} y={{15}} w={{10}} h={{5}}>
      <Metric label="Primitives" value="12+" hint="Reusable layout and content parts" />
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

        if !uses_spatial_canvas(&component_counts) {
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
    fs::write(project_path.join("slides.css"), DEFAULT_SLIDES_CSS)
        .map_err(|error| format!("Failed to create slides.css: {error}"))?;

    let mut config = normalized_config(load_config()?);
    remember_recent_project(&mut config, &project_path);
    save_config(&config)?;
    project_detail_for_path(&config, &project_path)
}

#[tauri::command]
fn toggle_project_pin(path: String) -> Result<AppState, String> {
    let project_path = normalize_existing_project_directory(&path)?;
    let mut config = normalized_config(load_config()?);
    let canonical_str = path_to_string(&project_path);

    if config.pinned_projects.contains(&canonical_str) {
        config.pinned_projects.retain(|p| p != &canonical_str);
    } else {
        config.pinned_projects.push(canonical_str);
    }

    save_config(&config)?;
    Ok(AppState {
        projects: list_projects(&config),
        config,
    })
}

#[tauri::command]
fn validate_project(path: String) -> Result<ValidationReport, String> {
    validate_project_folder(Path::new(&path))
}

#[tauri::command]
fn get_design_system() -> Result<DesignSystemRegistry, String> {
    Ok(design_system_registry())
}

#[tauri::command]
fn get_component_catalog() -> Result<ComponentCatalog, String> {
    build_component_catalog()
}

#[tauri::command]
fn get_component_template(name: String) -> Result<DesignTemplate, String> {
    component_template(&name)
}

#[tauri::command]
fn save_component_template(payload: SaveComponentPayload) -> Result<SaveComponentResponse, String> {
    let path = component_catalog_path()?;
    save_component_to_path(&path, payload)
}

#[tauri::command]
fn get_composition_template(name: String) -> Result<DesignTemplate, String> {
    composition_template(&name)
}

#[tauri::command]
fn get_recipe_template(name: String) -> Result<DesignTemplate, String> {
    recipe_template(&name)
}

#[tauri::command]
fn analyze_project(path: String) -> Result<ProjectAnalysis, String> {
    build_project_analysis(Path::new(&path))
}

#[tauri::command]
fn compile_project_scene(path: String) -> Result<ProjectScene, String> {
    build_project_scene(Path::new(&path))
}

#[tauri::command]
fn compile_project_scene_manifest(path: String) -> Result<ProjectSceneManifest, String> {
    build_project_scene_manifest(Path::new(&path))
}

#[tauri::command]
fn compile_project_scene_slide(path: String, index: usize) -> Result<SceneSlide, String> {
    build_project_scene_slide(Path::new(&path), index)
}

#[tauri::command]
fn capture_slide_image(
    path: String,
    slide: Option<usize>,
    output_dir: Option<String>,
    headed: Option<bool>,
) -> Result<SlideCaptureResponse, String> {
    capture_slide_image_for_project(Path::new(&path), slide, output_dir.as_deref(), headed.unwrap_or(false))
}

#[tauri::command]
fn start_project_scene_session(
    app: tauri::AppHandle,
    path: String,
    priority_index: Option<usize>,
    session_id: Option<String>,
) -> Result<ProjectSceneSessionHandle, String> {
    let source = load_project_scene_source(Path::new(&path))?;
    let session_id = session_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(next_scene_session_id);
    let handle = ProjectSceneSessionHandle {
        session_id: session_id.clone(),
        path: source.path.clone(),
        slide_count: source.slides.len(),
    };
    let worker_count = preferred_scene_session_worker_count();
    let priority_index = priority_index.unwrap_or(0);

    thread::spawn(move || {
        let mut sequence = 0u64;
        let mut emit_event = |payload: ProjectSceneSessionEventPayload| {
            let next_event = ProjectSceneSessionEvent {
                session_id: session_id.clone(),
                sequence,
                payload,
            };
            sequence += 1;
            if let Err(error) = app.emit(SCENE_SESSION_EVENT_NAME, &next_event) {
                log::warn!("Failed to emit scene session event: {}", error);
            }
        };

        if let Err(error) =
            emit_project_scene_session_events(source, priority_index, worker_count, &mut emit_event)
        {
            emit_event(ProjectSceneSessionEventPayload::SlideError { index: 0, error });
            emit_event(ProjectSceneSessionEventPayload::Complete {
                ready_count: 0,
                error_count: 1,
            });
        }
    });

    Ok(handle)
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
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Failed to create destination folder {}: {error}",
            parent.display()
        )
    })?;

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

fn optional_codex_home_directory() -> Option<PathBuf> {
    env::var("CODEX_HOME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var("HOME")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .map(|home| PathBuf::from(home).join(".codex"))
        })
}

fn resolve_codex_home_directory() -> Result<PathBuf, String> {
    optional_codex_home_directory()
        .ok_or_else(|| "Could not resolve Codex home directory from CODEX_HOME or HOME.".to_string())
}

fn resolve_codex_config_path() -> Result<PathBuf, String> {
    Ok(resolve_codex_home_directory()?.join("config.toml"))
}

fn fastslides_codex_mcp_block(server_name: &str, url: &str) -> String {
    format!("[mcp_servers.{server_name}]\nurl = \"{url}\"")
}

fn stitch_toml_sections(prefix: &str, block: &str, suffix: &str) -> String {
    let prefix = prefix.trim_end();
    let suffix = suffix.trim_start();

    match (prefix.is_empty(), suffix.is_empty()) {
        (true, true) => format!("{block}\n"),
        (true, false) => format!("{block}\n\n{suffix}\n"),
        (false, true) => format!("{prefix}\n\n{block}\n"),
        (false, false) => format!("{prefix}\n\n{block}\n\n{suffix}\n"),
    }
}

fn upsert_codex_mcp_server_block(
    source: &str,
    server_name: &str,
    url: &str,
) -> Result<(String, CodexInstallStatus), String> {
    let block = fastslides_codex_mcp_block(server_name, url);
    let header = format!("[mcp_servers.{server_name}]");
    let header_re = Regex::new(&format!(r"(?m)^{}\s*$", regex::escape(&header)))
        .map_err(|error| format!("Failed to build Codex config matcher: {error}"))?;
    let table_re = Regex::new(r"(?m)^\[[^\n]+\]\s*$")
        .map_err(|error| format!("Failed to build TOML section matcher: {error}"))?;

    if let Some(header_match) = header_re.find(source) {
        let after_header = &source[header_match.end()..];
        let next_table_start = table_re
            .find(after_header)
            .map(|matched| header_match.end() + matched.start())
            .unwrap_or(source.len());
        let current_block = source[header_match.start()..next_table_start].trim();
        if current_block == block {
            return Ok((
                stitch_toml_sections("", source.trim(), ""),
                CodexInstallStatus::Unchanged,
            ));
        }

        let updated = stitch_toml_sections(
            &source[..header_match.start()],
            &block,
            &source[next_table_start..],
        );
        return Ok((updated, CodexInstallStatus::Updated));
    }

    Ok((
        stitch_toml_sections(source, &block, ""),
        CodexInstallStatus::Installed,
    ))
}

fn install_codex_mcp_server_config() -> Result<CodexMcpInstallResponse, String> {
    let config_path = resolve_codex_config_path()?;
    let parent = config_path.parent().ok_or_else(|| {
        format!(
            "Codex config path has no parent directory: {}",
            config_path.display()
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Failed to create Codex config directory {}: {error}",
            parent.display()
        )
    })?;

    let existing = if config_path.exists() {
        fs::read_to_string(&config_path)
            .map_err(|error| format!("Failed to read {}: {error}", config_path.display()))?
    } else {
        String::new()
    };

    let url = mcp_server_url(mcp_server_addr().as_str(), mcp_server_path().as_str());
    let server_name = "fastslides";
    let (updated, status) = upsert_codex_mcp_server_block(&existing, server_name, &url)?;

    if status != CodexInstallStatus::Unchanged || !config_path.exists() {
        fs::write(&config_path, updated)
            .map_err(|error| format!("Failed to write {}: {error}", config_path.display()))?;
    }

    Ok(CodexMcpInstallResponse {
        ok: true,
        status,
        config_path: path_to_string(&config_path),
        server_name: server_name.to_string(),
        url,
    })
}

#[tauri::command]
fn install_codex_mcp_server() -> Result<CodexMcpInstallResponse, String> {
    install_codex_mcp_server_config()
}

#[tauri::command]
fn read_project_css(path: String) -> Result<String, String> {
    let project_dir = normalize_existing_project_directory(&path)?;
    let css_path = project_dir.join("slides.css");
    if !css_path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(&css_path)
        .map_err(|error| format!("Failed to read {}: {error}", css_path.display()))
}

#[tauri::command]
fn save_project_css(path: String, css: String) -> Result<(), String> {
    let project_dir = normalize_existing_project_directory(&path)?;
    let css_path = project_dir.join("slides.css");
    fs::write(&css_path, &css)
        .map_err(|error| format!("Failed to write {}: {error}", css_path.display()))
}

fn build_app_menu<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(app)?;

    #[cfg(target_os = "macos")]
    {
        let items = menu.items()?;
        if let Some(app_submenu) = items.first().and_then(|item| item.as_submenu()) {
            let settings_item = MenuItem::with_id(
                app,
                MENU_OPEN_SETTINGS_ID,
                "Settings…",
                true,
                Some("CmdOrCtrl+,"),
            )?;
            let install_codex_item = MenuItem::with_id(
                app,
                MENU_INSTALL_CODEX_MCP_ID,
                "Install MCP in Codex",
                true,
                None::<&str>,
            )?;
            let export_item = MenuItem::with_id(
                app,
                MENU_EXPORT_SKILL_ID,
                "Download FastSlides Skill…",
                true,
                None::<&str>,
            )?;
            let separator = PredefinedMenuItem::separator(app)?;
            app_submenu.insert(&settings_item, 2)?;
            app_submenu.insert(&install_codex_item, 3)?;
            app_submenu.insert(&separator, 4)?;
            app_submenu.insert(&export_item, 5)?;
        }
    }

    Ok(menu)
}

fn add_agent_hook_cors_headers(response: &mut Response<Cursor<Vec<u8>>>) {
    if let Ok(access_control) = Header::from_bytes("Access-Control-Allow-Origin", "*") {
        response.add_header(access_control);
    }
    if let Ok(allow_methods) =
        Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
    {
        response.add_header(allow_methods);
    }
    if let Ok(allow_headers) = Header::from_bytes("Access-Control-Allow-Headers", "Content-Type") {
        response.add_header(allow_headers);
    }
}

fn json_response(status_code: u16, payload: impl Serialize) -> Response<Cursor<Vec<u8>>> {
    let body = serde_json::to_string(&payload)
        .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"JSON serialization failed.\"}".to_string());
    let mut response = Response::from_string(body).with_status_code(StatusCode(status_code));

    if let Ok(content_type) = Header::from_bytes("Content-Type", "application/json") {
        response.add_header(content_type);
    }
    add_agent_hook_cors_headers(&mut response);
    response
}

fn empty_response(status_code: u16) -> Response<Cursor<Vec<u8>>> {
    let mut response = Response::from_string("").with_status_code(StatusCode(status_code));
    add_agent_hook_cors_headers(&mut response);
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

fn env_string(keys: &[&str], default: &str) -> String {
    keys.iter()
        .find_map(|key| env::var(key).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn preview_base_url() -> String {
    env_string(
        &["FASTSLIDES_PREVIEW_URL", "NEXT_PUBLIC_FASTSLIDES_PREVIEW_URL"],
        DEFAULT_PREVIEW_BASE_URL,
    )
        .trim_end_matches('/')
        .to_string()
}

fn agent_hook_addr() -> String {
    env_string(&["FASTSLIDES_AGENT_HOOK_ADDR"], DEFAULT_AGENT_HOOK_ADDR)
}

fn mcp_server_addr() -> String {
    env_string(&["FASTSLIDES_MCP_ADDR"], DEFAULT_MCP_SERVER_ADDR)
}

fn mcp_server_path() -> String {
    let raw = env_string(&["FASTSLIDES_MCP_PATH"], DEFAULT_MCP_SERVER_PATH);

    if raw.starts_with('/') {
        raw
    } else {
        format!("/{raw}")
    }
}

fn mcp_server_url(bind_addr: &str, route_path: &str) -> String {
    format!("http://{}{}", bind_addr, route_path)
}

fn build_mcp_server_config(cancellation_token: CancellationToken) -> StreamableHttpServerConfig {
    StreamableHttpServerConfig {
        stateful_mode: FASTSLIDES_MCP_STATEFUL_MODE,
        cancellation_token,
        ..Default::default()
    }
}

fn is_allowed_mcp_origin(origin: &str) -> bool {
    let parsed = match Url::parse(origin) {
        Ok(url) => url,
        Err(_) => return false,
    };

    matches!(
        parsed.host_str(),
        Some("localhost") | Some("127.0.0.1") | Some("::1")
    )
}

async fn validate_mcp_origin(request: AxumRequest, next: Next) -> axum::response::Response {
    if let Some(origin_value) = request.headers().get(axum::http::header::ORIGIN) {
        match origin_value.to_str() {
            Ok(origin) if is_allowed_mcp_origin(origin) => {}
            _ => {
                return (
                    axum::http::StatusCode::FORBIDDEN,
                    "Forbidden origin for local FastSlides MCP endpoint.",
                )
                    .into_response();
            }
        }
    }

    next.run(request).await
}

fn is_preview_url_reachable(preview_url: &str) -> bool {
    let parsed = match Url::parse(preview_url) {
        Ok(url) => url,
        Err(_) => return false,
    };

    let host = match parsed.host_str() {
        Some(host) => host,
        None => return false,
    };
    let port = parsed.port_or_known_default().unwrap_or(80);
    let addr = format!("{host}:{port}");
    let socket_addr = match addr.to_socket_addrs() {
        Ok(mut addrs) => addrs.next(),
        Err(_) => None,
    };

    match socket_addr {
        Some(socket) => TcpStream::connect_timeout(&socket, Duration::from_millis(1200)).is_ok(),
        None => false,
    }
}

fn wait_for_preview_url(preview_url: &str, attempts: usize) -> bool {
    for attempt in 0..attempts {
        if is_preview_url_reachable(preview_url) {
            return true;
        }
        if attempt + 1 < attempts {
            thread::sleep(Duration::from_millis(350));
        }
    }
    false
}

fn parse_line_column(message: &str) -> (Option<u64>, Option<u64>) {
    if let Some(captures) = Regex::new(r":(\d+):(\d+)")
        .ok()
        .and_then(|re| re.captures(message))
    {
        let line = captures.get(1).and_then(|m| m.as_str().parse::<u64>().ok());
        let column = captures.get(2).and_then(|m| m.as_str().parse::<u64>().ok());
        return (line, column);
    }

    if let Some(captures) = Regex::new(r"(?i)\bline\s+(\d+)\b")
        .ok()
        .and_then(|re| re.captures(message))
    {
        let line = captures.get(1).and_then(|m| m.as_str().parse::<u64>().ok());
        return (line, None);
    }

    (None, None)
}

fn build_preview_url_for_path(project_path: &str) -> String {
    let mut serializer = UrlQuerySerializer::new(String::new());
    serializer.append_pair("deckPath", project_path);
    let query = serializer.finish();
    format!("{}/?{query}", preview_base_url())
}

fn build_slide_preview_url(preview_url: &str, slide: usize) -> Result<String, String> {
    if slide == 0 {
        return Err("Slide number must be 1 or greater.".to_string());
    }

    let parsed = Url::parse(preview_url)
        .map_err(|error| format!("Failed to parse preview URL `{preview_url}`: {error}"))?;
    let mut query: HashMap<String, String> = parsed.query_pairs().into_owned().collect();
    query.insert("slide".to_string(), slide.to_string());
    query.insert("presenter".to_string(), "1".to_string());

    let mut serializer = UrlQuerySerializer::new(String::new());
    let mut entries: Vec<_> = query.into_iter().collect();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    for (key, value) in entries {
        serializer.append_pair(key.as_str(), value.as_str());
    }
    let query = serializer.finish();
    Ok(parsed
        .join(format!("?{query}").as_str())
        .map_err(|error| format!("Failed to build slide preview URL: {error}"))?
        .to_string())
}

fn default_slide_capture_output_dir(project_path: &Path) -> PathBuf {
    if let Some(projects_dir) = project_path.parent() {
        if projects_dir
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value == "projects")
        {
            if let Some(repo_root) = projects_dir.parent() {
                return repo_root
                    .join("output")
                    .join("playwright")
                    .join("fastslides-captures");
            }
        }
    }

    if let Ok(current_dir) = env::current_dir() {
        if current_dir
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value == "fastslides-desktop")
        {
            if let Some(repo_root) = current_dir.parent() {
                return repo_root
                    .join("output")
                    .join("playwright")
                    .join("fastslides-captures");
            }
        }
        return current_dir
            .join("output")
            .join("playwright")
            .join("fastslides-captures");
    }

    env::temp_dir().join("fastslides-captures")
}

fn resolve_slide_capture_output_dir(project_path: &Path, output_dir: Option<&str>) -> PathBuf {
    let trimmed = output_dir.map(str::trim).unwrap_or_default();
    if trimmed.is_empty() {
        return default_slide_capture_output_dir(project_path);
    }
    PathBuf::from(trimmed)
}

fn resolve_playwright_cli_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(explicit) = env::var("FASTSLIDES_PLAYWRIGHT_CLI") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            candidates.push(PathBuf::from(trimmed));
        }
    }

    if let Some(codex_home) = optional_codex_home_directory() {
        candidates.push(codex_home.join("skills/playwright/scripts/playwright_cli.sh"));
    }

    if let Ok(home) = env::var("HOME") {
        if !home.trim().is_empty() {
            candidates.push(PathBuf::from(home).join(".agents/skills/playwright/scripts/playwright_cli.sh"));
        }
    }

    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn build_subprocess_search_path(base: Option<&std::ffi::OsStr>) -> String {
    let mut entries: Vec<PathBuf> = base
        .map(env::split_paths)
        .into_iter()
        .flatten()
        .collect();
    for extra in [
        "/opt/homebrew/bin",
        "/opt/homebrew/sbin",
        "/usr/local/bin",
        "/usr/local/sbin",
    ] {
        let candidate = PathBuf::from(extra);
        if entries.iter().all(|existing| existing != &candidate) {
            entries.push(candidate);
        }
    }
    env::join_paths(entries)
        .ok()
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.trim().is_empty())
        .or_else(|| base.map(|value| value.to_string_lossy().into_owned()))
        .unwrap_or_default()
}

fn collect_png_artifacts(output_dir: &Path) -> Result<Vec<(PathBuf, SystemTime)>, String> {
    let mut artifacts = Vec::new();
    let mut pending_dirs = VecDeque::from([output_dir.to_path_buf()]);

    while let Some(next_dir) = pending_dirs.pop_front() {
        let entries = fs::read_dir(&next_dir).map_err(|error| {
            format!(
                "Failed to read capture output directory `{}`: {error}",
                next_dir.display()
            )
        })?;

        for entry in entries {
            let entry = entry.map_err(|error| {
                format!(
                    "Failed to read capture artifact entry in `{}`: {error}",
                    next_dir.display()
                )
            })?;
            let path = entry.path();
            if path.is_dir() {
                pending_dirs.push_back(path);
                continue;
            }
            let is_png = path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("png"));
            if !is_png {
                continue;
            }
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(UNIX_EPOCH);
            artifacts.push((path, modified));
        }
    }

    Ok(artifacts)
}

fn pick_slide_capture_artifact(
    before: &HashSet<PathBuf>,
    after: &[(PathBuf, SystemTime)],
) -> Option<PathBuf> {
    let mut new_paths: Vec<_> = after
        .iter()
        .filter(|(path, _)| !before.contains(path))
        .cloned()
        .collect();
    new_paths.sort_by(|left, right| right.1.cmp(&left.1));
    if let Some((path, _)) = new_paths.first() {
        return Some(path.clone());
    }

    let mut fallback = after.to_vec();
    fallback.sort_by(|left, right| right.1.cmp(&left.1));
    fallback.first().map(|(path, _)| path.clone())
}

fn run_playwright_cli_command(
    pwcli: &Path,
    session_name: &str,
    output_dir: &Path,
    args: &[String],
) -> Result<String, String> {
    let mut command = Command::new(pwcli);
    command.current_dir(output_dir);
    command.arg("--session").arg(session_name);
    command.env("PATH", build_subprocess_search_path(env::var_os("PATH").as_deref()));
    for arg in args {
        command.arg(arg);
    }

    let output = command.output().map_err(|error| {
        format!(
            "Failed to launch Playwright CLI `{}`: {error}",
            pwcli.display()
        )
    })?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("exit status {}", output.status)
    };
    Err(format!("Playwright command failed: {detail}"))
}

fn wait_for_slide_capture_ready(
    pwcli: &Path,
    session_name: &str,
    output_dir: &Path,
    target_slide_index: usize,
) -> Result<(), String> {
    let readiness_eval = format!(
        "() => {{
  const stage = document.querySelector('.preview-stage.presenter-mode');
  const deck = document.querySelector('.embedded-preview-deck.embedded-preview-single');
  const activeSlide = document.querySelector('.embedded-preview-deck .slide[data-active=\"true\"]');
  if (!stage || !deck || !activeSlide) {{
    return 'pending';
  }}
  return activeSlide.getAttribute('data-slide-index') === '{}' ? 'ready' : 'pending';
}}",
        target_slide_index
    );

    for _attempt in 0..30 {
        let output = run_playwright_cli_command(
            pwcli,
            session_name,
            output_dir,
            &["eval".to_string(), readiness_eval.clone()],
        )?;
        if output.contains("\"ready\"") {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(350));
    }

    Err(format!(
        "Timed out waiting for slide {} to become active in preview.",
        target_slide_index + 1
    ))
}

fn capture_slide_image_for_project(
    project_path: &Path,
    slide: Option<usize>,
    output_dir: Option<&str>,
    headed: bool,
) -> Result<SlideCaptureResponse, String> {
    let source = load_project_scene_source(project_path)?;
    if source.slides.is_empty() {
        return Err("Project does not contain any slides to capture.".to_string());
    }

    let slide_number = slide.unwrap_or(1);
    if slide_number == 0 {
        return Err("Slide number must be 1 or greater.".to_string());
    }
    if slide_number > source.slides.len() {
        return Err(format!(
            "Requested slide {} but project only has {} slides.",
            slide_number,
            source.slides.len()
        ));
    }

    let preview_url =
        build_slide_preview_url(build_preview_url_for_path(source.path.as_str()).as_str(), slide_number)?;
    if !wait_for_preview_url(preview_url.as_str(), 3) {
        return Err(format!(
            "Preview URL is not reachable for slide capture: {}",
            preview_url
        ));
    }

    let pwcli = resolve_playwright_cli_path().ok_or_else(|| {
        "Playwright wrapper script not found. Set FASTSLIDES_PLAYWRIGHT_CLI or install the playwright skill.".to_string()
    })?;

    let project_dir = Path::new(source.path.as_str());
    let output_dir = resolve_slide_capture_output_dir(project_dir, output_dir);
    fs::create_dir_all(&output_dir).map_err(|error| {
        format!(
            "Failed to create slide capture output directory `{}`: {error}",
            output_dir.display()
        )
    })?;

    let before = collect_png_artifacts(&output_dir)?;
    let before_paths: HashSet<PathBuf> = before.iter().map(|(path, _)| path.clone()).collect();
    let session_name = format!(
        "fs-{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        slide_number
    );
    let target_slide_index = slide_number - 1;

    let open_args = if headed {
        vec![
            "open".to_string(),
            preview_url.clone(),
            "--headed".to_string(),
        ]
    } else {
        vec!["open".to_string(), preview_url.clone()]
    };
    let capture_result = (|| -> Result<(), String> {
        let _ = run_playwright_cli_command(&pwcli, &session_name, &output_dir, &open_args)?;
        run_playwright_cli_command(
            &pwcli,
            &session_name,
            &output_dir,
            &["resize".to_string(), "1920".to_string(), "1080".to_string()],
        )?;
        wait_for_slide_capture_ready(&pwcli, &session_name, &output_dir, target_slide_index)?;
        run_playwright_cli_command(
            &pwcli,
            &session_name,
            &output_dir,
            &["snapshot".to_string()],
        )?;
        run_playwright_cli_command(
            &pwcli,
            &session_name,
            &output_dir,
            &["screenshot".to_string()],
        )?;
        Ok(())
    })();

    if let Err(close_error) = run_playwright_cli_command(
        &pwcli,
        &session_name,
        &output_dir,
        &["close".to_string()],
    ) {
        log::warn!("Failed to close Playwright session `{}`: {}", session_name, close_error);
    }

    capture_result?;

    let after = collect_png_artifacts(&output_dir)?;
    let artifact = pick_slide_capture_artifact(&before_paths, &after).ok_or_else(|| {
        format!(
            "Screenshot command completed but no PNG artifact was found in `{}`.",
            output_dir.display()
        )
    })?;
    let image_path = fs::canonicalize(&artifact).unwrap_or(artifact);
    let output_dir = fs::canonicalize(&output_dir).unwrap_or(output_dir);

    Ok(SlideCaptureResponse {
        ok: true,
        path: source.path,
        slide: slide_number,
        output_dir: path_to_string(&output_dir),
        image_path: path_to_string(&image_path),
        preview_url,
    })
}

fn start_mcp_server() -> McpServerStatus {
    let bind_addr = mcp_server_addr();
    let route_path = mcp_server_path();
    let server_url = mcp_server_url(bind_addr.as_str(), route_path.as_str());

    let (startup_tx, startup_rx) = std::sync::mpsc::channel::<Result<(), String>>();

    thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = startup_tx.send(Err(format!("Failed to build Tokio runtime: {error}")));
                return;
            }
        };

        runtime.block_on(async move {
            let listener = match tokio::net::TcpListener::bind(bind_addr.as_str()).await {
                Ok(listener) => listener,
                Err(error) => {
                    let _ = startup_tx.send(Err(format!(
                        "Failed to bind MCP server on {}: {}",
                        bind_addr, error
                    )));
                    return;
                }
            };

            let cancellation_token = CancellationToken::new();
            let service: StreamableHttpService<FastSlidesMcpServer, LocalSessionManager> =
                StreamableHttpService::new(
                    || Ok(FastSlidesMcpServer::new()),
                    Default::default(),
                    build_mcp_server_config(cancellation_token.child_token()),
                );
            let router = Router::new()
                .nest_service(route_path.as_str(), service)
                .layer(middleware::from_fn(validate_mcp_origin));
            let _ = startup_tx.send(Ok(()));

            log::info!(
                "FastSlides MCP server listening on {}",
                mcp_server_url(bind_addr.as_str(), route_path.as_str())
            );
            if let Err(error) = axum::serve(listener, router)
                .with_graceful_shutdown(async move { cancellation_token.cancelled_owned().await })
                .await
            {
                log::warn!("FastSlides MCP server stopped: {}", error);
            }
        });
    });

    match startup_rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(())) => McpServerStatus {
            running: true,
            url: server_url,
            error: None,
        },
        Ok(Err(error)) => McpServerStatus {
            running: false,
            url: server_url,
            error: Some(error),
        },
        Err(_) => McpServerStatus {
            running: false,
            url: server_url,
            error: Some("Timed out waiting for MCP server startup.".to_string()),
        },
    }
}

fn setup_mcp_tray_icon<R: Runtime>(
    app: &tauri::App<R>,
    status: &McpServerStatus,
) -> Result<(), String> {
    if !status.running {
        return Ok(());
    }

    let open_item = MenuItem::with_id(
        app,
        MENU_OPEN_MAIN_WINDOW_ID,
        "Open FastSlides",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to create tray open item: {error}"))?;
    let install_codex_item = MenuItem::with_id(
        app,
        MENU_INSTALL_CODEX_MCP_ID,
        "Install MCP in Codex",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to create tray install item: {error}"))?;
    let export_item = MenuItem::with_id(
        app,
        MENU_EXPORT_SKILL_ID,
        "Download FastSlides Skill…",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to create tray export item: {error}"))?;
    let separator = PredefinedMenuItem::separator(app)
        .map_err(|error| format!("Failed to create tray separator: {error}"))?;
    let tray_menu = Menu::with_items(
        app,
        &[&open_item, &install_codex_item, &separator, &export_item],
    )
    .map_err(|error| format!("Failed to create tray menu: {error}"))?;

    let mut tray_builder = TrayIconBuilder::with_id(MCP_TRAY_ICON_ID)
        .menu(&tray_menu)
        .show_menu_on_left_click(true)
        .tooltip(format!("FastSlides MCP server running at {}", status.url));

    if let Some(icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(icon).icon_as_template(true);
    }

    tray_builder
        .build(app)
        .map_err(|error| format!("Failed to create tray icon: {error}"))?;
    Ok(())
}

fn handle_agent_hook_request(
    method: &Method,
    request_url: &str,
    request: &mut Request,
) -> Response<Cursor<Vec<u8>>> {
    let parsed = Url::parse(format!("http://localhost{request_url}").as_str());
    let parsed_url = match parsed {
        Ok(url) => url,
        Err(error) => return json_error_response(400, format!("Invalid request URL: {error}")),
    };

    let path = parsed_url.path();

    match (method, path) {
        (&Method::Options, _) => empty_response(204),
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
        (&Method::Get, "/design-system") => match get_design_system() {
            Ok(registry) => json_response(200, registry),
            Err(error) => json_error_response(500, error),
        },
        (&Method::Get, "/component-catalog") => match get_component_catalog() {
            Ok(catalog) => json_response(200, catalog),
            Err(error) => json_error_response(500, error),
        },
        (&Method::Get, "/component-template") => {
            let name = parsed_url
                .query_pairs()
                .find_map(|(key, value)| (key == "name").then(|| value.into_owned()))
                .unwrap_or_default();

            if name.trim().is_empty() {
                return json_error_response(
                    400,
                    "Missing required query parameter: name".to_string(),
                );
            }

            match get_component_template(name) {
                Ok(template) => json_response(200, template),
                Err(error) => json_error_response(400, error),
            }
        }
        (&Method::Get, "/composition-template") => {
            let name = parsed_url
                .query_pairs()
                .find_map(|(key, value)| (key == "name").then(|| value.into_owned()))
                .unwrap_or_default();

            if name.trim().is_empty() {
                return json_error_response(
                    400,
                    "Missing required query parameter: name".to_string(),
                );
            }

            match get_composition_template(name) {
                Ok(template) => json_response(200, template),
                Err(error) => json_error_response(400, error),
            }
        }
        (&Method::Get, "/recipe-template") => {
            let name = parsed_url
                .query_pairs()
                .find_map(|(key, value)| (key == "name").then(|| value.into_owned()))
                .unwrap_or_default();

            if name.trim().is_empty() {
                return json_error_response(
                    400,
                    "Missing required query parameter: name".to_string(),
                );
            }

            match get_recipe_template(name) {
                Ok(template) => json_response(200, template),
                Err(error) => json_error_response(400, error),
            }
        }
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
        (&Method::Get, "/analyze-project") => {
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

            match analyze_project(project_path) {
                Ok(analysis) => json_response(200, analysis),
                Err(error) => json_error_response(400, error),
            }
        }
        (&Method::Get, "/compile-project-scene") => {
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

            match compile_project_scene(project_path) {
                Ok(scene) => json_response(200, scene),
                Err(error) => json_error_response(400, error),
            }
        }
        (&Method::Get, "/compile-project-scene-manifest") => {
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

            match compile_project_scene_manifest(project_path) {
                Ok(scene) => json_response(200, scene),
                Err(error) => json_error_response(400, error),
            }
        }
        (&Method::Get, "/compile-project-scene-slide") => {
            let project_path = parsed_url
                .query_pairs()
                .find_map(|(key, value)| (key == "path").then(|| value.into_owned()))
                .unwrap_or_default();
            let raw_index = parsed_url
                .query_pairs()
                .find_map(|(key, value)| (key == "index").then(|| value.into_owned()))
                .unwrap_or_default();

            if project_path.trim().is_empty() {
                return json_error_response(
                    400,
                    "Missing required query parameter: path".to_string(),
                );
            }

            let index = match raw_index.parse::<usize>() {
                Ok(value) => value,
                Err(_) => {
                    return json_error_response(
                        400,
                        "Missing or invalid query parameter: index".to_string(),
                    );
                }
            };

            match compile_project_scene_slide(project_path, index) {
                Ok(scene) => json_response(200, scene),
                Err(error) => json_error_response(400, error),
            }
        }
        (&Method::Get, "/project-css") => {
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

            match read_project_css(project_path) {
                Ok(css) => json_response(200, css),
                Err(error) => json_error_response(400, error),
            }
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
        (&Method::Post, "/install-codex-mcp") => match install_codex_mcp_server() {
            Ok(detail) => json_response(200, detail),
            Err(error) => json_error_response(400, error),
        },
        (&Method::Post, "/save-component-template") => {
            let payload = match read_json_body::<SaveComponentPayload>(request) {
                Ok(value) => value,
                Err(error) => return json_error_response(400, error),
            };
            match save_component_template(payload) {
                Ok(saved) => json_response(200, saved),
                Err(error) => json_error_response(400, error),
            }
        }
        (&Method::Post, "/capture-slide-image") => {
            let payload = match read_json_body::<SlideCapturePayload>(request) {
                Ok(value) => value,
                Err(error) => return json_error_response(400, error),
            };

            match capture_slide_image(payload.path, payload.slide, payload.output_dir, payload.headed) {
                Ok(capture) => json_response(200, capture),
                Err(error) => json_error_response(400, error),
            }
        }
        (&Method::Post, "/project-css") => {
            let payload = match read_json_body::<ProjectCssPayload>(request) {
                Ok(value) => value,
                Err(error) => return json_error_response(400, error),
            };
            match save_project_css(payload.path, payload.css) {
                Ok(()) => json_response(200, serde_json::json!({ "ok": true })),
                Err(error) => json_error_response(400, error),
            }
        }
        _ => json_error_response(404, format!("Unknown endpoint: {:?} {}", method, path)),
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

        for request in server.incoming_requests() {
            thread::spawn(move || {
                let mut request = request;
                let method = request.method().clone();
                let request_url = request.url().to_string();
                let response =
                    handle_agent_hook_request(&method, request_url.as_str(), &mut request);
                let _ = request.respond(response);
            });
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    start_agent_hook_server();
    let mcp_status = start_mcp_server();

    if mcp_status.running {
        log::info!("FastSlides MCP endpoint available at {}", mcp_status.url);
    } else if let Some(error) = &mcp_status.error {
        log::warn!(
            "FastSlides MCP endpoint unavailable at {}: {}",
            mcp_status.url,
            error
        );
    } else {
        log::warn!("FastSlides MCP endpoint unavailable at {}", mcp_status.url);
    }

    tauri::Builder::default()
        .menu(build_app_menu)
        .on_menu_event(|app, event| {
            if event.id() == MENU_OPEN_MAIN_WINDOW_ID {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            } else if event.id() == MENU_OPEN_SETTINGS_ID {
                if let Err(error) = app.emit(MENU_OPEN_SETTINGS_EVENT, ()) {
                    log::warn!("Failed to emit settings menu event: {}", error);
                }
            } else if event.id() == MENU_INSTALL_CODEX_MCP_ID {
                match install_codex_mcp_server_config() {
                    Ok(installed) => {
                        let message = format!(
                            "Installed FastSlides MCP as `{}` in Codex.\n\nConfig: {}\nURL: {}",
                            installed.server_name, installed.config_path, installed.url
                        );
                        log::info!("{message}");
                        app.dialog()
                            .message(message)
                            .title("Install MCP in Codex")
                            .kind(MessageDialogKind::Info)
                            .show(|_| {});
                    }
                    Err(error) => {
                        log::warn!("Failed to install FastSlides MCP in Codex: {}", error);
                        app.dialog()
                            .message(error)
                            .title("Install MCP in Codex")
                            .kind(MessageDialogKind::Error)
                            .show(|_| {});
                    }
                }
            } else if event.id() == MENU_EXPORT_SKILL_ID {
                if let Err(error) = app.emit(MENU_EXPORT_SKILL_EVENT, ()) {
                    log::warn!("Failed to emit skill export menu event: {}", error);
                }
            }
        })
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ =
                        apply_vibrancy(&window, NSVisualEffectMaterial::Sidebar, None, Some(10.0));
                }
            }

            if let Err(error) = setup_mcp_tray_icon(app, &mcp_status) {
                log::warn!("Failed to initialize MCP tray icon: {}", error);
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
            analyze_project,
            capture_slide_image,
            compile_project_scene,
            compile_project_scene_manifest,
            compile_project_scene_slide,
            create_project,
            get_component_catalog,
            get_component_template,
            get_composition_template,
            get_design_system,
            get_recipe_template,
            get_app_state,
            install_codex_mcp_server,
            load_project,
            open_project,
            open_in_file_manager,
            export_fastslides_skill,
            read_project_css,
            remove_project,
            remove_projects_root,
            save_project,
            save_component_template,
            save_project_css,
            resolve_project_asset_data_url,
            start_project_scene_session,
            validate_project,
            toggle_project_pin
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_scene_source() -> ProjectSceneSource {
        ProjectSceneSource {
            path: "/tmp/sample".to_string(),
            project: Some("sample".to_string()),
            title: Some("Sample".to_string()),
            subtitle: Some("Subtitle".to_string()),
            date: Some("March 2026".to_string()),
            deck_class_name: Some("deck sample".to_string()),
            slides: vec![
                "<Canvas cols={50} rows={25}><Area x={1} y={1} w={10} h={5}><h1>Slide One</h1></Area></Canvas>".to_string(),
                "<Canvas cols={50} rows={25}><Area x={1} y={1} w={10} h={5}><h1>Slide Two</h1></Area></Canvas>".to_string(),
                "<Canvas cols={50} rows={25}><Area x={1} y={1} w={10} h={5}><h1>Slide Three</h1></Area></Canvas>".to_string(),
            ],
        }
    }

    #[test]
    fn prioritize_scene_slide_indices_biases_requested_slide_then_neighbors() {
        assert_eq!(
            prioritize_scene_slide_indices(7, 0),
            vec![0, 1, 2, 3, 4, 5, 6]
        );
        assert_eq!(prioritize_scene_slide_indices(5, 2), vec![2, 3, 1, 4, 0]);
        assert_eq!(prioritize_scene_slide_indices(4, 99), vec![3, 2, 1, 0]);
    }

    #[test]
    fn collect_project_scene_session_events_emits_manifest_then_priority_slide_then_complete() {
        let events = collect_project_scene_session_events(sample_scene_source(), 2, 1)
            .expect("session events should compile");

        assert!(matches!(
            &events[0],
            ProjectSceneSessionEventPayload::Manifest { scene } if scene.slide_count == 3
        ));

        assert!(matches!(
            &events[1],
            ProjectSceneSessionEventPayload::SlideReady { slide } if slide.index == 2
        ));

        let ready_indices: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                ProjectSceneSessionEventPayload::SlideReady { slide } => Some(slide.index),
                _ => None,
            })
            .collect();
        assert_eq!(ready_indices, vec![2, 1, 0]);

        assert!(matches!(
            events.last().expect("complete event should exist"),
            ProjectSceneSessionEventPayload::Complete {
                ready_count: 3,
                error_count: 0
            }
        ));
    }

    #[test]
    fn collect_project_scene_session_events_handles_empty_deck() {
        let mut source = sample_scene_source();
        source.slides.clear();

        let events = collect_project_scene_session_events(source, 0, 4)
            .expect("empty deck should still produce session events");

        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            ProjectSceneSessionEventPayload::Manifest { scene } if scene.slide_count == 0
        ));
        assert!(matches!(
            &events[1],
            ProjectSceneSessionEventPayload::Complete {
                ready_count: 0,
                error_count: 0
            }
        ));
    }

    #[test]
    fn slide_contract_warnings_flag_dense_takeaway_metric_grid_and_callout() {
        let slide = r#"
<Canvas cols={50} rows={25} gap="1px">
  <Area x={2} y={4} w={30} h={4}>
    <Takeaway>The same deck should exercise primitive layouts, metrics, and captions without dropping back to legacy HTML patterns.</Takeaway>
  </Area>
  <Area x={2} y={10} w={30} h={10}>
    <Grid cols={3} gap="md">
      <Metric label="MDX mode" value="Runtime" hint="Content-only decks" />
      <Metric label="Layout API" value="Canvas-first" hint="Area plus bounded primitives" />
      <Metric label="Visual QA" value="Screenshots" hint="Playwright-powered" />
    </Grid>
  </Area>
  <Area x={34} y={10} w={14} h={10}>
    <Callout title="Check">
      Metrics, captions, and internal grids should align cleanly inside one region without growing random borders or side accents.
    </Callout>
  </Area>
</Canvas>
"#;

        let warnings = slide_contract_warnings(slide);

        assert!(warnings.iter().any(|warning| warning.contains("Takeaway is too dense")));
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("metric grid is too tight")));
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("Callout copy is too dense")));
    }

    #[test]
    fn slide_contract_warnings_allow_spacious_scorecard_layout() {
        let slide = r#"
<Canvas cols={50} rows={25} gap="1px">
  <Area x={2} y={4} w={30} h={6}>
    <Takeaway>Stable primitives make complex decks easier to generate and review.</Takeaway>
  </Area>
  <Area x={2} y={11} w={33} h={6}>
    <Grid cols={3} gap="sm">
      <Metric label="Grid" value="50x25" hint="Base canvas" />
      <Metric label="QA" value="Live" hint="Preview first" />
      <Metric label="Export" value="Shared" hint="One scene model" />
    </Grid>
  </Area>
  <Area x={36} y={10} w={12} h={9}>
    <Callout title="Check">Short commentary sits cleanly in the rail.</Callout>
  </Area>
</Canvas>
"#;

        assert!(slide_contract_warnings(slide).is_empty());
    }

    #[test]
    fn design_system_registry_uses_thin_chrome_and_small_variant_sets() {
        let registry = design_system_registry();

        assert_eq!(registry.default_frame.cols, 50);
        assert_eq!(registry.default_frame.rows, 25);
        assert!(registry.default_frame.header_rows <= 2);
        assert!(registry.default_frame.footer_rows <= 2);
        assert!(registry.default_frame.body_rows > registry.default_frame.header_rows);
        assert!(registry.default_frame.body_rows > registry.default_frame.footer_rows);

        assert_eq!(registry.compositions.len(), 5);
        assert_eq!(registry.recipes.len(), 5);
        assert_eq!(registry.sections.len(), 3);

        assert!(registry
            .primitives
            .iter()
            .all(|primitive| primitive.variants.len() <= 3));
        assert!(registry
            .compositions
            .iter()
            .all(|composition| composition.variants.len() <= 3));
    }

    #[test]
    fn subprocess_search_path_includes_common_npx_locations() {
        let path = build_subprocess_search_path(Some(std::ffi::OsStr::new("/usr/bin:/bin")));
        let entries: Vec<_> = env::split_paths(&path).collect();

        assert!(entries.contains(&PathBuf::from("/opt/homebrew/bin")));
        assert!(entries.contains(&PathBuf::from("/usr/local/bin")));
    }

    #[test]
    fn design_system_registry_sections_reference_known_recipes() {
        let registry = design_system_registry();
        let recipe_names: HashSet<_> = registry.recipes.iter().map(|recipe| recipe.name.clone()).collect();

        assert!(registry.sections.iter().all(|section| section
            .recipes
            .iter()
            .all(|recipe| recipe_names.contains(recipe))));
    }

    #[test]
    fn known_template_names_match_design_system_registry() {
        let registry = design_system_registry();
        let primitive_names: Vec<_> = registry
            .primitives
            .iter()
            .map(|primitive| primitive.name.clone())
            .collect();
        let composition_names: Vec<_> = registry
            .compositions
            .iter()
            .map(|composition| composition.name.clone())
            .collect();
        let recipe_names: Vec<_> = registry
            .recipes
            .iter()
            .map(|recipe| recipe.name.clone())
            .collect();

        assert_eq!(primitive_names, known_primitive_names());
        assert_eq!(composition_names, known_composition_names());
        assert_eq!(recipe_names, known_recipe_names());
    }

    #[test]
    fn registered_primitive_templates_are_spatial_and_warning_clean() {
        for name in known_primitive_names() {
            let template = primitive_template(&name)
                .unwrap_or_else(|error| panic!("primitive template `{name}` should resolve: {error}"));
            let slide = format!(
                "<section className=\"slide\">\n  <Canvas cols={{50}} rows={{25}} gap=\"1px\">\n{}\n  </Canvas>\n</section>",
                template.mdx
            );
            let slides = extract_slides(&slide);
            let warnings = slide_contract_warnings(&slides[0]);

            assert_eq!(slides.len(), 1, "primitive `{name}` should produce one wrapped slide");
            validate_scene_slide_contract(&slides[0], 0)
                .unwrap_or_else(|error| panic!("primitive `{name}` should satisfy spatial contract: {error}"));
            assert!(
                warnings.is_empty(),
                "primitive `{name}` should not trigger base contract warnings: {warnings:?}"
            );
        }
    }

    #[test]
    fn registered_composition_templates_are_spatial_and_warning_clean() {
        for name in known_composition_names() {
            let template = composition_template(&name)
                .unwrap_or_else(|error| panic!("composition template `{name}` should resolve: {error}"));
            let slide = format!(
                "<section className=\"slide\">\n  <Canvas cols={{50}} rows={{25}} gap=\"1px\">\n{}\n  </Canvas>\n</section>",
                template.mdx
            );
            let slides = extract_slides(&slide);
            let warnings = slide_contract_warnings(&slides[0]);

            assert_eq!(slides.len(), 1, "composition `{name}` should produce one wrapped slide");
            validate_scene_slide_contract(&slides[0], 0)
                .unwrap_or_else(|error| panic!("composition `{name}` should satisfy spatial contract: {error}"));
            assert!(
                warnings.is_empty(),
                "composition `{name}` should not trigger base contract warnings: {warnings:?}"
            );
        }
    }

    #[test]
    fn registered_recipe_templates_are_spatial_and_warning_clean() {
        for name in known_recipe_names() {
            let template = recipe_template(&name)
                .unwrap_or_else(|error| panic!("recipe template `{name}` should resolve: {error}"));
            let slides = extract_slides(&template.mdx);
            let warnings = slide_contract_warnings(&slides[0]);

            assert_eq!(slides.len(), 1, "recipe `{name}` should contain one slide");
            validate_scene_slide_contract(&slides[0], 0)
                .unwrap_or_else(|error| panic!("recipe `{name}` should satisfy spatial contract: {error}"));
            assert!(
                warnings.is_empty(),
                "recipe `{name}` should not trigger base contract warnings: {warnings:?}"
            );
        }
    }

    #[test]
    fn chart_component_is_counted_compiled_and_warning_clean() {
        let slide = r#"
<section className="slide">
  <Canvas cols={50} rows={25} gap="1px">
    <Area x={2} y={2} w={14} h={1}>
      <Kicker>Evidence</Kicker>
    </Area>
    <Area x={2} y={4} w={46} h={4}>
      <Takeaway>Charts should render through the shared scene model, not raw HTML.</Takeaway>
    </Area>
    <Area x={2} y={10} w={30} h={11}>
      <Chart
        type="bar"
        title="Priority"
        data="Workflow:88;Distribution:76;Feedback:69"
        suffix="%"
        highlight="Workflow"
      />
    </Area>
    <Area x={34} y={10} w={14} h={11}>
      <Callout title="Read-through">One focused chart beats a hand-built box garden.</Callout>
    </Area>
  </Canvas>
</section>
"#;

        let counts = component_counts_for_slide(slide);
        assert_eq!(counts.get("Chart").copied(), Some(1));
        assert!(slide_contract_warnings(slide).is_empty());

        let compiled = compile_slide_nodes(slide);
        assert!(compiled.iter().any(|node| matches!(
            node,
            SceneNode::Canvas { children, .. }
                if children.iter().any(|child| matches!(
                    child,
                    SceneNode::Area { children, .. }
                        if children.iter().any(|grandchild| matches!(grandchild, SceneNode::Chart { .. }))
                ))
        )));
    }

    #[test]
    fn component_catalog_includes_builtin_patterns_and_marks() {
        let catalog = build_component_catalog().expect("component catalog should build");
        let names: HashSet<_> = catalog.items.iter().map(|item| item.name.as_str()).collect();
        assert!(names.contains("Arrow"));
        assert!(names.contains("ImageFigure"));
        assert!(names.contains("TrendChartCommentary"));
    }

    #[test]
    fn save_component_to_path_round_trips_custom_entry() {
        let library_path = env::temp_dir().join(format!(
            "fastslides-component-library-{}.json",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        if library_path.exists() {
            let _ = fs::remove_file(&library_path);
        }

        let payload = SaveComponentPayload {
            name: "SavedCallout".to_string(),
            family: "narrative".to_string(),
            summary: "Saved custom callout".to_string(),
            tags: Some(vec!["saved".to_string(), "callout".to_string()]),
            mdx: "<Area x={34} y={10} w={14} h={8}><Callout title=\"Saved\">Reusable note</Callout></Area>".to_string(),
            notes: Some(vec!["Captured from a good slide.".to_string()]),
        };

        let saved = save_component_to_path(&library_path, payload)
            .expect("saving component should succeed");
        let records = load_saved_components_from(&library_path)
            .expect("saved component library should load");

        assert_eq!(saved.component.name, "SavedCallout");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].family, "narrative");
        assert_eq!(records[0].summary, "Saved custom callout");

        let _ = fs::remove_file(&library_path);
    }

    #[test]
    fn codex_mcp_install_inserts_fastslides_block_when_missing() {
        let source = r#"model = "gpt-5.4"

[mcp_servers.deepwiki]
url = "https://mcp.deepwiki.com/mcp"
"#;

        let (updated, status) = upsert_codex_mcp_server_block(
            source,
            "fastslides",
            "http://127.0.0.1:38474/mcp",
        )
        .expect("upsert should succeed");

        assert_eq!(status, CodexInstallStatus::Installed);
        assert!(updated.contains("[mcp_servers.fastslides]"));
        assert!(updated.contains("url = \"http://127.0.0.1:38474/mcp\""));
        assert!(updated.contains("[mcp_servers.deepwiki]"));
    }

    #[test]
    fn codex_mcp_install_updates_existing_fastslides_block() {
        let source = r#"model = "gpt-5.4"

[mcp_servers.fastslides]
command = "old-fastslides"

[mcp_servers.linear]
url = "https://mcp.linear.app/mcp"
"#;

        let (updated, status) = upsert_codex_mcp_server_block(
            source,
            "fastslides",
            "http://127.0.0.1:38474/mcp",
        )
        .expect("upsert should succeed");

        assert_eq!(status, CodexInstallStatus::Updated);
        assert!(updated.contains("[mcp_servers.fastslides]"));
        assert!(updated.contains("url = \"http://127.0.0.1:38474/mcp\""));
        assert!(!updated.contains("old-fastslides"));
        assert!(updated.contains("[mcp_servers.linear]"));
    }

    #[test]
    fn codex_mcp_install_is_idempotent_when_fastslides_block_matches() {
        let source = r#"model = "gpt-5.4"

[mcp_servers.fastslides]
url = "http://127.0.0.1:38474/mcp"
"#;

        let (updated, status) = upsert_codex_mcp_server_block(
            source,
            "fastslides",
            "http://127.0.0.1:38474/mcp",
        )
        .expect("upsert should succeed");

        assert_eq!(status, CodexInstallStatus::Unchanged);
        assert_eq!(updated.trim(), source.trim());
    }

    #[test]
    fn build_slide_preview_url_adds_slide_and_presenter_params() {
        let preview_url =
            build_slide_preview_url("http://127.0.0.1:1420/?deckPath=%2Ftmp%2Fdeck", 5)
                .expect("slide preview URL should build");
        let parsed = Url::parse(&preview_url).expect("url should parse");
        let params: HashMap<_, _> = parsed.query_pairs().into_owned().collect();

        assert_eq!(params.get("deckPath"), Some(&"/tmp/deck".to_string()));
        assert_eq!(params.get("slide"), Some(&"5".to_string()));
        assert_eq!(params.get("presenter"), Some(&"1".to_string()));
    }

    #[test]
    fn mcp_server_uses_stateless_streamable_http_mode() {
        let config = build_mcp_server_config(CancellationToken::new());
        assert!(!config.stateful_mode);
    }

    #[test]
    fn pick_slide_capture_artifact_prefers_new_pngs_then_newest_file() {
        let before = HashSet::from([PathBuf::from("/tmp/older.png")]);
        let after = vec![
            (PathBuf::from("/tmp/older.png"), UNIX_EPOCH + Duration::from_secs(1)),
            (PathBuf::from("/tmp/newer.png"), UNIX_EPOCH + Duration::from_secs(2)),
            (
                PathBuf::from("/tmp/newest.png"),
                UNIX_EPOCH + Duration::from_secs(3),
            ),
        ];

        assert_eq!(
            pick_slide_capture_artifact(&before, &after),
            Some(PathBuf::from("/tmp/newest.png"))
        );

        let before_all = HashSet::from([
            PathBuf::from("/tmp/older.png"),
            PathBuf::from("/tmp/newer.png"),
            PathBuf::from("/tmp/newest.png"),
        ]);

        assert_eq!(
            pick_slide_capture_artifact(&before_all, &after),
            Some(PathBuf::from("/tmp/newest.png"))
        );
    }

    #[test]
    fn collect_png_artifacts_walks_nested_directories() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should advance")
            .as_nanos();
        let root = env::temp_dir().join(format!("fastslides-capture-test-{unique}"));
        let nested = root.join("nested");
        fs::create_dir_all(&nested).expect("nested capture dir should create");
        let png_path = nested.join("slide.png");
        fs::write(&png_path, b"png").expect("png fixture should write");

        let artifacts = collect_png_artifacts(&root).expect("artifact scan should succeed");
        let artifact_paths: HashSet<_> = artifacts.into_iter().map(|(path, _)| path).collect();
        assert!(artifact_paths.contains(&png_path));

        fs::remove_dir_all(&root).expect("temp capture dir should remove");
    }
}
