use super::compile::{
    build_project_scene_manifest_from_source, clean_scene_text, compile_slide_nodes,
    try_build_scene_slide,
};
use super::types::{
    DeckOutlineEntry, NamedCount, ProjectAnalysis, ProjectSceneSessionEventPayload,
    ProjectSceneSource, SceneNode, SceneSlide, SlideAnalysis,
};
use crate::config::{normalize_existing_project_directory, path_to_string};
use crate::deck::{
    bullet_re, component_counts_for_slide, extract_frontmatter, extract_slides, increment_count,
    inferred_archetype, max_paragraph_words, slide_title_for, split_class_re, uses_spatial_canvas,
    words_in_text, STRUCTURED_COMPONENT_NAMES,
};
use crate::projects::read_page_mdx;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
};

static SCENE_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy)]
struct CanvasFrame {
    cols: usize,
    rows: usize,
}

#[derive(Debug, Clone, Copy)]
struct AreaFrame {
    w: usize,
    h: usize,
}

impl AreaFrame {
    fn size_label(self) -> String {
        format!("{} x {}", self.w, self.h)
    }
}

pub(crate) fn next_scene_session_id() -> String {
    let next = SCENE_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("scene-session-{}-{next}", crate::now_epoch_seconds())
}

pub(crate) fn preferred_scene_session_worker_count() -> usize {
    let available = thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4);
    available.clamp(2, 6)
}

pub(crate) fn prioritize_scene_slide_indices(total: usize, start_index: usize) -> Vec<usize> {
    if total == 0 {
        return Vec::new();
    }

    let clamped_start = start_index.min(total - 1);
    let mut ordered = Vec::<usize>::with_capacity(total);

    for offset in 0..total {
        let forward = clamped_start + offset;
        if forward < total {
            ordered.push(forward);
        }

        if offset > 0 {
            let backward = clamped_start.saturating_sub(offset);
            if backward < clamped_start {
                ordered.push(backward);
            }
        }

        if ordered.len() >= total {
            break;
        }
    }

    ordered.truncate(total);
    ordered
}

fn sorted_named_counts(counts: HashMap<String, usize>) -> Vec<NamedCount> {
    let mut entries: Vec<_> = counts
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(name, count)| NamedCount { name, count })
        .collect();

    entries.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.name.cmp(&right.name))
    });
    entries
}

pub(crate) fn emit_project_scene_session_events<F>(
    source: ProjectSceneSource,
    priority_index: usize,
    worker_count: usize,
    emit: &mut F,
) -> Result<(), String>
where
    F: FnMut(ProjectSceneSessionEventPayload),
{
    emit(ProjectSceneSessionEventPayload::Manifest {
        scene: build_project_scene_manifest_from_source(&source),
    });

    let slide_count = source.slides.len();
    if slide_count == 0 {
        emit(ProjectSceneSessionEventPayload::Complete {
            ready_count: 0,
            error_count: 0,
        });
        return Ok(());
    }

    let ordered_indices = prioritize_scene_slide_indices(slide_count, priority_index);
    let primary_index = ordered_indices[0];
    let mut ready_count = 0usize;
    let mut error_count = 0usize;

    match try_build_scene_slide(&source.slides, primary_index) {
        Ok(slide) => {
            ready_count += 1;
            emit(ProjectSceneSessionEventPayload::SlideReady { slide });
        }
        Err(error) => {
            error_count += 1;
            emit(ProjectSceneSessionEventPayload::SlideError {
                index: primary_index,
                error,
            });
        }
    }

    let remaining: Vec<usize> = ordered_indices.into_iter().skip(1).collect();
    if remaining.is_empty() {
        emit(ProjectSceneSessionEventPayload::Complete {
            ready_count,
            error_count,
        });
        return Ok(());
    }

    let slides = Arc::new(source.slides);
    let queue = Arc::new(Mutex::new(VecDeque::from(remaining.clone())));
    let (tx, rx) = mpsc::channel::<(usize, Result<SceneSlide, String>)>();
    let worker_total = remaining.len().min(worker_count.max(1));
    let mut handles = Vec::<thread::JoinHandle<()>>::with_capacity(worker_total);

    for _ in 0..worker_total {
        let tx = tx.clone();
        let queue = Arc::clone(&queue);
        let slides = Arc::clone(&slides);
        handles.push(thread::spawn(move || loop {
            let next_index = {
                let mut pending = match queue.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                pending.pop_front()
            };

            let Some(index) = next_index else {
                break;
            };

            let result = try_build_scene_slide(slides.as_ref(), index);
            if tx.send((index, result)).is_err() {
                break;
            }
        }));
    }
    drop(tx);

    for _ in 0..remaining.len() {
        let (index, result) = rx
            .recv()
            .map_err(|error| format!("Scene session worker channel failed: {error}"))?;
        match result {
            Ok(slide) => {
                ready_count += 1;
                emit(ProjectSceneSessionEventPayload::SlideReady { slide });
            }
            Err(error) => {
                error_count += 1;
                emit(ProjectSceneSessionEventPayload::SlideError { index, error });
            }
        }
    }

    for handle in handles {
        let _ = handle.join();
    }

    emit(ProjectSceneSessionEventPayload::Complete {
        ready_count,
        error_count,
    });
    Ok(())
}

