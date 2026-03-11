use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NamedCount {
    pub(crate) name: String,
    pub(crate) count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeckOutlineEntry {
    pub(crate) index: usize,
    pub(crate) title: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SlideAnalysis {
    pub(crate) index: usize,
    pub(crate) title: String,
    pub(crate) archetype: String,
    pub(crate) words: usize,
    pub(crate) bullets: usize,
    pub(crate) max_paragraph_words: usize,
    pub(crate) components: Vec<NamedCount>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectAnalysis {
    pub(crate) path: String,
    pub(crate) slide_count: usize,
    pub(crate) has_project_css: bool,
    pub(crate) outline: Vec<DeckOutlineEntry>,
    pub(crate) components: Vec<NamedCount>,
    pub(crate) archetypes: Vec<NamedCount>,
    pub(crate) warnings: Vec<String>,
    pub(crate) slides: Vec<SlideAnalysis>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectScene {
    pub(crate) path: String,
    pub(crate) project: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) subtitle: Option<String>,
    pub(crate) date: Option<String>,
    pub(crate) deck_class_name: Option<String>,
    pub(crate) slide_count: usize,
    pub(crate) slides: Vec<SceneSlide>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectSceneManifest {
    pub(crate) path: String,
    pub(crate) project: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) subtitle: Option<String>,
    pub(crate) date: Option<String>,
    pub(crate) deck_class_name: Option<String>,
    pub(crate) slide_count: usize,
    pub(crate) slides: Vec<SceneSlideManifest>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectSceneSessionHandle {
    pub(crate) session_id: String,
    pub(crate) path: String,
    pub(crate) slide_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectSceneSessionEvent {
    pub(crate) session_id: String,
    pub(crate) sequence: u64,
    #[serde(flatten)]
    pub(crate) payload: ProjectSceneSessionEventPayload,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum ProjectSceneSessionEventPayload {
    Manifest {
        scene: ProjectSceneManifest,
    },
    SlideReady {
        slide: SceneSlide,
    },
    SlideError {
        index: usize,
        error: String,
    },
    Complete {
        ready_count: usize,
        error_count: usize,
    },
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SceneSlide {
    pub(crate) index: usize,
    pub(crate) title: String,
    pub(crate) layout: SceneLayout,
    pub(crate) nodes: Vec<SceneNode>,
    pub(crate) source_mdx: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SceneSlideManifest {
    pub(crate) index: usize,
    pub(crate) title: String,
    pub(crate) layout: SceneLayout,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SceneLayout {
    pub(crate) kind: String,
    pub(crate) cols: Option<usize>,
    pub(crate) rows: Option<usize>,
    pub(crate) gap: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SceneChartDatum {
    pub(crate) label: String,
    pub(crate) value: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum SceneNode {
    Canvas {
        cols: usize,
        rows: usize,
        gap: Option<String>,
        class_name: Option<String>,
        children: Vec<SceneNode>,
        source_mdx: String,
    },
    Area {
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        layer: Option<usize>,
        gap: Option<String>,
        align: Option<String>,
        justify: Option<String>,
        class_name: Option<String>,
        children: Vec<SceneNode>,
        source_mdx: String,
    },
    LayoutGroup {
        component: String,
        cols: Option<usize>,
        gap: Option<String>,
        align: Option<String>,
        justify: Option<String>,
        nowrap: Option<bool>,
        class_name: Option<String>,
        children: Vec<SceneNode>,
        source_mdx: String,
    },
    Surface {
        component: String,
        tone: Option<String>,
        title: Option<String>,
        kicker: Option<String>,
        subtitle: Option<String>,
        foot: Option<String>,
        attribution: Option<String>,
        class_name: Option<String>,
        children: Vec<SceneNode>,
        source_mdx: String,
    },
    Metric {
        label: Option<String>,
        value: Option<String>,
        hint: Option<String>,
        class_name: Option<String>,
        source_mdx: String,
    },
    Chart {
        chart_type: String,
        title: Option<String>,
        tone: Option<String>,
        value_suffix: Option<String>,
        highlight: Option<String>,
        data: Vec<SceneChartDatum>,
        class_name: Option<String>,
        source_mdx: String,
    },
    Text {
        role: String,
        text: String,
        level: Option<u8>,
        class_name: Option<String>,
    },
    List {
        ordered: bool,
        items: Vec<String>,
    },
    Media {
        media_kind: String,
        src: String,
        alt: Option<String>,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Pill {
        tone: Option<String>,
        text: String,
        class_name: Option<String>,
    },
    Rule {
        class_name: Option<String>,
    },
    Arrow {
        direction: Option<String>,
        tone: Option<String>,
        label: Option<String>,
        class_name: Option<String>,
        source_mdx: String,
    },
    Raw {
        format: String,
        text: String,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectSceneSource {
    pub(crate) path: String,
    pub(crate) project: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) subtitle: Option<String>,
    pub(crate) date: Option<String>,
    pub(crate) deck_class_name: Option<String>,
    pub(crate) slides: Vec<String>,
}
