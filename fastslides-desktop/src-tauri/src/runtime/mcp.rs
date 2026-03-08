use super::capture::{
    build_preview_url_for_path, capture_slide_image_for_project, is_preview_url_reachable,
    parse_line_column,
};
use crate::commands::{
    analyze_project, compile_project_scene, compile_project_scene_manifest,
    compile_project_scene_slide, get_app_state, get_component_catalog, get_component_template,
    get_composition_template, get_design_system, get_recipe_template, open_project,
    save_component_template, validate_project,
};
use crate::constants::{
    DEFAULT_MCP_SERVER_ADDR, DEFAULT_MCP_SERVER_PATH, FASTSLIDES_MCP_STATEFUL_MODE,
};
use crate::design_system::SaveComponentPayload;
use axum::{
    extract::Request as AxumRequest, middleware, middleware::Next, response::IntoResponse, Router,
};
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
use std::{path::Path, thread, time::Duration};
use tokio_util::sync::CancellationToken;
use url::Url;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct McpServerStatus {
    pub(crate) running: bool,
    pub(crate) url: String,
    pub(crate) error: Option<String>,
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
struct FastSlidesMcpServer {
    tool_router: ToolRouter<Self>,
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
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "service": "fastslides-agent-hook",
        }))
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

        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "preview_url": build_preview_url_for_path(project_path),
        }))
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
        let capture = capture_slide_image_for_project(
            Path::new(&params.path),
            params.slide,
            params.output_dir.as_deref(),
            params.headed.unwrap_or(false),
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

fn mcp_server_addr() -> String {
    super::env_string(&["FASTSLIDES_MCP_ADDR"], DEFAULT_MCP_SERVER_ADDR)
}

fn mcp_server_path() -> String {
    let raw = super::env_string(&["FASTSLIDES_MCP_PATH"], DEFAULT_MCP_SERVER_PATH);
    if raw.starts_with('/') {
        raw
    } else {
        format!("/{raw}")
    }
}

pub(crate) fn configured_mcp_server_url() -> String {
    super::mcp_server_url(mcp_server_addr().as_str(), mcp_server_path().as_str())
}

pub(crate) fn build_mcp_server_config(
    cancellation_token: CancellationToken,
) -> StreamableHttpServerConfig {
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

pub(crate) fn start_mcp_server() -> McpServerStatus {
    let bind_addr = mcp_server_addr();
    let route_path = mcp_server_path();
    let server_url = super::mcp_server_url(bind_addr.as_str(), route_path.as_str());

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
                super::mcp_server_url(bind_addr.as_str(), route_path.as_str())
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
