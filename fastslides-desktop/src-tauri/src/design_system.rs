use crate::config::{expand_user_path, path_to_string};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
pub(crate) struct SaveComponentPayload {
    pub(crate) name: String,
    pub(crate) family: String,
    pub(crate) summary: String,
    pub(crate) tags: Option<Vec<String>>,
    pub(crate) mdx: String,
    pub(crate) notes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SaveComponentResponse {
    pub(crate) ok: bool,
    pub(crate) component: ComponentCatalogEntry,
    pub(crate) library_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DesignSystemRegistry {
    pub(crate) version: String,
    pub(crate) philosophy: Vec<String>,
    pub(crate) default_frame: DesignFrameSpec,
    pub(crate) primitives: Vec<PrimitiveSpec>,
    pub(crate) compositions: Vec<CompositionSpec>,
    pub(crate) recipes: Vec<RecipeSpec>,
    pub(crate) sections: Vec<SectionSpec>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DesignFrameSpec {
    pub(crate) cols: usize,
    pub(crate) rows: usize,
    pub(crate) header_rows: usize,
    pub(crate) body_rows: usize,
    pub(crate) footer_rows: usize,
    pub(crate) body_slices: Vec<BodySliceSpec>,
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BodySliceSpec {
    pub(crate) name: String,
    pub(crate) min_rows: usize,
    pub(crate) preferred_rows: usize,
    pub(crate) purpose: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AreaSizeSpec {
    pub(crate) cols: usize,
    pub(crate) rows: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SlotSpec {
    pub(crate) name: String,
    pub(crate) accepts: Vec<String>,
    pub(crate) min: usize,
    pub(crate) max: usize,
    pub(crate) note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PrimitiveSpec {
    pub(crate) name: String,
    pub(crate) purpose: String,
    pub(crate) variants: Vec<String>,
    pub(crate) min_area: Option<AreaSizeSpec>,
    pub(crate) preferred_area: Option<AreaSizeSpec>,
    pub(crate) allowed_parents: Vec<String>,
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CompositionSpec {
    pub(crate) name: String,
    pub(crate) purpose: String,
    pub(crate) variants: Vec<String>,
    pub(crate) min_area: AreaSizeSpec,
    pub(crate) preferred_area: AreaSizeSpec,
    pub(crate) source_primitives: Vec<String>,
    pub(crate) slots: Vec<SlotSpec>,
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RecipeSpec {
    pub(crate) name: String,
    pub(crate) summary: String,
    pub(crate) frame: DesignFrameSpec,
    pub(crate) compositions: Vec<String>,
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SectionSpec {
    pub(crate) name: String,
    pub(crate) summary: String,
    pub(crate) recipes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DesignTemplate {
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) mdx: String,
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComponentCatalog {
    pub(crate) version: String,
    pub(crate) items: Vec<ComponentCatalogEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComponentCatalogEntry {
    pub(crate) name: String,
    pub(crate) family: String,
    pub(crate) kind: String,
    pub(crate) scope: String,
    pub(crate) summary: String,
    pub(crate) tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SavedComponentRecord {
    pub(crate) name: String,
    pub(crate) family: String,
    pub(crate) summary: String,
    pub(crate) tags: Vec<String>,
    pub(crate) mdx: String,
    pub(crate) notes: Vec<String>,
}

fn size(cols: usize, rows: usize) -> AreaSizeSpec {
    AreaSizeSpec { cols, rows }
}

fn named_spec_list<T>(items: Vec<T>, name_of: impl Fn(&T) -> &str) -> Vec<String> {
    items
        .into_iter()
        .map(|item| name_of(&item).to_string())
        .collect()
}

fn default_design_frame() -> DesignFrameSpec {
    DesignFrameSpec {
        cols: 50,
        rows: 25,
        header_rows: 1,
        body_rows: 23,
        footer_rows: 1,
        body_slices: vec![
            BodySliceSpec {
                name: "hero".to_string(),
                min_rows: 3,
                preferred_rows: 3,
                purpose: "Primary conclusion or title band.".to_string(),
            },
            BodySliceSpec {
                name: "primary".to_string(),
                min_rows: 9,
                preferred_rows: 10,
                purpose: "Main exhibit or first composition row.".to_string(),
            },
            BodySliceSpec {
                name: "secondary".to_string(),
                min_rows: 6,
                preferred_rows: 7,
                purpose: "Supporting comparison, KPIs, or commentary.".to_string(),
            },
            BodySliceSpec {
                name: "note".to_string(),
                min_rows: 1,
                preferred_rows: 2,
                purpose: "Caption, source note, or operator footer within the body band."
                    .to_string(),
            },
        ],
        notes: vec![
            "Keep chrome thin. Header and footer should frame the page, not consume it."
                .to_string(),
            "Start the main exhibit high. Leave at most one quiet transition row below the takeaway."
                .to_string(),
            "Treat the body as the design surface. Compose it with 2 to 4 deliberate modules."
                .to_string(),
        ],
    }
}

pub(crate) fn design_system_registry() -> DesignSystemRegistry {
    let default_frame = default_design_frame();

    let primitives = vec![
        PrimitiveSpec {
            name: "Kicker".to_string(),
            purpose: "Thin meta label that names the story without competing with the main point."
                .to_string(),
            variants: vec!["default".to_string()],
            min_area: Some(size(10, 1)),
            preferred_area: Some(size(14, 1)),
            allowed_parents: vec!["Area".to_string(), "Panel".to_string()],
            notes: vec!["Use as top-line chrome only.".to_string()],
        },
        PrimitiveSpec {
            name: "Takeaway".to_string(),
            purpose: "Main conclusion band. This is the strongest sentence on the slide."
                .to_string(),
            variants: vec![
                "compact".to_string(),
                "balanced".to_string(),
                "hero".to_string(),
            ],
            min_area: Some(size(36, 4)),
            preferred_area: Some(size(44, 4)),
            allowed_parents: vec!["Area".to_string()],
            notes: vec![
                "Prefer one sentence.".to_string(),
                "Use the full hero band before shrinking the title into a narrow left column."
                    .to_string(),
            ],
        },
        PrimitiveSpec {
            name: "Panel".to_string(),
            purpose: "Structured evidence block for lists, code, tables, or mixed narrative."
                .to_string(),
            variants: vec![
                "compact".to_string(),
                "standard".to_string(),
                "roomy".to_string(),
            ],
            min_area: Some(size(14, 9)),
            preferred_area: Some(size(18, 11)),
            allowed_parents: vec!["Area".to_string(), "Grid".to_string()],
            notes: vec!["Use this as the default evidence container.".to_string()],
        },
        PrimitiveSpec {
            name: "Metric".to_string(),
            purpose: "Compact KPI tile for one value, one label, and a small hint.".to_string(),
            variants: vec![
                "compact".to_string(),
                "standard".to_string(),
                "feature".to_string(),
            ],
            min_area: Some(size(10, 5)),
            preferred_area: Some(size(11, 5)),
            allowed_parents: vec!["Area".to_string(), "Grid".to_string()],
            notes: vec![
                "Do not use long narrative copy inside a metric.".to_string(),
                "Three-up scorecards need around 30 columns of total width.".to_string(),
            ],
        },
        PrimitiveSpec {
            name: "Chart".to_string(),
            purpose: "Single-series analytical chart that renders in the slide theme without raw SVG handwork."
                .to_string(),
            variants: vec!["bar".to_string(), "trend".to_string()],
            min_area: Some(size(18, 8)),
            preferred_area: Some(size(30, 11)),
            allowed_parents: vec!["Area".to_string(), "Panel".to_string()],
            notes: vec![
                "Keep charts focused: one series, 3 to 8 points.".to_string(),
                "Use a panel only when the chart also needs a narrative title or footnote."
                    .to_string(),
            ],
        },
        PrimitiveSpec {
            name: "Rule".to_string(),
            purpose: "Thin divider or anchor line that adds structure without another box."
                .to_string(),
            variants: vec!["default".to_string()],
            min_area: Some(size(8, 1)),
            preferred_area: Some(size(18, 1)),
            allowed_parents: vec!["Area".to_string()],
            notes: vec!["Use for rhythm, grouping, or section breaks.".to_string()],
        },
        PrimitiveSpec {
            name: "Arrow".to_string(),
            purpose: "Directional mark that connects two regions or indicates flow."
                .to_string(),
            variants: vec![
                "right".to_string(),
                "left".to_string(),
                "down".to_string(),
            ],
            min_area: Some(size(8, 2)),
            preferred_area: Some(size(14, 2)),
            allowed_parents: vec!["Area".to_string()],
            notes: vec![
                "Prefer one arrow over many.".to_string(),
                "Use to connect modules, not decorate them.".to_string(),
            ],
        },
        PrimitiveSpec {
            name: "Callout".to_string(),
            purpose: "Interpretive rail that explains why the evidence matters.".to_string(),
            variants: vec![
                "compact".to_string(),
                "rail".to_string(),
                "standard".to_string(),
            ],
            min_area: Some(size(13, 9)),
            preferred_area: Some(size(14, 10)),
            allowed_parents: vec!["Area".to_string()],
            notes: vec![
                "Keep it short enough to read in one glance.".to_string(),
                "If it grows, widen the rail or turn it into a panel.".to_string(),
            ],
        },
        PrimitiveSpec {
            name: "Quote".to_string(),
            purpose: "Short proof point with attribution when a voice should carry the slide."
                .to_string(),
            variants: vec!["standard".to_string(), "evidence".to_string()],
            min_area: Some(size(16, 8)),
            preferred_area: Some(size(20, 9)),
            allowed_parents: vec!["Area".to_string()],
            notes: vec![
                "Prefer one sharp quote over several fragments.".to_string(),
                "Pair it with explicit evidence or a decision when possible.".to_string(),
            ],
        },
        PrimitiveSpec {
            name: "Caption".to_string(),
            purpose: "Thin note or source line that should live near the bottom of the body band."
                .to_string(),
            variants: vec!["default".to_string()],
            min_area: Some(size(12, 1)),
            preferred_area: Some(size(18, 1)),
            allowed_parents: vec!["Area".to_string()],
            notes: vec!["Footer copy should be quiet and low-height.".to_string()],
        },
    ];

    let compositions = vec![
        CompositionSpec {
            name: "TakeawayRail".to_string(),
            purpose: "One takeaway band with one interpretation rail.".to_string(),
            variants: vec!["standard".to_string(), "executive".to_string()],
            min_area: size(50, 16),
            preferred_area: size(50, 19),
            source_primitives: vec![
                "Kicker".to_string(),
                "Takeaway".to_string(),
                "Callout".to_string(),
                "Caption".to_string(),
            ],
            slots: vec![
                SlotSpec {
                    name: "takeaway".to_string(),
                    accepts: vec!["Takeaway".to_string()],
                    min: 1,
                    max: 1,
                    note: Some("Primary conclusion band.".to_string()),
                },
                SlotSpec {
                    name: "rail".to_string(),
                    accepts: vec!["Callout".to_string()],
                    min: 1,
                    max: 1,
                    note: Some("Interpretive commentary.".to_string()),
                },
            ],
            notes: vec!["Default opener pattern.".to_string()],
        },
        CompositionSpec {
            name: "MetricStrip".to_string(),
            purpose: "Two to four metrics in a controlled scorecard row.".to_string(),
            variants: vec![
                "compact".to_string(),
                "standard".to_string(),
                "executive".to_string(),
            ],
            min_area: size(24, 5),
            preferred_area: size(30, 6),
            source_primitives: vec!["Metric".to_string(), "Caption".to_string()],
            slots: vec![SlotSpec {
                name: "items".to_string(),
                accepts: vec!["Metric".to_string()],
                min: 2,
                max: 4,
                note: Some("Fewer metrics are stronger than many tiny ones.".to_string()),
            }],
            notes: vec!["Default scorecard module.".to_string()],
        },
        CompositionSpec {
            name: "ExhibitCommentary".to_string(),
            purpose: "One evidence panel paired with one commentary rail.".to_string(),
            variants: vec!["standard".to_string(), "chart".to_string()],
            min_area: size(50, 14),
            preferred_area: size(50, 17),
            source_primitives: vec![
                "Panel".to_string(),
                "Chart".to_string(),
                "Callout".to_string(),
                "Caption".to_string(),
            ],
            slots: vec![
                SlotSpec {
                    name: "exhibit".to_string(),
                    accepts: vec!["Panel".to_string(), "Chart".to_string()],
                    min: 1,
                    max: 1,
                    note: Some("Table, code, chart, or structured evidence.".to_string()),
                },
                SlotSpec {
                    name: "commentary".to_string(),
                    accepts: vec!["Callout".to_string()],
                    min: 1,
                    max: 1,
                    note: Some("Short interpretation.".to_string()),
                },
            ],
            notes: vec!["Best for one main exhibit per slide.".to_string()],
        },
        CompositionSpec {
            name: "ThreeUpPanels".to_string(),
            purpose: "Three parallel panels for compare, problem/move/outcome, or options."
                .to_string(),
            variants: vec!["standard".to_string(), "compare".to_string()],
            min_area: size(44, 10),
            preferred_area: size(46, 11),
            source_primitives: vec!["Panel".to_string()],
            slots: vec![SlotSpec {
                name: "panels".to_string(),
                accepts: vec!["Panel".to_string()],
                min: 3,
                max: 3,
                note: Some("Parallel structure only.".to_string()),
            }],
            notes: vec!["Use when symmetry is the point.".to_string()],
        },
        CompositionSpec {
            name: "KpiPair".to_string(),
            purpose: "Two stacked metrics beside a supporting note or exhibit.".to_string(),
            variants: vec!["stacked".to_string(), "rail".to_string()],
            min_area: size(20, 11),
            preferred_area: size(22, 11),
            source_primitives: vec![
                "Metric".to_string(),
                "Callout".to_string(),
                "Caption".to_string(),
            ],
            slots: vec![
                SlotSpec {
                    name: "metrics".to_string(),
                    accepts: vec!["Metric".to_string()],
                    min: 2,
                    max: 2,
                    note: Some("Two related KPIs only.".to_string()),
                },
                SlotSpec {
                    name: "note".to_string(),
                    accepts: vec!["Callout".to_string(), "Caption".to_string()],
                    min: 0,
                    max: 1,
                    note: Some("Optional interpretation or source line.".to_string()),
                },
            ],
            notes: vec!["Use for side modules, not the whole story.".to_string()],
        },
        CompositionSpec {
            name: "BeforeAfter".to_string(),
            purpose: "Two contrasted states with a clear directional change.".to_string(),
            variants: vec!["before-after".to_string(), "static-dynamic".to_string()],
            min_area: size(46, 11),
            preferred_area: size(46, 12),
            source_primitives: vec![
                "Panel".to_string(),
                "Arrow".to_string(),
                "Caption".to_string(),
            ],
            slots: vec![
                SlotSpec {
                    name: "before".to_string(),
                    accepts: vec!["Panel".to_string()],
                    min: 1,
                    max: 1,
                    note: Some("Current or baseline state.".to_string()),
                },
                SlotSpec {
                    name: "after".to_string(),
                    accepts: vec!["Panel".to_string()],
                    min: 1,
                    max: 1,
                    note: Some("Target state after the change.".to_string()),
                },
            ],
            notes: vec!["Use when contrast is the argument, not just the layout.".to_string()],
        },
        CompositionSpec {
            name: "OperatingModelRow".to_string(),
            purpose: "Three linked stages that explain how work should move.".to_string(),
            variants: vec!["three-stage".to_string(), "flow".to_string()],
            min_area: size(46, 10),
            preferred_area: size(46, 11),
            source_primitives: vec!["Panel".to_string(), "Arrow".to_string()],
            slots: vec![SlotSpec {
                name: "stages".to_string(),
                accepts: vec!["Panel".to_string()],
                min: 3,
                max: 3,
                note: Some("Keep each stage to one action and one support line.".to_string()),
            }],
            notes: vec!["Best for operating models, handoffs, or sequence bands.".to_string()],
        },
        CompositionSpec {
            name: "QuoteEvidence".to_string(),
            purpose: "One proof quote paired with one evidence block.".to_string(),
            variants: vec!["customer-proof".to_string(), "operator-proof".to_string()],
            min_area: size(46, 11),
            preferred_area: size(46, 12),
            source_primitives: vec![
                "Quote".to_string(),
                "Panel".to_string(),
                "Caption".to_string(),
            ],
            slots: vec![
                SlotSpec {
                    name: "quote".to_string(),
                    accepts: vec!["Quote".to_string()],
                    min: 1,
                    max: 1,
                    note: Some("One credible voice or proof point.".to_string()),
                },
                SlotSpec {
                    name: "evidence".to_string(),
                    accepts: vec!["Panel".to_string()],
                    min: 1,
                    max: 1,
                    note: Some("Structured evidence that supports the quote.".to_string()),
                },
            ],
            notes: vec!["Use when credibility matters more than volume.".to_string()],
        },
    ];

    let recipes = vec![
        RecipeSpec {
            name: "takeaway_plus_rail".to_string(),
            summary: "Default opening slide with one conclusion and one side interpretation."
                .to_string(),
            frame: default_frame.clone(),
            compositions: vec!["TakeawayRail".to_string()],
            notes: vec!["Thin header and footer; most rows belong to the body.".to_string()],
        },
        RecipeSpec {
            name: "scorecard_with_note".to_string(),
            summary: "One takeaway band, one metric strip, and one note rail.".to_string(),
            frame: default_frame.clone(),
            compositions: vec!["MetricStrip".to_string(), "TakeawayRail".to_string()],
            notes: vec!["Good for status and executive summaries.".to_string()],
        },
        RecipeSpec {
            name: "exhibit_left_commentary_right".to_string(),
            summary: "Primary evidence on the left with commentary on the right.".to_string(),
            frame: default_frame.clone(),
            compositions: vec!["ExhibitCommentary".to_string()],
            notes: vec!["The most reliable analytic-slide pattern.".to_string()],
        },
        RecipeSpec {
            name: "static_vs_dynamic_compare".to_string(),
            summary: "Contrast the baseline with the target state under one framing takeaway."
                .to_string(),
            frame: default_frame.clone(),
            compositions: vec!["BeforeAfter".to_string()],
            notes: vec![
                "Use when the audience needs a clean before/after decision frame.".to_string(),
            ],
        },
        RecipeSpec {
            name: "operating_model".to_string(),
            summary: "One takeaway with a linked three-stage operating model band.".to_string(),
            frame: default_frame.clone(),
            compositions: vec!["OperatingModelRow".to_string()],
            notes: vec!["Keep the row directional and the labels concrete.".to_string()],
        },
        RecipeSpec {
            name: "three_up_compare".to_string(),
            summary: "Three parallel panels under one takeaway.".to_string(),
            frame: default_frame.clone(),
            compositions: vec!["ThreeUpPanels".to_string()],
            notes: vec!["Use when the slide needs equal-weight comparison.".to_string()],
        },
        RecipeSpec {
            name: "kpi_pair_with_exhibit".to_string(),
            summary: "Two KPIs paired with a supporting panel or note.".to_string(),
            frame: default_frame.clone(),
            compositions: vec!["KpiPair".to_string(), "ExhibitCommentary".to_string()],
            notes: vec!["Useful for dashboard-like pages with one side story.".to_string()],
        },
        RecipeSpec {
            name: "quote_with_evidence".to_string(),
            summary: "One proof quote and one structured evidence block under a single takeaway."
                .to_string(),
            frame: default_frame.clone(),
            compositions: vec!["QuoteEvidence".to_string()],
            notes: vec!["Use when credibility or customer voice should lead the page.".to_string()],
        },
    ];

    let sections = vec![
        SectionSpec {
            name: "Situation -> Evidence -> Recommendation".to_string(),
            summary:
                "Classic consulting arc with one opener, one proof slide, and one action page."
                    .to_string(),
            recipes: vec![
                "takeaway_plus_rail".to_string(),
                "exhibit_left_commentary_right".to_string(),
                "scorecard_with_note".to_string(),
            ],
        },
        SectionSpec {
            name: "Problem -> Options -> Decision".to_string(),
            summary: "Use a compare slide between context and chosen path.".to_string(),
            recipes: vec![
                "takeaway_plus_rail".to_string(),
                "static_vs_dynamic_compare".to_string(),
                "scorecard_with_note".to_string(),
            ],
        },
        SectionSpec {
            name: "Dashboard -> Insight -> Operator Note".to_string(),
            summary: "KPI-heavy flow for product and operational reviews.".to_string(),
            recipes: vec![
                "scorecard_with_note".to_string(),
                "kpi_pair_with_exhibit".to_string(),
                "takeaway_plus_rail".to_string(),
            ],
        },
        SectionSpec {
            name: "Current -> Future -> Operating model".to_string(),
            summary: "Use contrast first, then show the target operating flow and proof."
                .to_string(),
            recipes: vec![
                "static_vs_dynamic_compare".to_string(),
                "operating_model".to_string(),
                "quote_with_evidence".to_string(),
            ],
        },
    ];

    DesignSystemRegistry {
        version: "2.0-base".to_string(),
        philosophy: vec![
            "Use few expressive modules instead of many decorative variants.".to_string(),
            "Keep header and footer thin so the body owns the page.".to_string(),
            "Compose slides from modules first; use raw areas only as an escape hatch.".to_string(),
            "Prefer native layout primitives and real images before Mermaid or standalone SVG diagrams."
                .to_string(),
        ],
        default_frame,
        primitives,
        compositions,
        recipes,
        sections,
    }
}

pub(crate) fn known_primitive_names() -> Vec<String> {
    named_spec_list(design_system_registry().primitives, |primitive| {
        &primitive.name
    })
}

pub(crate) fn known_composition_names() -> Vec<String> {
    named_spec_list(design_system_registry().compositions, |composition| {
        &composition.name
    })
}

pub(crate) fn known_recipe_names() -> Vec<String> {
    named_spec_list(design_system_registry().recipes, |recipe| &recipe.name)
}

pub(crate) fn primitive_template(name: &str) -> Result<DesignTemplate, String> {
    match name.trim() {
        "Kicker" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Kicker".to_string(),
            mdx: r#"<Area x={2} y={2} w={14} h={1}>
  <Kicker>Section label</Kicker>
</Area>"#
                .to_string(),
            notes: vec!["Use as quiet header chrome only.".to_string()],
        }),
        "Takeaway" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Takeaway".to_string(),
            mdx: r#"<Area x={2} y={3} w={46} h={4}>
  <Takeaway>Replace this with the single conclusion for the slide.</Takeaway>
</Area>"#
                .to_string(),
            notes: vec!["Use the full hero band by default.".to_string()],
        }),
        "Panel" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Panel".to_string(),
            mdx: r#"<Area x={2} y={9} w={18} h={12}>
  <Panel title="Evidence" tone="accent">
    Replace with one structured block of evidence.
  </Panel>
</Area>"#
                .to_string(),
            notes: vec!["Use as the default evidence container.".to_string()],
        }),
        "Metric" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Metric".to_string(),
            mdx: r#"<Area x={2} y={9} w={10} h={5}>
  <Metric label="Metric" value="42%" hint="Short note" />
</Area>"#
                .to_string(),
            notes: vec!["Keep values short and hints quiet.".to_string()],
        }),
        "Chart" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Chart".to_string(),
            mdx: r#"<Area x={2} y={9} w={30} h={12}>
  <Chart
    type="bar"
    data="Option A:72;Option B:54;Option C:39"
    suffix="%"
    highlight="Option A"
  />
</Area>"#
                .to_string(),
            notes: vec![
                "Prefer a rendered image asset for polished decks; use the built-in chart only for quick structured exhibits."
                    .to_string(),
            ],
        }),
        "Rule" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Rule".to_string(),
            mdx: r#"<Area x={2} y={20} w={18} h={1}>
  <Rule />
</Area>"#
                .to_string(),
            notes: vec!["Good for separating evidence rows or notes.".to_string()],
        }),
        "Arrow" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Arrow".to_string(),
            mdx: r#"<Area x={22} y={13} w={12} h={2}>
  <Arrow direction="right" label="Flow of work" tone="accent" />
</Area>"#
                .to_string(),
            notes: vec!["Use to connect modules or show sequence.".to_string()],
        }),
        "Callout" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Callout".to_string(),
            mdx: r#"<Area x={34} y={9} w={14} h={11}>
  <Callout tone="accent">
    Explain why the evidence matters.
  </Callout>
</Area>"#
                .to_string(),
            notes: vec!["Keep it short enough to read in one glance.".to_string()],
        }),
        "Quote" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Quote".to_string(),
            mdx: r#"<Area x={2} y={9} w={20} h={10}>
  <Quote attribution="Customer lead">
    Replace with one sharp proof point in the speaker's own words.
  </Quote>
