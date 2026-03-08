use crate::design_system::{
    build_component_catalog, component_template, composition_template, design_system_registry,
    recipe_template, save_component_template as persist_component_template, ComponentCatalog,
    DesignSystemRegistry, DesignTemplate, SaveComponentPayload, SaveComponentResponse,
};

#[tauri::command]
pub(crate) fn get_design_system() -> Result<DesignSystemRegistry, String> {
    Ok(design_system_registry())
}

#[tauri::command]
pub(crate) fn get_component_catalog() -> Result<ComponentCatalog, String> {
    build_component_catalog()
}

#[tauri::command]
pub(crate) fn get_component_template(name: String) -> Result<DesignTemplate, String> {
    component_template(&name)
}

#[tauri::command]
pub(crate) fn save_component_template(
    payload: SaveComponentPayload,
) -> Result<SaveComponentResponse, String> {
    persist_component_template(payload)
}

#[tauri::command]
pub(crate) fn get_composition_template(name: String) -> Result<DesignTemplate, String> {
    composition_template(&name)
}

#[tauri::command]
pub(crate) fn get_recipe_template(name: String) -> Result<DesignTemplate, String> {
    recipe_template(&name)
}
