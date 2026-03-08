use crate::codex::{upsert_codex_mcp_server_block, CodexInstallStatus};
use crate::deck::{component_counts_for_slide, extract_slides};
use crate::design_system::{
    build_component_catalog, composition_template, design_system_registry, known_composition_names,
    known_primitive_names, known_recipe_names, load_saved_components_from, primitive_template,
    recipe_template, save_component_to_path, SaveComponentPayload,
};
use crate::runtime::{
    build_mcp_server_config, build_slide_preview_url, build_subprocess_search_path,
    collect_png_artifacts, pick_slide_capture_artifact,
};
use crate::scene::{
    collect_project_scene_session_events, compile_slide_nodes, prioritize_scene_slide_indices,
    slide_contract_warnings, validate_scene_slide_contract, ProjectSceneSessionEventPayload,
    ProjectSceneSource, SceneNode,
};
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio_util::sync::CancellationToken;
use url::Url;

fn sample_scene_source() -> ProjectSceneSource {
    ProjectSceneSource {
        path: "/tmp/sample".to_string(),
        project: Some("sample".to_string()),
        title: Some("Sample".to_string()),
        subtitle: Some("Subtitle".to_string()),
        date: Some("March 2026".to_string()),
        deck_class_name: Some("deck sample".to_string()),
        slides: vec![
            "<Canvas cols={50} rows={25}><Area x={1} y={1} w={10} h={5}><h1>Slide One</h1></Area></Canvas>".to_string(),
            "<Canvas cols={50} rows={25}><Area x={1} y={1} w={10} h={5}><h1>Slide Two</h1></Area></Canvas>".to_string(),
            "<Canvas cols={50} rows={25}><Area x={1} y={1} w={10} h={5}><h1>Slide Three</h1></Area></Canvas>".to_string(),
        ],
    }
}

#[test]
fn prioritize_scene_slide_indices_biases_requested_slide_then_neighbors() {
    assert_eq!(
        prioritize_scene_slide_indices(7, 0),
        vec![0, 1, 2, 3, 4, 5, 6]
    );
    assert_eq!(prioritize_scene_slide_indices(5, 2), vec![2, 3, 1, 4, 0]);
    assert_eq!(prioritize_scene_slide_indices(4, 99), vec![3, 2, 1, 0]);
}

#[test]
fn collect_project_scene_session_events_emits_manifest_then_priority_slide_then_complete() {
    let events = collect_project_scene_session_events(sample_scene_source(), 2, 1)
        .expect("session events should compile");

    assert!(matches!(
        &events[0],
        ProjectSceneSessionEventPayload::Manifest { scene } if scene.slide_count == 3
    ));

    assert!(matches!(
        &events[1],
        ProjectSceneSessionEventPayload::SlideReady { slide } if slide.index == 2
    ));

    let ready_indices: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            ProjectSceneSessionEventPayload::SlideReady { slide } => Some(slide.index),
            _ => None,
        })
        .collect();
    assert_eq!(ready_indices, vec![2, 1, 0]);

    assert!(matches!(
        events.last().expect("complete event should exist"),
        ProjectSceneSessionEventPayload::Complete {
            ready_count: 3,
            error_count: 0
        }
    ));
}

#[test]
fn collect_project_scene_session_events_handles_empty_deck() {
    let mut source = sample_scene_source();
    source.slides.clear();

    let events = collect_project_scene_session_events(source, 0, 4)
        .expect("empty deck should still produce session events");

    assert_eq!(events.len(), 2);
    assert!(matches!(
        &events[0],
        ProjectSceneSessionEventPayload::Manifest { scene } if scene.slide_count == 0
    ));
    assert!(matches!(
        &events[1],
        ProjectSceneSessionEventPayload::Complete {
            ready_count: 0,
            error_count: 0
        }
    ));
}

#[test]
fn slide_contract_warnings_flag_dense_takeaway_metric_grid_and_callout() {
    let slide = r#"
<Canvas cols={50} rows={25} gap="1px">
  <Area x={2} y={4} w={30} h={4}>
    <Takeaway>The same deck should exercise primitive layouts, metrics, and captions without dropping back to legacy HTML patterns.</Takeaway>
  </Area>
  <Area x={2} y={10} w={30} h={10}>
    <Grid cols={3} gap="md">
      <Metric label="MDX mode" value="Runtime" hint="Content-only decks" />
      <Metric label="Layout API" value="Canvas-first" hint="Area plus bounded primitives" />
      <Metric label="Visual QA" value="Screenshots" hint="Playwright-powered" />
    </Grid>
  </Area>
  <Area x={34} y={10} w={14} h={10}>
    <Callout title="Check">
      Metrics, captions, and internal grids should align cleanly inside one region without growing random borders or side accents.
    </Callout>
  </Area>
</Canvas>
"#;

    let warnings = slide_contract_warnings(slide);

    assert!(warnings
        .iter()
        .any(|warning| warning.contains("Takeaway is too dense")));
    assert!(warnings
        .iter()
        .any(|warning| warning.contains("metric grid is too tight")));
    assert!(warnings
        .iter()
        .any(|warning| warning.contains("Callout copy is too dense")));
}

