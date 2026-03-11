use crate::codex::{
    export_fastslides_skill_archive, install_codex_mcp_server_config, CodexMcpInstallResponse,
};
use crate::runtime::open_in_file_manager as open_in_file_manager_service;

#[tauri::command]
pub(crate) fn open_in_file_manager(path: String) -> Result<(), String> {
    open_in_file_manager_service(path)
}

#[tauri::command]
pub(crate) fn export_fastslides_skill(destination: String) -> Result<String, String> {
    export_fastslides_skill_archive(destination)
}

#[tauri::command]
pub(crate) fn install_codex_mcp_server() -> Result<CodexMcpInstallResponse, String> {
    install_codex_mcp_server_config()
}