</Area>"#
                .to_string(),
            notes: vec!["Use when one credible voice should anchor the slide.".to_string()],
        }),
        "Caption" => Ok(DesignTemplate {
            kind: "primitive".to_string(),
            name: "Caption".to_string(),
            mdx: r#"<Area x={2} y={24} w={18} h={1}>
  <Caption>Source or operator note</Caption>
</Area>"#
                .to_string(),
            notes: vec!["Use as low-height footer copy inside the body band.".to_string()],
        }),
        _ => Err(format!(
            "Unknown primitive `{}`. Available primitives: {}.",
            name,
            known_primitive_names().join(", ")
        )),
    }
}

fn component_pattern_entries() -> Vec<ComponentCatalogEntry> {
    vec![
        ComponentCatalogEntry {
            name: "ImageFigure".to_string(),
            family: "media".to_string(),
            kind: "pattern".to_string(),
            scope: "builtin".to_string(),
            summary: "One image with a quiet caption below it.".to_string(),
            tags: vec!["image".to_string(), "caption".to_string()],
        },
        ComponentCatalogEntry {
            name: "LogoStrip".to_string(),
            family: "media".to_string(),
            kind: "pattern".to_string(),
            scope: "builtin".to_string(),
            summary: "Three to five visual references in one row.".to_string(),
            tags: vec![
                "image".to_string(),
                "row".to_string(),
                "reference".to_string(),
            ],
        },
        ComponentCatalogEntry {
            name: "BarChartCommentary".to_string(),
            family: "exhibit".to_string(),
            kind: "pattern".to_string(),
            scope: "builtin".to_string(),
            summary: "One bar chart with one interpretation rail.".to_string(),
            tags: vec![
                "chart".to_string(),
                "bar".to_string(),
                "callout".to_string(),
            ],
        },
        ComponentCatalogEntry {
            name: "TrendChartCommentary".to_string(),
            family: "exhibit".to_string(),
            kind: "pattern".to_string(),
            scope: "builtin".to_string(),
            summary: "One trend chart with one interpretation rail.".to_string(),
            tags: vec![
                "chart".to_string(),
                "trend".to_string(),
                "callout".to_string(),
            ],
        },
        ComponentCatalogEntry {
            name: "ArrowBridge".to_string(),
            family: "mark".to_string(),
            kind: "pattern".to_string(),
            scope: "builtin".to_string(),
            summary: "Use one arrow to bridge two modules.".to_string(),
            tags: vec![
                "arrow".to_string(),
                "flow".to_string(),
                "connection".to_string(),
            ],
        },
    ]
}