#[test]
fn slide_contract_warnings_allow_spacious_scorecard_layout() {
    let slide = r#"
<Canvas cols={50} rows={25} gap="1px">
  <Area x={2} y={4} w={30} h={6}>
    <Takeaway>Stable primitives make complex decks easier to generate and review.</Takeaway>
  </Area>
  <Area x={2} y={11} w={33} h={6}>
    <Grid cols={3} gap="sm">
      <Metric label="Grid" value="50x25" hint="Base canvas" />
      <Metric label="QA" value="Live" hint="Preview first" />
      <Metric label="Export" value="Shared" hint="One scene model" />
    </Grid>
  </Area>
  <Area x={36} y={10} w={12} h={9}>
    <Callout title="Check">Short commentary sits cleanly in the rail.</Callout>
  </Area>
</Canvas>
"#;

    assert!(slide_contract_warnings(slide).is_empty());
}

#[test]
fn design_system_registry_uses_thin_chrome_and_small_variant_sets() {
    let registry = design_system_registry();

    assert_eq!(registry.default_frame.cols, 50);
    assert_eq!(registry.default_frame.rows, 25);
    assert!(registry.default_frame.header_rows <= 2);
    assert!(registry.default_frame.footer_rows <= 2);
    assert!(registry.default_frame.body_rows > registry.default_frame.header_rows);
    assert!(registry.default_frame.body_rows > registry.default_frame.footer_rows);

    assert_eq!(registry.compositions.len(), 5);
    assert_eq!(registry.recipes.len(), 5);
    assert_eq!(registry.sections.len(), 3);

    assert!(registry
        .primitives
        .iter()
        .all(|primitive| primitive.variants.len() <= 3));
    assert!(registry
        .compositions
        .iter()
        .all(|composition| composition.variants.len() <= 3));
}

#[test]
fn subprocess_search_path_includes_common_npx_locations() {
    let path = build_subprocess_search_path(Some(std::ffi::OsStr::new("/usr/bin:/bin")));
    let entries: Vec<_> = env::split_paths(&path).collect();

    assert!(entries.contains(&PathBuf::from("/opt/homebrew/bin")));
    assert!(entries.contains(&PathBuf::from("/usr/local/bin")));
}

#[test]
fn design_system_registry_sections_reference_known_recipes() {
    let registry = design_system_registry();
    let recipe_names: HashSet<_> = registry
        .recipes
        .iter()
        .map(|recipe| recipe.name.clone())
        .collect();

    assert!(registry.sections.iter().all(|section| section
        .recipes
        .iter()
        .all(|recipe| recipe_names.contains(recipe))));
}

#[test]
fn known_template_names_match_design_system_registry() {
    let registry = design_system_registry();
    let primitive_names: Vec<_> = registry
        .primitives
        .iter()
        .map(|primitive| primitive.name.clone())
        .collect();
    let composition_names: Vec<_> = registry
        .compositions
        .iter()
        .map(|composition| composition.name.clone())
        .collect();
    let recipe_names: Vec<_> = registry
        .recipes
        .iter()
        .map(|recipe| recipe.name.clone())
        .collect();

    assert_eq!(primitive_names, known_primitive_names());
    assert_eq!(composition_names, known_composition_names());
    assert_eq!(recipe_names, known_recipe_names());
}

#[test]
fn registered_primitive_templates_are_spatial_and_warning_clean() {
    for name in known_primitive_names() {
        let template = primitive_template(&name)
            .unwrap_or_else(|error| panic!("primitive template `{name}` should resolve: {error}"));
        let slide = format!(
            "<section className=\"slide\">\n  <Canvas cols={{50}} rows={{25}} gap=\"1px\">\n{}\n  </Canvas>\n</section>",
            template.mdx
        );
        let slides = extract_slides(&slide);
        let warnings = slide_contract_warnings(&slides[0]);

        assert_eq!(
            slides.len(),
            1,
            "primitive `{name}` should produce one wrapped slide"
        );
        validate_scene_slide_contract(&slides[0], 0).unwrap_or_else(|error| {
            panic!("primitive `{name}` should satisfy spatial contract: {error}")
        });
        assert!(
            warnings.is_empty(),
            "primitive `{name}` should not trigger base contract warnings: {warnings:?}"
        );
    }
}

