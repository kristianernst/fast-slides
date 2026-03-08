mod analysis;
mod compile;
mod types;

pub(crate) use analysis::{
    build_project_analysis, emit_project_scene_session_events, next_scene_session_id,
    preferred_scene_session_worker_count, slide_contract_warnings,
};
#[cfg(test)]
pub(crate) use analysis::{collect_project_scene_session_events, prioritize_scene_slide_indices};
pub(crate) use compile::{
    build_project_scene, build_project_scene_manifest, build_project_scene_slide,
    load_project_scene_source,
};
#[cfg(test)]
pub(crate) use compile::{compile_slide_nodes, validate_scene_slide_contract};
pub(crate) use types::{
    ProjectAnalysis, ProjectScene, ProjectSceneManifest, ProjectSceneSessionEvent,
    ProjectSceneSessionEventPayload, ProjectSceneSessionHandle, SceneSlide,
};
#[cfg(test)]
pub(crate) use types::{ProjectSceneSource, SceneNode};