#[cfg(test)]
pub(crate) fn collect_project_scene_session_events(
    source: ProjectSceneSource,
    priority_index: usize,
    worker_count: usize,
) -> Result<Vec<ProjectSceneSessionEventPayload>, String> {
    let mut events = Vec::<ProjectSceneSessionEventPayload>::new();
    emit_project_scene_session_events(source, priority_index, worker_count, &mut |event| {
        events.push(event);
    })?;
    Ok(events)
}

fn push_contract_warning(seen: &mut HashSet<String>, warnings: &mut Vec<String>, message: String) {
    if seen.insert(message.clone()) {
        warnings.push(message);
    }
}

fn scene_nodes_text_length(nodes: &[SceneNode]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            SceneNode::Canvas { children, .. }
            | SceneNode::Area { children, .. }
            | SceneNode::LayoutGroup { children, .. }
            | SceneNode::Surface { children, .. } => scene_nodes_text_length(children),
            SceneNode::Metric {
                label, value, hint, ..
            } => {
                label.as_deref().unwrap_or_default().chars().count()
                    + value.as_deref().unwrap_or_default().chars().count()
                    + hint.as_deref().unwrap_or_default().chars().count()
            }
            SceneNode::Chart {
                title,
                data,
                value_suffix,
                ..
            } => {
                title.as_deref().unwrap_or_default().chars().count()
                    + value_suffix.as_deref().unwrap_or_default().chars().count()
                    + data
                        .iter()
                        .map(|item| item.label.chars().count())
                        .sum::<usize>()
            }
            SceneNode::Text { text, .. } => text.chars().count(),
            SceneNode::List { items, .. } => items.iter().map(|item| item.chars().count()).sum(),
            SceneNode::Media { alt, .. } => alt.as_deref().unwrap_or_default().chars().count(),
            SceneNode::Arrow { label, .. } => label.as_deref().unwrap_or_default().chars().count(),
            SceneNode::CodeBlock { code, .. } | SceneNode::Raw { text: code, .. } => {
                code.chars().count()
            }
            SceneNode::Pill { text, .. } => text.chars().count(),
            SceneNode::Rule { .. } => 0,
        })
        .sum()
}

fn metric_node_summary(node: &SceneNode) -> Option<(usize, usize, usize)> {
    match node {
        SceneNode::Metric {
            label, value, hint, ..
        } => Some((
            label.as_deref().unwrap_or_default().chars().count(),
            value.as_deref().unwrap_or_default().chars().count(),
            hint.as_deref().unwrap_or_default().chars().count(),
        )),
        _ => None,
    }
}

fn estimate_takeaway_rows(text: &str, area_cols: usize) -> f32 {
    let text_len = clean_scene_text(text).chars().count() as f32;
    if text_len <= 0.0 {
        return 0.0;
    }
    let chars_per_line = (area_cols as f32 * 1.05).max(18.0);
    let line_count = (text_len / chars_per_line).ceil().max(1.0);
    1.0 + (line_count * 1.55)
}

fn estimate_callout_rows(title: Option<&str>, body_len: usize, area_cols: usize) -> f32 {
    let title_len = title.unwrap_or_default().chars().count();
    let total_len = title_len + body_len;
    if total_len == 0 {
        return 0.0;
    }
    let chars_per_line = (area_cols as f32 * 1.25).max(16.0);
    let line_count = (total_len as f32 / chars_per_line).ceil().max(1.0);
    2.6 + (line_count * 1.15)
}