#[test]
fn registered_composition_templates_are_spatial_and_warning_clean() {
    for name in known_composition_names() {
        let template = composition_template(&name).unwrap_or_else(|error| {
            panic!("composition template `{name}` should resolve: {error}")
        });
        let slide = format!(
            "<section className=\"slide\">\n  <Canvas cols={{50}} rows={{25}} gap=\"1px\">\n{}\n  </Canvas>\n</section>",
            template.mdx
        );
        let slides = extract_slides(&slide);
        let warnings = slide_contract_warnings(&slides[0]);

        assert_eq!(
            slides.len(),
            1,
            "composition `{name}` should produce one wrapped slide"
        );
        validate_scene_slide_contract(&slides[0], 0).unwrap_or_else(|error| {
            panic!("composition `{name}` should satisfy spatial contract: {error}")
        });
        assert!(
            warnings.is_empty(),
            "composition `{name}` should not trigger base contract warnings: {warnings:?}"
        );
    }
}

#[test]
fn registered_recipe_templates_are_spatial_and_warning_clean() {
    for name in known_recipe_names() {
        let template = recipe_template(&name)
            .unwrap_or_else(|error| panic!("recipe template `{name}` should resolve: {error}"));
        let slides = extract_slides(&template.mdx);
        let warnings = slide_contract_warnings(&slides[0]);

        assert_eq!(slides.len(), 1, "recipe `{name}` should contain one slide");
        validate_scene_slide_contract(&slides[0], 0).unwrap_or_else(|error| {
            panic!("recipe `{name}` should satisfy spatial contract: {error}")
        });
        assert!(
            warnings.is_empty(),
            "recipe `{name}` should not trigger base contract warnings: {warnings:?}"
        );
    }
}

#[test]
fn chart_component_is_counted_compiled_and_warning_clean() {
    let slide = r#"
<section className="slide">
  <Canvas cols={50} rows={25} gap="1px">
    <Area x={2} y={2} w={14} h={1}>
      <Kicker>Evidence</Kicker>
    </Area>
    <Area x={2} y={4} w={46} h={4}>
      <Takeaway>Charts should render through the shared scene model, not raw HTML.</Takeaway>
    </Area>
    <Area x={2} y={10} w={30} h={11}>
      <Chart
        type="bar"
        title="Priority"
        data="Workflow:88;Distribution:76;Feedback:69"
        suffix="%"
        highlight="Workflow"
      />
    </Area>
    <Area x={34} y={10} w={14} h={11}>
      <Callout title="Read-through">One focused chart beats a hand-built box garden.</Callout>
    </Area>
  </Canvas>
</section>
"#;

    let counts = component_counts_for_slide(slide);
    assert_eq!(counts.get("Chart").copied(), Some(1));
    assert!(slide_contract_warnings(slide).is_empty());

    let compiled = compile_slide_nodes(slide);
    assert!(compiled.iter().any(|node| matches!(
        node,
        SceneNode::Canvas { children, .. }
            if children.iter().any(|child| matches!(
                child,
                SceneNode::Area { children, .. }
                    if children.iter().any(|grandchild| matches!(grandchild, SceneNode::Chart { .. }))
            ))
    )));
}

#[test]
fn component_catalog_includes_builtin_patterns_and_marks() {
    let catalog = build_component_catalog().expect("component catalog should build");
    let names: HashSet<_> = catalog
        .items
        .iter()
        .map(|item| item.name.as_str())
        .collect();
    assert!(names.contains("Arrow"));
    assert!(names.contains("ImageFigure"));
    assert!(names.contains("TrendChartCommentary"));
}

#[test]
fn save_component_to_path_round_trips_custom_entry() {
    let library_path = env::temp_dir().join(format!(
        "fastslides-component-library-{}.json",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    if library_path.exists() {
        let _ = fs::remove_file(&library_path);
    }

    let payload = SaveComponentPayload {
        name: "SavedCallout".to_string(),
        family: "narrative".to_string(),
        summary: "Saved custom callout".to_string(),
        tags: Some(vec!["saved".to_string(), "callout".to_string()]),
        mdx: "<Area x={34} y={10} w={14} h={8}><Callout title=\"Saved\">Reusable note</Callout></Area>".to_string(),
        notes: Some(vec!["Captured from a good slide.".to_string()]),
    };

    let saved =
        save_component_to_path(&library_path, payload).expect("saving component should succeed");
    let records =
        load_saved_components_from(&library_path).expect("saved component library should load");

    assert_eq!(saved.component.name, "SavedCallout");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].family, "narrative");
    assert_eq!(records[0].summary, "Saved custom callout");

    let _ = fs::remove_file(&library_path);
}

