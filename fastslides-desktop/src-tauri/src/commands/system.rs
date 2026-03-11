use std::{thread, time::Duration};

use serde::Serialize;
use tauri_plugin_updater::UpdaterExt;
use url::Url;

use crate::codex::{
    export_fastslides_skill_archive, install_codex_mcp_server_config, CodexMcpInstallResponse,
};
use crate::runtime::open_in_file_manager as open_in_file_manager_service;

const DEFAULT_UPDATE_ENDPOINT: &str =
    "https://github.com/kristianernst/fast-slides/releases/latest/download/latest.json";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppUpdateStatus {
    configured: bool,
    current_version: String,
    available: bool,
    version: Option<String>,
    date: Option<String>,
    body: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppUpdateInstallResult {
    installed: bool,
    version: Option<String>,
    message: String,
}

fn update_endpoint() -> &'static str {
    option_env!("FASTSLIDES_UPDATE_ENDPOINT")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_UPDATE_ENDPOINT)
}

fn update_pubkey() -> Option<&'static str> {
    option_env!("FASTSLIDES_UPDATE_PUBKEY")
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn build_updater(app: &tauri::AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    let pubkey = update_pubkey()
        .ok_or_else(|| "App updates are not configured for this build.".to_string())?;
    let endpoint = update_endpoint()
        .parse::<Url>()
        .map_err(|error| format!("Invalid update endpoint: {error}"))?;

    app.updater_builder()
        .pubkey(pubkey)
        .endpoints(vec![endpoint])
        .map_err(|error| format!("Failed to configure updater: {error}"))?
        .build()
        .map_err(|error| format!("Failed to initialize updater: {error}"))
}

#[tauri::command]
pub(crate) fn open_in_file_manager(path: String) -> Result<(), String> {
    open_in_file_manager_service(path)
}

#[tauri::command]
pub(crate) async fn check_app_update(app: tauri::AppHandle) -> Result<AppUpdateStatus, String> {
    let current_version = app.package_info().version.to_string();

    if update_pubkey().is_none() {
        return Ok(AppUpdateStatus {
            configured: false,
            current_version,
            available: false,
            version: None,
            date: None,
            body: None,
        });
    }

    let update = build_updater(&app)?
        .check()
        .await
        .map_err(|error| format!("Failed to check for updates: {error}"))?;

    Ok(match update {
        Some(update) => AppUpdateStatus {
            configured: true,
            current_version,
            available: true,
            version: Some(update.version),
            date: update.date.map(|value| value.to_string()),
            body: update.body,
        },
        None => AppUpdateStatus {
            configured: true,
            current_version,
            available: false,
            version: None,
            date: None,
            body: None,
        },
    })
}

#[tauri::command]
pub(crate) async fn install_app_update(
    app: tauri::AppHandle,
) -> Result<AppUpdateInstallResult, String> {
    let updater = build_updater(&app)?;
    let Some(update) = updater
        .check()
        .await
        .map_err(|error| format!("Failed to check for updates: {error}"))?
    else {
        return Ok(AppUpdateInstallResult {
            installed: false,
            version: None,
            message: "FastSlides is already up to date.".to_string(),
        });
    };

    let version = update.version.clone();
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|error| format!("Failed to install FastSlides {version}: {error}"))?;

    let handle = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(300));
        handle.request_restart();
    });

    Ok(AppUpdateInstallResult {
        installed: true,
        version: Some(version.clone()),
        message: format!("Installed FastSlides {version}. Restarting now."),
    })
}

#[tauri::command]
pub(crate) fn export_fastslides_skill(destination: String) -> Result<String, String> {
    export_fastslides_skill_archive(destination)
}

#[tauri::command]
pub(crate) fn install_codex_mcp_server() -> Result<CodexMcpInstallResponse, String> {
    install_codex_mcp_server_config()
}