fn maybe_warn_takeaway_contract(
    area: AreaFrame,
    text: &str,
    seen: &mut HashSet<String>,
    warnings: &mut Vec<String>,
) {
    let required_rows = estimate_takeaway_rows(text, area.w);
    if required_rows > area.h as f32 + 0.25 {
        push_contract_warning(
            seen,
            warnings,
            format!(
                "Takeaway is too dense for a {} hero area. Give it more rows or shorten the sentence.",
                area.size_label()
            ),
        );
    }
}

fn maybe_warn_metric_contract(
    area: AreaFrame,
    value_len: usize,
    hint_len: usize,
    grid_cols: Option<usize>,
    seen: &mut HashSet<String>,
    warnings: &mut Vec<String>,
) {
    if grid_cols.is_some() || value_len == 0 {
        return;
    }

    let narrow_value = area.w <= 10 && value_len >= 10;
    let dense_copy = area.h <= 5 && hint_len > area.w.saturating_mul(2);
    if narrow_value || dense_copy {
        push_contract_warning(
            seen,
            warnings,
            format!(
                "Metric is too dense for a {} tile. Use a shorter value, more columns, or a compact scorecard recipe.",
                area.size_label()
            ),
        );
    }
}

fn maybe_warn_metric_grid_contract(
    area: AreaFrame,
    cols: usize,
    children: &[SceneNode],
    seen: &mut HashSet<String>,
    warnings: &mut Vec<String>,
) {
    if cols < 2 {
        return;
    }

    let metrics: Vec<_> = children.iter().filter_map(metric_node_summary).collect();
    if metrics.len() < 2 {
        return;
    }

    let card_cols = area.w as f32 / cols as f32;
    let longest_value = metrics
        .iter()
        .map(|(_, value_len, _)| *value_len)
        .max()
        .unwrap_or(0);
    let longest_hint = metrics
        .iter()
        .map(|(_, _, hint_len)| *hint_len)
        .max()
        .unwrap_or(0);
    let values_too_long = longest_value as f32 > card_cols * 0.95;
    let narrow_cards = card_cols <= 10.5 && longest_value >= 8;
    let dense_hints = area.h <= 10 && longest_hint > (card_cols as usize).saturating_mul(3);

    if values_too_long || narrow_cards || dense_hints {
        push_contract_warning(
            seen,
            warnings,
            format!(
                "{}-up metric grid is too tight inside a {} area. Use wider cards, fewer columns, or shorter metric values.",
                cols,
                area.size_label()
            ),
        );
    }
}

fn maybe_warn_callout_contract(
    area: AreaFrame,
    title: Option<&str>,
    children: &[SceneNode],
    seen: &mut HashSet<String>,
    warnings: &mut Vec<String>,
) {
    let required_rows = estimate_callout_rows(title, scene_nodes_text_length(children), area.w);
    if required_rows > area.h as f32 + 0.25 {
        push_contract_warning(
            seen,
            warnings,
            format!(
                "Callout copy is too dense for a {} rail. Widen the rail or reduce the copy.",
                area.size_label()
            ),
        );
    }
}

fn maybe_warn_chart_contract(
    area: AreaFrame,
    chart_type: &str,
    points: usize,
    seen: &mut HashSet<String>,
    warnings: &mut Vec<String>,
) {
    let min_cols = if chart_type.eq_ignore_ascii_case("trend") {
        20
    } else {
        18
    };
    let too_small = area.w < min_cols || area.h < 8;
    let too_many_points = points > 8 && area.w < 32;
    let too_few_points = points < 2;
    if too_small || too_many_points || too_few_points {
        push_contract_warning(
            seen,
            warnings,
            format!(
                "Chart is too constrained for a {} area. Give it at least {} x 8 and keep the series concise.",
                area.size_label(),
                min_cols
            ),
        );
    }
}

