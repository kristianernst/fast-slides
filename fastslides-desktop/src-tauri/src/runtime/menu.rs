use super::mcp::McpServerStatus;
use crate::codex::install_codex_mcp_server_config;
use crate::config::normalize_existing_directory;
use crate::constants::{
    MCP_TRAY_ICON_ID, MENU_EXPORT_SKILL_EVENT, MENU_EXPORT_SKILL_ID, MENU_INSTALL_CODEX_MCP_ID,
    MENU_OPEN_MAIN_WINDOW_ID, MENU_OPEN_SETTINGS_EVENT, MENU_OPEN_SETTINGS_ID,
};
use std::process::Command;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, Runtime,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

#[cfg(target_os = "macos")]
const TRAY_TEMPLATE_ICON: tauri::image::Image<'_> =
    tauri::include_image!("./icons/trayTemplate@2x.png");

pub(crate) fn build_app_menu<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
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

pub(crate) fn setup_mcp_tray_icon<R: Runtime>(
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

    #[cfg(target_os = "macos")]
    {
        tray_builder = tray_builder.icon(TRAY_TEMPLATE_ICON).icon_as_template(true);
    }

    #[cfg(not(target_os = "macos"))]
    if let Some(icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(icon);
    }

    tray_builder
        .build(app)
        .map_err(|error| format!("Failed to create tray icon: {error}"))?;
    Ok(())
}

pub(crate) fn open_in_file_manager(path: String) -> Result<(), String> {
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

pub(crate) fn handle_menu_event<R: Runtime>(
    app: &tauri::AppHandle<R>,
    event: tauri::menu::MenuEvent,
) {
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
}
