mod design;
mod project;
mod scene;
mod system;

pub(crate) use design::{
    get_component_catalog, get_component_template, get_composition_template, get_design_system,
    get_recipe_template, save_component_template,
};
pub(crate) use project::{
    add_projects_root, create_project, get_app_state, load_project, open_project, read_project_css,
    remove_project, remove_projects_root, save_project, save_project_css, toggle_project_pin,
    validate_project,
};
pub(crate) use scene::{
    analyze_project, capture_slide_image, compile_project_scene, compile_project_scene_manifest,
    compile_project_scene_slide, resolve_project_asset_data_url, start_project_scene_session,
};
pub(crate) use system::{export_fastslides_skill, install_codex_mcp_server, open_in_file_manager};