fn component_pattern_template(name: &str) -> Option<DesignTemplate> {
    match name.trim() {
        "ImageFigure" => Some(DesignTemplate {
            kind: "pattern".to_string(),
            name: "ImageFigure".to_string(),
            mdx: r#"<Area x={2} y={10} w={24} h={10}>
  ![Replace with figure](./assets/figure.png)
</Area>

<Area x={2} y={21} w={24} h={1}>
  <Caption>Replace with figure caption</Caption>
</Area>"#
                .to_string(),
            notes: vec!["Replace the image path with a real project asset.".to_string()],
        }),
        "LogoStrip" => Some(DesignTemplate {
            kind: "pattern".to_string(),
            name: "LogoStrip".to_string(),
            mdx: r#"<Area x={2} y={11} w={46} h={5}>
  <Row gap="sm" align="stretch">
    ![Reference one](./assets/ref-1.png)
    ![Reference two](./assets/ref-2.png)
    ![Reference three](./assets/ref-3.png)
  </Row>
</Area>"#
                .to_string(),
            notes: vec!["Use this for logos, interfaces, or visual references.".to_string()],
        }),
        "BarChartCommentary" => Some(DesignTemplate {
            kind: "pattern".to_string(),
            name: "BarChartCommentary".to_string(),
            mdx: r#"<Area x={2} y={10} w={30} h={11}>
  <Chart
    type="bar"
    data="Option A:72;Option B:54;Option C:39"
    suffix="%"
    highlight="Option A"
  />
</Area>

<Area x={34} y={10} w={14} h={11}>
  <Callout>
    Explain the one thing the audience should learn from the chart.
  </Callout>
</Area>"#
                .to_string(),
            notes: vec![
                "Prefer a themed Python-rendered visual asset when aesthetics matter.".to_string(),
            ],
        }),
        "TrendChartCommentary" => Some(DesignTemplate {
            kind: "pattern".to_string(),
            name: "TrendChartCommentary".to_string(),
            mdx: r#"<Area x={2} y={10} w={30} h={11}>
  <Chart
    type="trend"
    data="Baseline:42;Telemetry:58;Memory:71;Eval loop:84"
    highlight="Eval loop"
  />
</Area>

<Area x={34} y={10} w={14} h={11}>
  <Callout>
    Explain what is compounding and why it matters.
  </Callout>
</Area>"#
                .to_string(),
            notes: vec![
                "Prefer a themed Python-rendered visual asset when aesthetics matter.".to_string(),
                "Use for one directional story only.".to_string(),
            ],
        }),
        "ArrowBridge" => Some(DesignTemplate {
            kind: "pattern".to_string(),
            name: "ArrowBridge".to_string(),
            mdx: r#"<Area x={16} y={13} w={18} h={2}>
  <Arrow direction="right" label="Connect the modules" tone="accent" />
</Area>"#
                .to_string(),
            notes: vec!["Use between two modules, not as standalone decoration.".to_string()],
        }),
        _ => None,
    }
}

