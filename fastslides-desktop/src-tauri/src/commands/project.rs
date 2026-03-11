use crate::projects::{
    add_projects_root as add_projects_root_service, create_project as create_project_service,
    get_app_state as get_app_state_service, load_project as load_project_service,
    open_project as open_project_service, read_project_css as read_project_css_service,
    remove_project as remove_project_service, remove_projects_root as remove_projects_root_service,
    save_project as save_project_service, save_project_css as save_project_css_service,
    toggle_project_pin as toggle_project_pin_service, validate_project as validate_project_service,
    AppState, ProjectDetail, ValidationReport,
};

#[tauri::command]
pub(crate) fn get_app_state() -> Result<AppState, String> {
    get_app_state_service()
}

#[tauri::command]
pub(crate) fn open_project(path: String) -> Result<ProjectDetail, String> {
    open_project_service(path)
}

#[tauri::command]
pub(crate) fn add_projects_root(path: String) -> Result<AppState, String> {
    add_projects_root_service(path)
}

#[tauri::command]
pub(crate) fn remove_projects_root(path: String) -> Result<AppState, String> {
    remove_projects_root_service(path)
}

#[tauri::command]
pub(crate) fn remove_project(path: String) -> Result<AppState, String> {
    remove_project_service(path)
}

#[tauri::command]
pub(crate) fn load_project(path: String) -> Result<ProjectDetail, String> {
    load_project_service(path)
}

#[tauri::command]
pub(crate) fn save_project(path: String, page_mdx: String) -> Result<ProjectDetail, String> {
    save_project_service(path, page_mdx)
}

#[tauri::command]
pub(crate) fn create_project(
    root: String,
    name: String,
    title: Option<String>,
    subtitle: Option<String>,
    date_label: Option<String>,
) -> Result<ProjectDetail, String> {
    create_project_service(root, name, title, subtitle, date_label)
}

#[tauri::command]
pub(crate) fn toggle_project_pin(path: String) -> Result<AppState, String> {
    toggle_project_pin_service(path)
}

#[tauri::command]
pub(crate) fn validate_project(path: String) -> Result<ValidationReport, String> {
    validate_project_service(path)
}

#[tauri::command]
pub(crate) fn read_project_css(path: String) -> Result<String, String> {
    read_project_css_service(path)
}

#[tauri::command]
pub(crate) fn save_project_css(path: String, css: String) -> Result<(), String> {
    save_project_css_service(path, css)
}