fn collect_scene_contract_warnings(
    nodes: &[SceneNode],
    canvas: Option<CanvasFrame>,
    area: Option<AreaFrame>,
    grid_cols: Option<usize>,
    seen: &mut HashSet<String>,
    warnings: &mut Vec<String>,
) {
    for node in nodes {
        match node {
            SceneNode::Canvas {
                cols,
                rows,
                children,
                ..
            } => collect_scene_contract_warnings(
                children,
                Some(CanvasFrame {
                    cols: *cols,
                    rows: *rows,
                }),
                None,
                None,
                seen,
                warnings,
            ),
            SceneNode::Area {
                x,
                y,
                w,
                h,
                children,
                ..
            } => {
                let area_frame = AreaFrame { w: *w, h: *h };
                if let Some(canvas_frame) = canvas {
                    let right_edge = x.saturating_add(*w).saturating_sub(1);
                    let bottom_edge = y.saturating_add(*h).saturating_sub(1);
                    if right_edge > canvas_frame.cols || bottom_edge > canvas_frame.rows {
                        push_contract_warning(
                            seen,
                            warnings,
                            format!(
                                "Area at ({}, {}) with size {} exceeds the {} x {} canvas bounds.",
                                x,
                                y,
                                area_frame.size_label(),
                                canvas_frame.cols,
                                canvas_frame.rows
                            ),
                        );
                    }
                }
                collect_scene_contract_warnings(
                    children,
                    canvas,
                    Some(area_frame),
                    None,
                    seen,
                    warnings,
                );
            }
            SceneNode::LayoutGroup {
                component,
                cols,
                children,
                ..
            } => {
                let next_grid_cols = if component == "Grid" {
                    Some(cols.unwrap_or_else(|| {
                        children
                            .iter()
                            .filter(|child| matches!(child, SceneNode::Metric { .. }))
                            .count()
                            .clamp(1, 4)
                    }))
                } else {
                    grid_cols
                };

                if component == "Grid" {
                    if let (Some(area_frame), Some(resolved_cols)) = (area, next_grid_cols) {
                        maybe_warn_metric_grid_contract(
                            area_frame,
                            resolved_cols,
                            children,
                            seen,
                            warnings,
                        );
                    }
                }

                collect_scene_contract_warnings(
                    children,
                    canvas,
                    area,
                    next_grid_cols,
                    seen,
                    warnings,
                );
            }
            SceneNode::Surface {
                component,
                title,
                children,
                ..
            } => {
                if component == "Callout" {
                    if let Some(area_frame) = area {
                        maybe_warn_callout_contract(
                            area_frame,
                            title.as_deref(),
                            children,
                            seen,
                            warnings,
                        );
                    }
                }

                collect_scene_contract_warnings(children, canvas, area, grid_cols, seen, warnings);
            }
            SceneNode::Metric { value, hint, .. } => {
                if let Some(area_frame) = area {
                    maybe_warn_metric_contract(
                        area_frame,
                        value.as_deref().unwrap_or_default().chars().count(),
                        hint.as_deref().unwrap_or_default().chars().count(),
                        grid_cols,
                        seen,
                        warnings,
                    );
                }
            }
            SceneNode::Chart {
                chart_type, data, ..
            } => {
                if let Some(area_frame) = area {
                    maybe_warn_chart_contract(area_frame, chart_type, data.len(), seen, warnings);
                }
            }
            SceneNode::Text { role, text, .. } => {
                if role == "takeaway" {
                    if let Some(area_frame) = area {
                        maybe_warn_takeaway_contract(area_frame, text, seen, warnings);
                    }
                }
            }
            SceneNode::List { .. }
            | SceneNode::Media { .. }
            | SceneNode::CodeBlock { .. }
            | SceneNode::Pill { .. }
            | SceneNode::Rule { .. }
            | SceneNode::Arrow { .. }
            | SceneNode::Raw { .. } => {}
        }
    }
}

pub(crate) fn slide_contract_warnings(slide: &str) -> Vec<String> {
    let mut warnings = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    let nodes = compile_slide_nodes(slide);
    collect_scene_contract_warnings(&nodes, None, None, None, &mut seen, &mut warnings);
    warnings
}

