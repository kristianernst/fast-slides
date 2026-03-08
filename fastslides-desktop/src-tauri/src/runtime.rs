mod capture;
mod hook;
mod mcp;
mod menu;

use std::env;

#[cfg(test)]
pub(crate) use capture::{
    build_slide_preview_url, build_subprocess_search_path, collect_png_artifacts,
    pick_slide_capture_artifact,
};
pub(crate) use capture::{capture_slide_image_for_project, SlideCaptureResponse};
pub(crate) use hook::start_agent_hook_server;
#[cfg(test)]
pub(crate) use mcp::build_mcp_server_config;
pub(crate) use mcp::{configured_mcp_server_url, start_mcp_server};
pub(crate) use menu::{
    build_app_menu, handle_menu_event, open_in_file_manager, setup_mcp_tray_icon,
};

fn env_string(keys: &[&str], default: &str) -> String {
    keys.iter()
        .find_map(|key| env::var(key).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn mcp_server_url(bind_addr: &str, route_path: &str) -> String {
    format!("http://{}{}", bind_addr, route_path)
}
