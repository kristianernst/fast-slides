use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

mod codex;
mod commands;
mod config;
mod constants;
mod deck;
mod design_system;
mod projects;
mod runtime;
mod scene;
#[cfg(test)]
mod tests;

use commands::{
    add_projects_root, analyze_project, capture_slide_image, compile_project_scene,
    compile_project_scene_manifest, compile_project_scene_slide, create_project,
    export_fastslides_skill, get_app_state, get_component_catalog, get_component_template,
    get_composition_template, get_design_system, get_recipe_template, install_codex_mcp_server,
    load_project, open_in_file_manager, open_project, read_project_css, remove_project,
    remove_projects_root, resolve_project_asset_data_url, save_component_template, save_project,
    save_project_css, start_project_scene_session, toggle_project_pin, validate_project,
};
use runtime::{
    build_app_menu, handle_menu_event, setup_mcp_tray_icon, start_agent_hook_server,
    start_mcp_server,
};

#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
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
        .on_menu_event(handle_menu_event)
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