pub(crate) fn composition_template(name: &str) -> Result<DesignTemplate, String> {
    match name.trim() {
        "TakeawayRail" => Ok(DesignTemplate {
            kind: "composition".to_string(),
            name: "TakeawayRail".to_string(),
            mdx: r#"<Area x={2} y={2} w={14} h={1}>
  <Kicker>Section</Kicker>
</Area>

<Area x={2} y={3} w={46} h={4}>
  <Takeaway>Replace this with the main conclusion for the slide.</Takeaway>
</Area>

<Area x={34} y={8} w={14} h={14}>
  <Callout tone="accent">
    Explain why the takeaway matters in one short paragraph.
  </Callout>
</Area>

<Area x={2} y={24} w={12} h={1}>
  <Caption>Source or operator note</Caption>
</Area>"#
                .to_string(),
            notes: vec![
                "Paste this inside a Canvas.".to_string(),
                "Treat it as the default opener cluster.".to_string(),
            ],
        }),
        "MetricStrip" => Ok(DesignTemplate {
            kind: "composition".to_string(),
            name: "MetricStrip".to_string(),
            mdx: r#"<Area x={2} y={9} w={30} h={6}>
  <Grid cols={3} gap="sm">
    <Metric label="Metric A" value="12%" hint="Short supporting note" />
    <Metric label="Metric B" value="3.4x" hint="Short supporting note" />
    <Metric label="Metric C" value="24d" hint="Short supporting note" />
  </Grid>
</Area>"#
                .to_string(),
            notes: vec![
                "Use two to four metrics only.".to_string(),
                "If values get long, widen the strip or reduce columns.".to_string(),
            ],
        }),
        "ExhibitCommentary" => Ok(DesignTemplate {
            kind: "composition".to_string(),
            name: "ExhibitCommentary".to_string(),
            mdx: r#"<Area x={2} y={9} w={30} h={13}>
  <Panel>
    | Workflow | Owner | Signal |
    | --- | --- | --- |
    | Discovery | PM | Growing usage |
    | Execution | Ops | High automation fit |
    | Review | Lead | Needs human override |
  </Panel>
</Area>

<Area x={34} y={9} w={14} h={13}>
  <Callout>
    Explain the one thing the audience should take away from the exhibit.
  </Callout>
</Area>"#
                .to_string(),
            notes: vec![
                "Best when there is one clear exhibit.".to_string(),
                "Do not split the story across multiple equal-weight charts.".to_string(),
            ],
        }),
        "ThreeUpPanels" => Ok(DesignTemplate {
            kind: "composition".to_string(),
            name: "ThreeUpPanels".to_string(),
            mdx: r#"<Area x={2} y={9} w={14} h={13}>
  <Panel title="Column one" tone="accent">
    Replace with the first parallel point.
  </Panel>
</Area>

<Area x={18} y={9} w={14} h={13}>
  <Panel title="Column two">
    Replace with the second parallel point.
  </Panel>
</Area>

<Area x={34} y={9} w={14} h={13}>
  <Panel title="Column three">
    Replace with the third parallel point.
  </Panel>
</Area>"#
                .to_string(),
            notes: vec!["Use only when all three columns deserve equal weight.".to_string()],
        }),
        "KpiPair" => Ok(DesignTemplate {
            kind: "composition".to_string(),
            name: "KpiPair".to_string(),
            mdx: r#"<Area x={23} y={9} w={10} h={6}>
  <Metric label="KPI one" value="42%" hint="Short note" />
</Area>

<Area x={23} y={16} w={10} h={6}>
  <Metric label="KPI two" value="18d" hint="Short note" />
</Area>

<Area x={35} y={9} w={13} h={13}>
  <Callout>
    Add the interpretation, risk, or decision implied by the two KPIs.
  </Callout>
</Area>"#
                .to_string(),
            notes: vec!["Use for side modules, not the main story.".to_string()],
        }),
        "BeforeAfter" => Ok(DesignTemplate {
            kind: "composition".to_string(),
            name: "BeforeAfter".to_string(),
            mdx: r#"<Area x={2} y={9} w={19} h={13}>
  <Panel title="Before">
    Replace with the baseline operating model, workflow, or customer experience.
  </Panel>
</Area>

<Area x={22} y={14} w={6} h={2}>
  <Arrow direction="right" label="Shift" tone="accent" />
</Area>

<Area x={29} y={9} w={19} h={13}>
  <Panel title="After" tone="accent">
    Replace with the target state after the change.
  </Panel>
</Area>"#
                .to_string(),
            notes: vec!["Use when the contrast itself is the decision aid.".to_string()],
        }),
        "OperatingModelRow" => Ok(DesignTemplate {
            kind: "composition".to_string(),
            name: "OperatingModelRow".to_string(),
            mdx: r#"<Area x={2} y={9} w={13} h={12}>
  <Panel title="Sense" tone="accent">
    Inputs, signals, and intake.
  </Panel>
</Area>

<Area x={16} y={14} w={2} h={2}>
  <Arrow direction="right" tone="accent" />
</Area>

<Area x={19} y={9} w={13} h={12}>
  <Panel title="Decide">
    Rules, routing, and human checks.
  </Panel>
</Area>

<Area x={33} y={14} w={2} h={2}>
  <Arrow direction="right" tone="accent" />
</Area>

<Area x={36} y={9} w={12} h={12}>
  <Panel title="Act">
    Execution, logging, and review.
  </Panel>
</Area>"#
                .to_string(),
            notes: vec!["Best for sequence bands, handoffs, and operating models.".to_string()],
        }),
        "QuoteEvidence" => Ok(DesignTemplate {
            kind: "composition".to_string(),
            name: "QuoteEvidence".to_string(),
            mdx: r#"<Area x={2} y={9} w={18} h={13}>
  <Quote attribution="Customer lead">
    Replace with one proof quote that earns the audience's trust.
  </Quote>
</Area>

<Area x={22} y={9} w={26} h={13}>
  <Panel title="Evidence" tone="accent">
    - 38% lower handling time
    - Better long-tail answer quality
    - Reviewers kept control at the handoff
  </Panel>
</Area>"#
                .to_string(),
            notes: vec!["Pair the quote with explicit proof, not more narrative.".to_string()],
        }),
        _ => Err(format!(
            "Unknown composition `{}`. Available compositions: {}.",
            name,
            known_composition_names().join(", ")
        )),
    }
}

