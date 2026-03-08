use super::capture::{build_preview_url_for_path, capture_slide_image_for_project};
use crate::commands::{
    analyze_project, compile_project_scene, compile_project_scene_manifest,
    compile_project_scene_slide, get_app_state, get_component_catalog, get_component_template,
    get_composition_template, get_design_system, get_recipe_template, install_codex_mcp_server,
    open_project, read_project_css, save_component_template, save_project_css, validate_project,
};
use crate::constants::DEFAULT_AGENT_HOOK_ADDR;
use crate::design_system::SaveComponentPayload;
use serde::Deserialize;
use std::{io::Cursor, path::Path, thread};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use url::Url;

#[derive(Debug, Deserialize)]
struct PathPayload {
    path: String,
}

#[derive(Debug, Deserialize)]
struct ProjectCssPayload {
    path: String,
    css: String,
}

#[derive(Debug, Deserialize)]
struct SlideCapturePayload {
    path: String,
    slide: Option<usize>,
    output_dir: Option<String>,
    headed: Option<bool>,
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

fn json_response(status_code: u16, payload: impl serde::Serialize) -> Response<Cursor<Vec<u8>>> {
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
        serde_json::json!({
            "ok": false,
            "error": message,
        }),
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

fn agent_hook_addr() -> String {
    super::env_string(&["FASTSLIDES_AGENT_HOOK_ADDR"], DEFAULT_AGENT_HOOK_ADDR)
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
            serde_json::json!({
                "ok": true,
                "service": "fastslides-agent-hook",
            }),
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
                serde_json::json!({
                    "ok": true,
                    "preview_url": build_preview_url_for_path(project_path.as_str()),
                }),
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

            match capture_slide_image_for_project(
                Path::new(&payload.path),
                payload.slide,
                payload.output_dir.as_deref(),
                payload.headed.unwrap_or(false),
            ) {
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

pub(crate) fn start_agent_hook_server() {
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