fn slide_review_warnings(
    slide: &str,
    title: &str,
    words: usize,
    bullets: usize,
    paragraph_words: usize,
    component_counts: &HashMap<String, usize>,
) -> Vec<String> {
    let mut warnings = Vec::<String>::new();

    if title.starts_with("Slide ") {
        warnings.push("Slide has no explicit heading.".to_string());
    }
    if !uses_spatial_canvas(component_counts) {
        warnings.push(
            "Slide must use the 2.0 spatial contract: one Canvas with Area regions.".to_string(),
        );
    }
    if words > 110 {
        warnings.push(format!("High density: {words} words."));
    }
    if bullets > 6 {
        warnings.push(format!("Too many list items: {bullets}."));
    }
    if paragraph_words > 45 {
        warnings.push(format!("Longest paragraph is {paragraph_words} words."));
    }
    if split_class_re().is_match(slide) {
        warnings.push(
            "Legacy `split` layout is no longer supported. Replace it with Canvas and Area."
                .to_string(),
        );
    }
    warnings.extend(slide_contract_warnings(slide));

    warnings
}

pub(crate) fn build_project_analysis(project_path: &Path) -> Result<ProjectAnalysis, String> {
    let canonical_project = normalize_existing_project_directory(&path_to_string(project_path))?;
    let source = read_page_mdx(&canonical_project)?;
    let (_, body) = extract_frontmatter(&source);
    let slides = extract_slides(&body);

    let mut outline = Vec::<DeckOutlineEntry>::new();
    let mut slide_analyses = Vec::<SlideAnalysis>::new();
    let mut project_components = HashMap::<String, usize>::new();
    let mut archetype_counts = HashMap::<String, usize>::new();
    let mut warnings = Vec::<String>::new();
    let mut non_spatial_slide_total = 0usize;

    for (index, slide) in slides.iter().enumerate() {
        let title = slide_title_for(slide, index);
        let words = words_in_text(slide);
        let bullets = bullet_re().find_iter(slide).count();
        let paragraph_words = max_paragraph_words(slide);
        let component_counts = component_counts_for_slide(slide);
        if !uses_spatial_canvas(&component_counts) {
            non_spatial_slide_total += 1;
        }
        let archetype = inferred_archetype(slide, words, bullets, &component_counts);
        let slide_warnings = slide_review_warnings(
            slide,
            &title,
            words,
            bullets,
            paragraph_words,
            &component_counts,
        );

        outline.push(DeckOutlineEntry {
            index,
            title: title.clone(),
        });

        for (name, count) in &component_counts {
            increment_count(&mut project_components, name.as_str(), *count);
        }
        increment_count(&mut archetype_counts, archetype.as_str(), 1);

        slide_analyses.push(SlideAnalysis {
            index,
            title,
            archetype,
            words,
            bullets,
            max_paragraph_words: paragraph_words,
            components: sorted_named_counts(component_counts),
            warnings: slide_warnings,
        });
    }

    let has_project_css = canonical_project.join("slides.css").exists();
    if !has_project_css {
        warnings.push(
            "Project has no `slides.css`; 2.0 decks should carry project-level theme tokens."
                .to_string(),
        );
    }

    let structured_component_total = STRUCTURED_COMPONENT_NAMES
        .iter()
        .map(|name| project_components.get(*name).copied().unwrap_or(0))
        .sum::<usize>();
    if structured_component_total == 0 && !slides.is_empty() {
        warnings.push(
            "Deck does not use FastSlides 2.0 primitives; move slides to Canvas and Area."
                .to_string(),
        );
    }

    if non_spatial_slide_total > 0 {
        warnings.push(format!(
            "{non_spatial_slide_total} slide(s) are not on the 2.0 spatial canvas contract."
        ));
    }

    if slides.len() >= 4 {
        if let Some(dominant) = archetype_counts
            .iter()
            .max_by(|left, right| left.1.cmp(right.1).then_with(|| right.0.cmp(left.0)))
        {
            if dominant.0 != "spatial-canvas" && *dominant.1 * 100 / slides.len() >= 75 {
                warnings.push(format!(
                    "Most slides resolve to `{}`. Consider more archetype variety for stronger pacing.",
                    dominant.0
                ));
            }
        }
    }

    let slides_with_findings = slide_analyses
        .iter()
        .filter(|slide| !slide.warnings.is_empty())
        .count();
    if slides_with_findings > 0 {
        warnings.push(format!(
            "{slides_with_findings} slide(s) have density or structure findings."
        ));
    }

    Ok(ProjectAnalysis {
        path: path_to_string(&canonical_project),
        slide_count: slides.len(),
        has_project_css,
        outline,
        components: sorted_named_counts(project_components),
        archetypes: sorted_named_counts(archetype_counts),
        warnings,
        slides: slide_analyses,
    })
}