pub(crate) fn recipe_template(name: &str) -> Result<DesignTemplate, String> {
    match name.trim() {
        "takeaway_plus_rail" => Ok(DesignTemplate {
            kind: "recipe".to_string(),
            name: "takeaway_plus_rail".to_string(),
            mdx: format!(
                "<section className=\"slide\">\n  <Canvas cols={{50}} rows={{25}} gap=\"1px\">\n{}\n  </Canvas>\n</section>",
                composition_template("TakeawayRail")?.mdx
            ),
            notes: vec![
                "Default opener recipe.".to_string(),
                "Body-first frame: keep header/footer thin.".to_string(),
            ],
        }),
        "scorecard_with_note" => Ok(DesignTemplate {
            kind: "recipe".to_string(),
            name: "scorecard_with_note".to_string(),
            mdx: r#"<section className="slide">
  <Canvas cols={50} rows={25} gap="1px">
    <Area x={2} y={2} w={14} h={1}>
      <Kicker>Status</Kicker>
    </Area>

    <Area x={2} y={3} w={46} h={4}>
      <Takeaway>Replace this with one conclusion supported by a small scorecard.</Takeaway>
    </Area>

    <Area x={2} y={9} w={30} h={6}>
      <Grid cols={3} gap="sm">
        <Metric label="Metric A" value="12%" hint="Short note" />
        <Metric label="Metric B" value="3.4x" hint="Short note" />
        <Metric label="Metric C" value="24d" hint="Short note" />
      </Grid>
    </Area>

    <Area x={34} y={9} w={14} h={10}>
      <Callout tone="accent">
        Explain the scorecard in one sentence.
      </Callout>
    </Area>

    <Area x={2} y={24} w={16} h={1}>
      <Caption>Optional source note</Caption>
    </Area>
  </Canvas>
</section>"#
                .to_string(),
            notes: vec!["Use for executive progress and dashboard slides.".to_string()],
        }),
        "exhibit_left_commentary_right" => Ok(DesignTemplate {
            kind: "recipe".to_string(),
            name: "exhibit_left_commentary_right".to_string(),
            mdx: r#"<section className="slide">
  <Canvas cols={50} rows={25} gap="1px">
    <Area x={2} y={2} w={14} h={1}>
      <Kicker>Evidence</Kicker>
    </Area>

    <Area x={2} y={3} w={46} h={4}>
      <Takeaway>Replace this with the single conclusion the exhibit should prove.</Takeaway>
    </Area>

    <Area x={2} y={9} w={30} h={13}>
      <Chart
        type="bar"
        data="Option A:72;Option B:54;Option C:39"
        suffix="%"
        highlight="Option A"
      />
    </Area>

    <Area x={34} y={9} w={14} h={13}>
      <Callout>
        Explain why the exhibit matters and what decision it supports.
      </Callout>
    </Area>

    <Area x={2} y={24} w={16} h={1}>
      <Caption>Optional source note</Caption>
    </Area>
  </Canvas>
</section>"#
                .to_string(),
            notes: vec![
                "Prefer a themed Python-rendered visual asset on the left when polish matters."
                    .to_string(),
            ],
        }),
        "static_vs_dynamic_compare" => Ok(DesignTemplate {
            kind: "recipe".to_string(),
            name: "static_vs_dynamic_compare".to_string(),
            mdx: r#"<section className="slide">
  <Canvas cols={50} rows={25} gap="1px">
    <Area x={2} y={2} w={14} h={1}>
      <Kicker>Compare</Kicker>
    </Area>

    <Area x={2} y={3} w={46} h={4}>
      <Takeaway>Replace with the one contrast that should change the decision.</Takeaway>
    </Area>

    <Area x={2} y={9} w={19} h={13}>
      <Panel title="Static">
        Replace with the baseline process, context window, or operating model.
      </Panel>
    </Area>

    <Area x={22} y={14} w={6} h={2}>
      <Arrow direction="right" label="Shift" tone="accent" />
    </Area>

    <Area x={29} y={9} w={19} h={13}>
      <Panel title="Dynamic" tone="accent">
        Replace with the adaptive state after the move.
      </Panel>
    </Area>

    <Area x={2} y={24} w={16} h={1}>
      <Caption>Optional source note</Caption>
    </Area>
  </Canvas>
</section>"#
                .to_string(),
            notes: vec!["Use when before/after is the clearest executive story.".to_string()],
        }),
        "operating_model" => Ok(DesignTemplate {
            kind: "recipe".to_string(),
            name: "operating_model".to_string(),
            mdx: r#"<section className="slide">
  <Canvas cols={50} rows={25} gap="1px">
    <Area x={2} y={2} w={18} h={1}>
      <Kicker>Operating model</Kicker>
    </Area>

    <Area x={2} y={3} w={46} h={4}>
      <Takeaway>Replace with the operating principle the row should make obvious.</Takeaway>
    </Area>

    <Area x={2} y={9} w={13} h={12}>
      <Panel title="Sense" tone="accent">
        Inputs, signals, and intake.
      </Panel>
    </Area>

    <Area x={16} y={14} w={2} h={2}>
      <Arrow direction="right" tone="accent" />
    </Area>

    <Area x={19} y={9} w={13} h={12}>
      <Panel title="Decide">
        Rules, routing, and human checks.
      </Panel>
    </Area>

    <Area x={33} y={14} w={2} h={2}>
      <Arrow direction="right" tone="accent" />
    </Area>

    <Area x={36} y={9} w={12} h={12}>
      <Panel title="Act">
        Execution, logging, and review.
      </Panel>
    </Area>

    <Area x={2} y={24} w={18} h={1}>
      <Caption>Optional operator note</Caption>
    </Area>
  </Canvas>
</section>"#
                .to_string(),
            notes: vec!["Use concrete stage labels, not abstract nouns.".to_string()],
        }),
        "three_up_compare" => Ok(DesignTemplate {
            kind: "recipe".to_string(),
            name: "three_up_compare".to_string(),
            mdx: r#"<section className="slide">
  <Canvas cols={50} rows={25} gap="1px">
    <Area x={2} y={2} w={14} h={1}>
      <Kicker>Compare</Kicker>
    </Area>

    <Area x={2} y={3} w={46} h={4}>
      <Takeaway>Replace with the one conclusion that frames all three columns.</Takeaway>
    </Area>

    <Area x={2} y={9} w={14} h={13}>
      <Panel title="Column one" tone="accent">
        Replace with the first parallel point.
      </Panel>
    </Area>

    <Area x={18} y={9} w={14} h={13}>
      <Panel title="Column two">
        Replace with the second parallel point.
      </Panel>
    </Area>

    <Area x={34} y={9} w={14} h={13}>
      <Panel title="Column three">
        Replace with the third parallel point.
      </Panel>
    </Area>
  </Canvas>
</section>"#
                .to_string(),
            notes: vec!["Use only when symmetry is the story.".to_string()],
        }),
        "kpi_pair_with_exhibit" => Ok(DesignTemplate {
            kind: "recipe".to_string(),
            name: "kpi_pair_with_exhibit".to_string(),
            mdx: r#"<section className="slide">
  <Canvas cols={50} rows={25} gap="1px">
    <Area x={2} y={2} w={14} h={1}>
      <Kicker>Dashboard</Kicker>
    </Area>

    <Area x={2} y={3} w={46} h={4}>
      <Takeaway>Replace with the conclusion the KPIs and exhibit should prove.</Takeaway>
    </Area>

    <Area x={2} y={9} w={18} h={13}>
      <Chart
        type="bar"
        data="Segment A:47;Segment B:38;Segment C:29;Segment D:21"
        suffix="%"
        highlight="Segment A"
      />
    </Area>

    <Area x={23} y={9} w={10} h={6}>
      <Metric label="KPI one" value="42%" hint="Short note" />
    </Area>

    <Area x={23} y={16} w={10} h={6}>
      <Metric label="KPI two" value="18d" hint="Short note" />
    </Area>

    <Area x={34} y={9} w={14} h={13}>
      <Callout>
        Explain the decision or risk implied by the exhibit and KPIs.
      </Callout>
    </Area>
  </Canvas>
</section>"#
                .to_string(),
            notes: vec!["Good for operational or product reviews.".to_string()],
        }),
        "quote_with_evidence" => Ok(DesignTemplate {
            kind: "recipe".to_string(),
            name: "quote_with_evidence".to_string(),
            mdx: r#"<section className="slide">
  <Canvas cols={50} rows={25} gap="1px">
    <Area x={2} y={2} w={14} h={1}>
      <Kicker>Proof</Kicker>
    </Area>

    <Area x={2} y={3} w={46} h={4}>
      <Takeaway>Replace with the one claim the quote and evidence should substantiate.</Takeaway>
    </Area>

    <Area x={2} y={9} w={18} h={13}>
      <Quote attribution="Customer lead">
        Replace with one proof quote that deserves executive attention.
      </Quote>
    </Area>

    <Area x={22} y={9} w={26} h={13}>
      <Panel title="Evidence" tone="accent">
        - 38% lower handling time
        - Better answer quality on long-tail cases
        - Human review stayed in control
      </Panel>
    </Area>

    <Area x={2} y={24} w={18} h={1}>
      <Caption>Optional source note</Caption>
    </Area>
  </Canvas>
</section>"#
                .to_string(),
            notes: vec!["Use when trust needs both voice and proof.".to_string()],
        }),
        _ => Err(format!(
            "Unknown recipe `{}`. Available recipes: {}.",
            name,
            known_recipe_names().join(", ")
        )),
    }
}