#[test]
fn codex_mcp_install_inserts_fastslides_block_when_missing() {
    let source = r#"model = "gpt-5.4"

[mcp_servers.deepwiki]
url = "https://mcp.deepwiki.com/mcp"
"#;

    let (updated, status) =
        upsert_codex_mcp_server_block(source, "fastslides", "http://127.0.0.1:38474/mcp")
            .expect("upsert should succeed");

    assert_eq!(status, CodexInstallStatus::Installed);
    assert!(updated.contains("[mcp_servers.fastslides]"));
    assert!(updated.contains("url = \"http://127.0.0.1:38474/mcp\""));
    assert!(updated.contains("[mcp_servers.deepwiki]"));
}

#[test]
fn codex_mcp_install_updates_existing_fastslides_block() {
    let source = r#"model = "gpt-5.4"

[mcp_servers.fastslides]
command = "old-fastslides"

[mcp_servers.linear]
url = "https://mcp.linear.app/mcp"
"#;

    let (updated, status) =
        upsert_codex_mcp_server_block(source, "fastslides", "http://127.0.0.1:38474/mcp")
            .expect("upsert should succeed");

    assert_eq!(status, CodexInstallStatus::Updated);
    assert!(updated.contains("[mcp_servers.fastslides]"));
    assert!(updated.contains("url = \"http://127.0.0.1:38474/mcp\""));
    assert!(!updated.contains("old-fastslides"));
    assert!(updated.contains("[mcp_servers.linear]"));
}

#[test]
fn codex_mcp_install_is_idempotent_when_fastslides_block_matches() {
    let source = r#"model = "gpt-5.4"

[mcp_servers.fastslides]
url = "http://127.0.0.1:38474/mcp"
"#;

    let (updated, status) =
        upsert_codex_mcp_server_block(source, "fastslides", "http://127.0.0.1:38474/mcp")
            .expect("upsert should succeed");

    assert_eq!(status, CodexInstallStatus::Unchanged);
    assert_eq!(updated.trim(), source.trim());
}

#[test]
fn build_slide_preview_url_adds_slide_and_presenter_params() {
    let preview_url = build_slide_preview_url("http://127.0.0.1:1420/?deckPath=%2Ftmp%2Fdeck", 5)
        .expect("slide preview URL should build");
    let parsed = Url::parse(&preview_url).expect("url should parse");
    let params: HashMap<_, _> = parsed.query_pairs().into_owned().collect();

    assert_eq!(params.get("deckPath"), Some(&"/tmp/deck".to_string()));
    assert_eq!(params.get("slide"), Some(&"5".to_string()));
    assert_eq!(params.get("presenter"), Some(&"1".to_string()));
}

#[test]
fn mcp_server_uses_stateless_streamable_http_mode() {
    let config = build_mcp_server_config(CancellationToken::new());
    assert!(!config.stateful_mode);
}

#[test]
fn pick_slide_capture_artifact_prefers_new_pngs_then_newest_file() {
    let before = HashSet::from([PathBuf::from("/tmp/older.png")]);
    let after = vec![
        (
            PathBuf::from("/tmp/older.png"),
            UNIX_EPOCH + Duration::from_secs(1),
        ),
        (
            PathBuf::from("/tmp/newer.png"),
            UNIX_EPOCH + Duration::from_secs(2),
        ),
        (
            PathBuf::from("/tmp/newest.png"),
            UNIX_EPOCH + Duration::from_secs(3),
        ),
    ];

    assert_eq!(
        pick_slide_capture_artifact(&before, &after),
        Some(PathBuf::from("/tmp/newest.png"))
    );

    let before_all = HashSet::from([
        PathBuf::from("/tmp/older.png"),
        PathBuf::from("/tmp/newer.png"),
        PathBuf::from("/tmp/newest.png"),
    ]);

    assert_eq!(
        pick_slide_capture_artifact(&before_all, &after),
        Some(PathBuf::from("/tmp/newest.png"))
    );
}

#[test]
fn collect_png_artifacts_walks_nested_directories() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should advance")
        .as_nanos();
    let root = env::temp_dir().join(format!("fastslides-capture-test-{unique}"));
    let nested = root.join("nested");
    fs::create_dir_all(&nested).expect("nested capture dir should create");
    let png_path = nested.join("slide.png");
    fs::write(&png_path, b"png").expect("png fixture should write");

    let artifacts = collect_png_artifacts(&root).expect("artifact scan should succeed");
    let artifact_paths: HashSet<_> = artifacts.into_iter().map(|(path, _)| path).collect();
    assert!(artifact_paths.contains(&png_path));

    fs::remove_dir_all(&root).expect("temp capture dir should remove");
}
