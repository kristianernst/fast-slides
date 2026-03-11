use base64::Engine;
use std::{
    fs,
    path::{Path, PathBuf},
    thread,
};
use tauri::Emitter;

use crate::config::normalize_existing_project_directory;
use crate::constants::SCENE_SESSION_EVENT_NAME;
use crate::deck::{local_asset_path, mime_type_for_path, resolve_relative_path};
use crate::runtime::{capture_slide_image_for_project, SlideCaptureResponse};
use crate::scene::{
    build_project_analysis, build_project_scene, build_project_scene_manifest,
    build_project_scene_slide, emit_project_scene_session_events, load_project_scene_source,
    next_scene_session_id, preferred_scene_session_worker_count, ProjectAnalysis, ProjectScene,
    ProjectSceneManifest, ProjectSceneSessionEvent, ProjectSceneSessionEventPayload,
    ProjectSceneSessionHandle, SceneSlide,
};

pub(crate) struct ProjectAssetBytes {
    pub(crate) bytes: Vec<u8>,
    pub(crate) mime_type: &'static str,
}

fn normalized_session_id(session_id: Option<String>) -> String {
    session_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(next_scene_session_id)
}

fn session_handle(session_id: &str, path: &str, slide_count: usize) -> ProjectSceneSessionHandle {
    ProjectSceneSessionHandle {
        session_id: session_id.to_string(),
        path: path.to_string(),
        slide_count,
    }
}

fn resolved_project_asset_path(
    project_path: &str,
    raw_src: &str,
) -> Result<Option<PathBuf>, String> {
    let canonical_project = normalize_existing_project_directory(project_path)?;
    let Some(relative_path) = local_asset_path(raw_src) else {
        return Ok(None);
    };

    if relative_path == ".." || relative_path.starts_with("../") {
        return Err(format!("Invalid traversal asset path: {raw_src}"));
    }

    resolve_relative_path(&canonical_project, &relative_path)
        .ok_or_else(|| format!("Asset path escapes project folder: {raw_src}"))
        .map(Some)
}

fn asset_data_url(mime_type: &str, asset_bytes: &[u8]) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(asset_bytes);
    format!("data:{mime_type};base64,{encoded}")
}

pub(crate) fn read_project_asset(
    project_path: &str,
    raw_src: &str,
) -> Result<Option<ProjectAssetBytes>, String> {
    if raw_src.trim().is_empty() || raw_src.trim().starts_with('#') {
        return Ok(None);
    }

    let Some(resolved_path) = resolved_project_asset_path(project_path, raw_src)? else {
        return Ok(None);
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

    let bytes = fs::read(&resolved_path)
        .map_err(|error| format!("Failed to read {}: {error}", resolved_path.display()))?;
    Ok(Some(ProjectAssetBytes {
        bytes,
        mime_type: mime_type_for_path(&resolved_path),
    }))
}

#[tauri::command]
pub(crate) fn analyze_project(path: String) -> Result<ProjectAnalysis, String> {
    build_project_analysis(Path::new(&path))
}

#[tauri::command]
pub(crate) fn compile_project_scene(path: String) -> Result<ProjectScene, String> {
    build_project_scene(Path::new(&path))
}

#[tauri::command]
pub(crate) fn compile_project_scene_manifest(path: String) -> Result<ProjectSceneManifest, String> {
    build_project_scene_manifest(Path::new(&path))
}

#[tauri::command]
pub(crate) fn compile_project_scene_slide(
    path: String,
    index: usize,
) -> Result<SceneSlide, String> {
    build_project_scene_slide(Path::new(&path), index)
}

#[tauri::command]
pub(crate) fn capture_slide_image(
    path: String,
    slide: Option<usize>,
    output_dir: Option<String>,
    headed: Option<bool>,
) -> Result<SlideCaptureResponse, String> {
    capture_slide_image_for_project(
        Path::new(&path),
        slide,
        output_dir.as_deref(),
        headed.unwrap_or(false),
    )
}

#[tauri::command]
pub(crate) fn start_project_scene_session(
    app: tauri::AppHandle,
    path: String,
    priority_index: Option<usize>,
    session_id: Option<String>,
) -> Result<ProjectSceneSessionHandle, String> {
    let source = load_project_scene_source(Path::new(&path))?;
    let session_id = normalized_session_id(session_id);
    let handle = session_handle(&session_id, &source.path, source.slides.len());
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
pub(crate) fn resolve_project_asset_data_url(
    project_path: String,
    raw_src: String,
) -> Result<String, String> {
    let Some(asset) = read_project_asset(&project_path, &raw_src)? else {
        return Ok(raw_src);
    };
    Ok(asset_data_url(asset.mime_type, &asset.bytes))
}