fn primitive_family(name: &str) -> &'static str {
    match name {
        "Kicker" | "Takeaway" | "Quote" | "Caption" => "narrative",
        "Panel" | "Callout" | "Metric" => "container",
        "Chart" => "exhibit",
        "Rule" | "Arrow" => "mark",
        _ => "primitive",
    }
}

fn composition_family(name: &str) -> &'static str {
    match name {
        "TakeawayRail" => "narrative",
        "MetricStrip" | "KpiPair" => "scorecard",
        "ExhibitCommentary" | "QuoteEvidence" => "exhibit",
        "BeforeAfter" | "ThreeUpPanels" => "compare",
        "OperatingModelRow" => "process",
        _ => "composition",
    }
}

fn component_catalog_path() -> Result<PathBuf, String> {
    if let Ok(explicit) = env::var("FASTSLIDES_COMPONENT_LIBRARY_PATH") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return Ok(expand_user_path(trimmed));
        }
    }
    let home = env::var("HOME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| "Could not resolve HOME for component library.".to_string())?;
    Ok(home.join(".fastslides").join("component-library.json"))
}

pub(crate) fn load_saved_components_from(path: &Path) -> Result<Vec<SavedComponentRecord>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read component library {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_str(&content).map_err(|error| {
        format!(
            "Failed to parse component library {}: {error}",
            path.display()
        )
    })
}

pub(crate) fn save_component_to_path(
    path: &Path,
    payload: SaveComponentPayload,
) -> Result<SaveComponentResponse, String> {
    let name = payload.name.trim().to_string();
    let family = payload.family.trim().to_string();
    let summary = payload.summary.trim().to_string();
    let mdx = payload.mdx.trim().to_string();
    if name.is_empty() || family.is_empty() || summary.is_empty() || mdx.is_empty() {
        return Err(
            "Saved components require non-empty name, family, summary, and mdx.".to_string(),
        );
    }

    let parent = path.parent().ok_or_else(|| {
        format!(
            "Component library path has no parent directory: {}",
            path.display()
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Failed to create component library directory {}: {error}",
            parent.display()
        )
    })?;

    let mut records = load_saved_components_from(path)?;
    let record = SavedComponentRecord {
        name: name.clone(),
        family: family.clone(),
        summary: summary.clone(),
        tags: payload.tags.unwrap_or_default(),
        mdx,
        notes: payload.notes.unwrap_or_default(),
    };

    if let Some(existing) = records
        .iter_mut()
        .find(|existing| existing.name.eq_ignore_ascii_case(&name))
    {
        *existing = record.clone();
    } else {
        records.push(record.clone());
    }
    records.sort_by(|left, right| {
        left.family
            .cmp(&right.family)
            .then_with(|| left.name.cmp(&right.name))
    });
    let content = serde_json::to_string_pretty(&records)
        .map_err(|error| format!("Failed to serialize component library: {error}"))?;
    fs::write(path, content).map_err(|error| {
        format!(
            "Failed to write component library {}: {error}",
            path.display()
        )
    })?;

    Ok(SaveComponentResponse {
        ok: true,
        component: ComponentCatalogEntry {
            name: record.name,
            family: record.family,
            kind: "saved-component".to_string(),
            scope: "saved".to_string(),
            summary: record.summary,
            tags: record.tags,
        },
        library_path: path_to_string(path),
    })
}

fn component_catalog_entries() -> Result<Vec<ComponentCatalogEntry>, String> {
    let registry = design_system_registry();
    let mut entries = Vec::<ComponentCatalogEntry>::new();

    entries.extend(
        registry
            .primitives
            .into_iter()
            .map(|primitive| ComponentCatalogEntry {
                name: primitive.name.clone(),
                family: primitive_family(&primitive.name).to_string(),
                kind: "primitive".to_string(),
                scope: "builtin".to_string(),
                summary: primitive.purpose,
                tags: primitive.variants,
            }),
    );
    entries.extend(
        registry
            .compositions
            .into_iter()
            .map(|composition| ComponentCatalogEntry {
                name: composition.name.clone(),
                family: composition_family(&composition.name).to_string(),
                kind: "composition".to_string(),
                scope: "builtin".to_string(),
                summary: composition.purpose,
                tags: composition.source_primitives,
            }),
    );
    entries.extend(
        registry
            .recipes
            .into_iter()
            .map(|recipe| ComponentCatalogEntry {
                name: recipe.name,
                family: "recipe".to_string(),
                kind: "recipe".to_string(),
                scope: "builtin".to_string(),
                summary: recipe.summary,
                tags: recipe.compositions,
            }),
    );
    entries.extend(component_pattern_entries());

    if let Ok(path) = component_catalog_path() {
        entries.extend(
            load_saved_components_from(&path)?
                .into_iter()
                .map(|record| ComponentCatalogEntry {
                    name: record.name,
                    family: record.family,
                    kind: "saved-component".to_string(),
                    scope: "saved".to_string(),
                    summary: record.summary,
                    tags: record.tags,
                }),
        );
    }

    entries.sort_by(|left, right| {
        left.family
            .cmp(&right.family)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(entries)
}

pub(crate) fn save_component_template(
    payload: SaveComponentPayload,
) -> Result<SaveComponentResponse, String> {
    let path = component_catalog_path()?;
    save_component_to_path(&path, payload)
}

pub(crate) fn component_template(name: &str) -> Result<DesignTemplate, String> {
    primitive_template(name)
        .or_else(|_| composition_template(name))
        .or_else(|_| recipe_template(name))
        .or_else(|_| {
            component_pattern_template(name).ok_or_else(|| {
                format!("Unknown component `{name}`.")
            })
        })
        .or_else(|_| {
            let path = component_catalog_path()?;
            let record = load_saved_components_from(&path)?
                .into_iter()
                .find(|component| component.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| {
                    format!(
                        "Unknown component `{}`. Query the component catalog for available built-ins and saved snippets.",
                        name
                    )
                })?;
            Ok(DesignTemplate {
                kind: "saved-component".to_string(),
                name: record.name,
                mdx: record.mdx,
                notes: record.notes,
            })
        })
}

pub(crate) fn build_component_catalog() -> Result<ComponentCatalog, String> {
    Ok(ComponentCatalog {
        version: "1.0".to_string(),
        items: component_catalog_entries()?,
    })
}
