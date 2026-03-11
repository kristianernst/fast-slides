use super::types::{
    ProjectScene, ProjectSceneManifest, ProjectSceneSource, SceneChartDatum, SceneLayout,
    SceneNode, SceneSlide, SceneSlideManifest,
};
use crate::config::{normalize_existing_project_directory, path_to_string};
use crate::deck::{
    clean_heading_text, component_counts_for_slide, extract_frontmatter, extract_slides,
    html_tag_re, normalize_frontmatter_value, sanitize_markdown_target, slide_title_for,
    split_class_re, uses_spatial_canvas, LAYOUT_COMPONENT_NAMES,
};
use crate::projects::read_page_mdx;
use regex::Regex;
use std::{
    collections::HashMap,
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
    sync::OnceLock,
};

#[derive(Debug, Clone)]
struct ComponentBlock {
    name: String,
    start: usize,
    end: usize,
    attrs: HashMap<String, String>,
    inner: Option<String>,
    source: String,
}

#[derive(Debug, Clone)]
struct CodeFenceBlock {
    start: usize,
    end: usize,
    language: Option<String>,
    code: String,
}

fn scene_attr_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"([A-Za-z_][A-Za-z0-9_-]*)\s*=\s*(?:"([^"]*)"|'([^']*)'|\{([^{}]*)\})"#)
            .expect("invalid scene attr regex")
    })
}

fn markdown_heading_capture_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s{0,3}(#{1,3})\s+(.+?)\s*$"#)
            .expect("invalid markdown heading capture regex")
    })
}

fn html_heading_block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?is)<h([1-3])[^>]*>(.*?)</h[1-3]>"#)
            .expect("invalid html heading block regex")
    })
}

fn markdown_list_item_capture_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*((?:[-*+])|(?:\d+\.))\s+(.+?)\s*$"#)
            .expect("invalid markdown list capture regex")
    })
}

fn html_list_block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?is)<(ul|ol)[^>]*>(.*?)</(?:ul|ol)>"#)
            .expect("invalid html list block regex")
    })
}

fn html_list_item_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?is)<li[^>]*>(.*?)</li>"#).expect("invalid html list item regex")
    })
}

fn code_fence_block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)```([A-Za-z0-9_+-]+)?\n(.*?)\n?```"#).expect("invalid code fence regex")
    })
}

fn markdown_image_capture_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"!\[([^\]]*)\]\(([^)]+)\)"#).expect("invalid markdown image regex")
    })
}

fn html_image_block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?is)<img\b([^>]*?)>"#).expect("invalid html image regex"))
}

fn html_video_block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?is)<video\b([^>]*?)>"#).expect("invalid html video regex"))
}

fn find_tag_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut index = start + 1;
    let mut quote = None::<u8>;
    let mut brace_depth = 0usize;

    while index < bytes.len() {
        let current = bytes[index];
        if let Some(active_quote) = quote {
            if current == active_quote && bytes.get(index.saturating_sub(1)) != Some(&b'\\') {
                quote = None;
            }
        } else {
            match current {
                b'"' | b'\'' => quote = Some(current),
                b'{' => brace_depth += 1,
                b'}' => brace_depth = brace_depth.saturating_sub(1),
                b'>' if brace_depth == 0 => return Some(index),
                _ => {}
            }
        }
        index += 1;
    }

    None
}

fn is_self_closing_tag(source: &str, tag_start: usize, tag_end: usize) -> bool {
    let raw = source[tag_start + 1..tag_end].trim_end();
    raw.ends_with('/')
}

fn component_starts_at(source: &str, start: usize, name: &str) -> bool {
    let Some(rest) = source.get(start..) else {
        return false;
    };
    if !rest.starts_with('<') || rest.starts_with("</") {
        return false;
    }
    let expected = format!("<{name}");
    if !rest.starts_with(expected.as_str()) {
        return false;
    }
    rest.chars()
        .nth(expected.chars().count())
        .map(|ch| ch.is_whitespace() || ch == '>' || ch == '/')
        .unwrap_or(true)
}

fn parse_component_attrs(raw: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::<String, String>::new();
    for captures in scene_attr_re().captures_iter(raw) {
        let Some(name) = captures.get(1).map(|item| item.as_str().to_string()) else {
            continue;
        };
        let raw_value = captures
            .get(2)
            .or_else(|| captures.get(3))
            .or_else(|| captures.get(4))
            .map(|item| item.as_str())
            .unwrap_or_default();
        let value = normalize_frontmatter_value(raw_value);
        if !value.is_empty() {
            attrs.insert(name, value);
        }
    }
    attrs
}

fn attr_text(attrs: &HashMap<String, String>, name: &str) -> Option<String> {
    attrs
        .get(name)
        .cloned()
        .filter(|value| !value.trim().is_empty())
}

fn attr_usize(attrs: &HashMap<String, String>, name: &str) -> Option<usize> {
    attrs
        .get(name)
        .and_then(|value| value.trim().parse::<usize>().ok())
}

fn attr_bool(attrs: &HashMap<String, String>, name: &str) -> Option<bool> {
    attrs.get(name).and_then(|value| {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    })
}

fn attr_class_name(attrs: &HashMap<String, String>) -> Option<String> {
    attr_text(attrs, "className")
}

fn parse_chart_value(raw: &str) -> Option<f32> {
    let normalized = raw.trim().trim_end_matches('%').replace(',', "");
    if normalized.is_empty() {
        return None;
    }
    normalized.parse::<f32>().ok()
}

fn parse_chart_data(raw: &str) -> Vec<SceneChartDatum> {
    raw.split([';', '\n'])
        .filter_map(|entry| {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                return None;
            }
            let (label, value) = trimmed
                .split_once(':')
                .or_else(|| trimmed.split_once('='))
                .unwrap_or((trimmed, ""));
            let label = label.trim().to_string();
            let value = parse_chart_value(value)?;
            if label.is_empty() {
                return None;
            }
            Some(SceneChartDatum { label, value })
        })
        .collect()
}

fn build_component_block(source: &str, start: usize, name: &str) -> Option<ComponentBlock> {
    let tag_end = find_tag_end(source, start)?;
    let attrs_start = start + 1 + name.len();
    let attrs_source = if is_self_closing_tag(source, start, tag_end) {
        source[attrs_start..tag_end]
            .trim_end()
            .strip_suffix('/')
            .unwrap_or(source[attrs_start..tag_end].trim_end())
            .trim()
    } else {
        source[attrs_start..tag_end].trim()
    };
    let attrs = parse_component_attrs(attrs_source);

    if is_self_closing_tag(source, start, tag_end) {
        return Some(ComponentBlock {
            name: name.to_string(),
            start,
            end: tag_end + 1,
            attrs,
            inner: None,
            source: source[start..tag_end + 1].to_string(),
        });
    }

    let open_pattern = format!("<{name}");
    let close_pattern = format!("</{name}");
    let mut depth = 1usize;
    let mut cursor = tag_end + 1;

    while cursor < source.len() {
        let next_open = source[cursor..]
            .find(open_pattern.as_str())
            .map(|offset| cursor + offset)
            .filter(|position| component_starts_at(source, *position, name));
        let next_close = source[cursor..]
            .find(close_pattern.as_str())
            .map(|offset| cursor + offset);

        match (next_open, next_close) {
            (Some(open_start), Some(close_start)) if close_start < open_start => {
                let close_end = source[close_start..]
                    .find('>')
                    .map(|offset| close_start + offset)?;
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let inner = source[tag_end + 1..close_start].to_string();
                    return Some(ComponentBlock {
                        name: name.to_string(),
                        start,
                        end: close_end + 1,
                        attrs,
                        inner: Some(inner),
                        source: source[start..close_end + 1].to_string(),
                    });
                }
                cursor = close_end + 1;
            }
            (None, Some(close_start)) => {
                let close_end = source[close_start..]
                    .find('>')
                    .map(|offset| close_start + offset)?;
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let inner = source[tag_end + 1..close_start].to_string();
                    return Some(ComponentBlock {
                        name: name.to_string(),
                        start,
                        end: close_end + 1,
                        attrs,
                        inner: Some(inner),
                        source: source[start..close_end + 1].to_string(),
                    });
                }
                cursor = close_end + 1;
            }
            (Some(open_start), _) => {
                let open_end = find_tag_end(source, open_start)?;
                if !is_self_closing_tag(source, open_start, open_end) {
                    depth += 1;
                }
                cursor = open_end + 1;
            }
            (None, None) => break,
        }
    }

    None
}

fn next_component_block(source: &str, start: usize, names: &[&str]) -> Option<ComponentBlock> {
    let mut cursor = start;
    while let Some(offset) = source[cursor..].find('<') {
        let candidate = cursor + offset;
        for name in names {
            if component_starts_at(source, candidate, name) {
                if let Some(block) = build_component_block(source, candidate, name) {
                    return Some(block);
                }
            }
        }
        cursor = candidate + 1;
    }
    None
}

fn extract_component_blocks(source: &str, names: &[&str]) -> Vec<ComponentBlock> {
    let mut blocks = Vec::<ComponentBlock>::new();
    let mut cursor = 0usize;
    while let Some(block) = next_component_block(source, cursor, names) {
        cursor = block.end;
        blocks.push(block);
    }
    blocks
}

fn extract_code_fence_blocks(source: &str) -> Vec<CodeFenceBlock> {
    code_fence_block_re()
        .captures_iter(source)
        .filter_map(|captures| {
            let full = captures.get(0)?;
            let language = captures
                .get(1)
                .map(|item| item.as_str().trim().to_string())
                .filter(|value| !value.is_empty());
            let code = captures
                .get(2)
                .map(|item| item.as_str().trim_end().to_string())
                .unwrap_or_default();
            Some(CodeFenceBlock {
                start: full.start(),
                end: full.end(),
                language,
                code,
            })
        })
        .collect()
}

pub(super) fn clean_scene_text(raw: &str) -> String {
    let with_breaks = raw
        .replace("<br />", "\n")
        .replace("<br/>", "\n")
        .replace("<br>", "\n")
        .replace("</p>", "\n")
        .replace("</div>", "\n")
        .replace("</section>", "\n");
    let without_tags = html_tag_re().replace_all(&with_breaks, " ");
    without_tags
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn compile_component_node(block: &ComponentBlock) -> SceneNode {
    match block.name.as_str() {
        "Stack" | "Row" | "Grid" | "PillRow" => SceneNode::LayoutGroup {
            component: block.name.clone(),
            cols: attr_usize(&block.attrs, "cols"),
            gap: attr_text(&block.attrs, "gap"),
            align: attr_text(&block.attrs, "align"),
            justify: attr_text(&block.attrs, "justify"),
            nowrap: attr_bool(&block.attrs, "nowrap"),
            class_name: attr_class_name(&block.attrs),
            children: block
                .inner
                .as_deref()
                .map(compile_fragment_nodes)
                .unwrap_or_default(),
            source_mdx: block.source.clone(),
        },
        "Card" | "Panel" | "Callout" | "Quote" => SceneNode::Surface {
            component: block.name.clone(),
            tone: attr_text(&block.attrs, "tone"),
            title: attr_text(&block.attrs, "title"),
            kicker: attr_text(&block.attrs, "kicker"),
            subtitle: attr_text(&block.attrs, "subtitle"),
            foot: attr_text(&block.attrs, "foot"),
            attribution: attr_text(&block.attrs, "attribution"),
            class_name: attr_class_name(&block.attrs),
            children: block
                .inner
                .as_deref()
                .map(compile_fragment_nodes)
                .unwrap_or_default(),
            source_mdx: block.source.clone(),
        },
        "Metric" => {
            let fallback_value = block
                .inner
                .as_deref()
                .map(clean_scene_text)
                .filter(|value| !value.is_empty());
            SceneNode::Metric {
                label: attr_text(&block.attrs, "label"),
                value: attr_text(&block.attrs, "value").or(fallback_value),
                hint: attr_text(&block.attrs, "hint"),
                class_name: attr_class_name(&block.attrs),
                source_mdx: block.source.clone(),
            }
        }
        "Chart" => {
            let inline_data = block
                .inner
                .as_deref()
                .map(clean_scene_text)
                .filter(|value| !value.is_empty());
            SceneNode::Chart {
                chart_type: attr_text(&block.attrs, "type").unwrap_or_else(|| "bar".to_string()),
                title: attr_text(&block.attrs, "title"),
                tone: attr_text(&block.attrs, "tone"),
                value_suffix: attr_text(&block.attrs, "suffix")
                    .or_else(|| attr_text(&block.attrs, "valueSuffix")),
                highlight: attr_text(&block.attrs, "highlight"),
                data: attr_text(&block.attrs, "data")
                    .or_else(|| attr_text(&block.attrs, "items"))
                    .or(inline_data)
                    .map(|value| parse_chart_data(&value))
                    .unwrap_or_default(),
                class_name: attr_class_name(&block.attrs),
                source_mdx: block.source.clone(),
            }
        }
        "Caption" => SceneNode::Text {
            role: "caption".to_string(),
            text: block
                .inner
                .as_deref()
                .map(clean_scene_text)
                .unwrap_or_default(),
            level: None,
            class_name: attr_class_name(&block.attrs),
        },
        "Kicker" => SceneNode::Text {
            role: "kicker".to_string(),
            text: block
                .inner
                .as_deref()
                .map(clean_scene_text)
                .unwrap_or_default(),
            level: None,
            class_name: attr_class_name(&block.attrs),
        },
        "Takeaway" => {
            let level = attr_text(&block.attrs, "as").and_then(|value| match value.trim() {
                "h1" => Some(1),
                "h2" => Some(2),
                "h3" => Some(3),
                _ => None,
            });
            SceneNode::Text {
                role: "takeaway".to_string(),
                text: block
                    .inner
                    .as_deref()
                    .map(clean_scene_text)
                    .unwrap_or_default(),
                level,
                class_name: attr_class_name(&block.attrs),
            }
        }
        "Pill" => SceneNode::Pill {
            tone: attr_text(&block.attrs, "tone"),
            text: block
                .inner
                .as_deref()
                .map(clean_scene_text)
                .unwrap_or_default(),
            class_name: attr_class_name(&block.attrs),
        },
        "Rule" => SceneNode::Rule {
            class_name: attr_class_name(&block.attrs),
        },
        "Arrow" => SceneNode::Arrow {
            direction: attr_text(&block.attrs, "direction"),
            tone: attr_text(&block.attrs, "tone"),
            label: attr_text(&block.attrs, "label").or_else(|| {
                block
                    .inner
                    .as_deref()
                    .map(clean_scene_text)
                    .filter(|value| !value.is_empty())
            }),
            class_name: attr_class_name(&block.attrs),
            source_mdx: block.source.clone(),
        },
        _ => SceneNode::Raw {
            format: "mdx".to_string(),
            text: block.source.clone(),
        },
    }
}

fn extract_media_nodes(fragment: &str) -> Vec<SceneNode> {
    let mut nodes = Vec::<SceneNode>::new();

    for captures in markdown_image_capture_re().captures_iter(fragment) {
        let src = captures
            .get(2)
            .map(|item| sanitize_markdown_target(item.as_str()))
            .unwrap_or_default();
        if src.is_empty() {
            continue;
        }
        let alt = captures
            .get(1)
            .map(|item| item.as_str().trim().to_string())
            .filter(|value| !value.is_empty());
        nodes.push(SceneNode::Media {
            media_kind: "image".to_string(),
            src,
            alt,
        });
    }

    for captures in html_image_block_re().captures_iter(fragment) {
        let attrs = captures
            .get(1)
            .map(|item| parse_component_attrs(item.as_str()))
            .unwrap_or_default();
        let Some(src) = attr_text(&attrs, "src") else {
            continue;
        };
        nodes.push(SceneNode::Media {
            media_kind: "image".to_string(),
            src,
            alt: attr_text(&attrs, "alt"),
        });
    }

    for captures in html_video_block_re().captures_iter(fragment) {
        let attrs = captures
            .get(1)
            .map(|item| parse_component_attrs(item.as_str()))
            .unwrap_or_default();
        let Some(src) = attr_text(&attrs, "src").or_else(|| attr_text(&attrs, "poster")) else {
            continue;
        };
        nodes.push(SceneNode::Media {
            media_kind: "video".to_string(),
            src,
            alt: None,
        });
    }

    nodes
}

fn extract_heading_nodes(fragment: &str) -> (Vec<SceneNode>, String) {
    let mut nodes = Vec::<SceneNode>::new();

    for captures in html_heading_block_re().captures_iter(fragment) {
        let level = captures
            .get(1)
            .and_then(|item| item.as_str().parse::<u8>().ok());
        let text = captures
            .get(2)
            .map(|item| clean_heading_text(item.as_str()))
            .unwrap_or_default();
        if !text.is_empty() {
            nodes.push(SceneNode::Text {
                role: "heading".to_string(),
                text,
                level,
                class_name: None,
            });
        }
    }

    let without_html = html_heading_block_re()
        .replace_all(fragment, "\n")
        .to_string();

    for captures in markdown_heading_capture_re().captures_iter(&without_html) {
        let level = captures.get(1).map(|item| item.as_str().len() as u8);
        let text = captures
            .get(2)
            .map(|item| clean_heading_text(item.as_str()))
            .unwrap_or_default();
        if !text.is_empty() {
            nodes.push(SceneNode::Text {
                role: "heading".to_string(),
                text,
                level,
                class_name: None,
            });
        }
    }

    let remainder = markdown_heading_capture_re()
        .replace_all(&without_html, "\n")
        .to_string();

    (nodes, remainder)
}

fn extract_list_nodes(fragment: &str) -> (Vec<SceneNode>, String) {
    let mut nodes = Vec::<SceneNode>::new();

    for captures in html_list_block_re().captures_iter(fragment) {
        let ordered = captures
            .get(1)
            .map(|item| item.as_str().eq_ignore_ascii_case("ol"))
            .unwrap_or(false);
        let body = captures
            .get(2)
            .map(|item| item.as_str())
            .unwrap_or_default();
        let items: Vec<_> = html_list_item_re()
            .captures_iter(body)
            .filter_map(|item| item.get(1).map(|entry| clean_scene_text(entry.as_str())))
            .filter(|item| !item.is_empty())
            .collect();
        if !items.is_empty() {
            nodes.push(SceneNode::List { ordered, items });
        }
    }

    let without_html_lists = html_list_block_re().replace_all(fragment, "\n").to_string();
    let mut markdown_items = Vec::<String>::new();
    let mut ordered = true;
    for captures in markdown_list_item_capture_re().captures_iter(&without_html_lists) {
        let marker = captures
            .get(1)
            .map(|item| item.as_str())
            .unwrap_or_default();
        if !marker.ends_with('.') {
            ordered = false;
        }
        let item_text = captures
            .get(2)
            .map(|item| clean_scene_text(item.as_str()))
            .unwrap_or_default();
        if !item_text.is_empty() {
            markdown_items.push(item_text);
        }
    }
    if !markdown_items.is_empty() {
        nodes.push(SceneNode::List {
            ordered,
            items: markdown_items,
        });
    }

    let remainder = markdown_list_item_capture_re()
        .replace_all(&without_html_lists, "\n")
        .to_string();

    (nodes, remainder)
}

fn compile_plain_fragment_nodes(fragment: &str) -> Vec<SceneNode> {
    let trimmed = fragment.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut nodes = extract_media_nodes(trimmed);

    let without_media = markdown_image_capture_re()
        .replace_all(trimmed, " ")
        .to_string();
    let without_media = html_image_block_re()
        .replace_all(&without_media, " ")
        .to_string();
    let without_media = html_video_block_re()
        .replace_all(&without_media, " ")
        .to_string();

    let (mut heading_nodes, after_headings) = extract_heading_nodes(&without_media);
    nodes.append(&mut heading_nodes);

    let (mut list_nodes, after_lists) = extract_list_nodes(&after_headings);
    nodes.append(&mut list_nodes);

    let text = clean_scene_text(&after_lists);
    if !text.is_empty() {
        nodes.push(SceneNode::Text {
            role: "paragraph".to_string(),
            text,
            level: None,
            class_name: None,
        });
    }

    if nodes.is_empty() {
        return vec![SceneNode::Raw {
            format: "mdx".to_string(),
            text: trimmed.to_string(),
        }];
    }

    nodes
}

fn compile_loose_nodes(fragment: &str) -> Vec<SceneNode> {
    let trimmed = fragment.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if should_preserve_raw_html_fragment(trimmed) {
        return vec![SceneNode::Raw {
            format: "html".to_string(),
            text: trimmed.to_string(),
        }];
    }

    let code_blocks = extract_code_fence_blocks(trimmed);
    if code_blocks.is_empty() {
        return compile_plain_fragment_nodes(trimmed);
    }

    let mut nodes = Vec::<SceneNode>::new();
    let mut cursor = 0usize;
    for block in code_blocks {
        nodes.extend(compile_plain_fragment_nodes(&trimmed[cursor..block.start]));
        nodes.push(SceneNode::CodeBlock {
            language: block.language,
            code: block.code,
        });
        cursor = block.end;
    }
    nodes.extend(compile_plain_fragment_nodes(&trimmed[cursor..]));
    nodes
}

fn compile_fragment_nodes(fragment: &str) -> Vec<SceneNode> {
    let trimmed = fragment.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let block_names = [
        "Stack", "Row", "Grid", "Card", "Panel", "Metric", "Chart", "Caption", "Kicker",
        "Takeaway", "Callout", "PillRow", "Pill", "Quote", "Rule",
    ];
    let blocks = extract_component_blocks(trimmed, &block_names);
    if blocks.is_empty() {
        return compile_loose_nodes(trimmed);
    }

    let mut nodes = Vec::<SceneNode>::new();
    let mut cursor = 0usize;
    for block in blocks {
        nodes.extend(compile_loose_nodes(&trimmed[cursor..block.start]));
        nodes.push(compile_component_node(&block));
        cursor = block.end;
    }
    nodes.extend(compile_loose_nodes(&trimmed[cursor..]));
    nodes
}

fn compile_canvas_node(block: &ComponentBlock) -> SceneNode {
    let cols = attr_usize(&block.attrs, "cols").unwrap_or(50);
    let rows = attr_usize(&block.attrs, "rows").unwrap_or(25);
    let gap = attr_text(&block.attrs, "gap");
    let class_name = attr_class_name(&block.attrs);
    let inner = block.inner.as_deref().unwrap_or_default();
    let area_blocks = extract_component_blocks(inner, &["Area"]);
    let children = if area_blocks.is_empty() {
        compile_fragment_nodes(inner)
    } else {
        let mut nodes = Vec::<SceneNode>::new();
        let mut cursor = 0usize;
        for area_block in &area_blocks {
            nodes.extend(compile_fragment_nodes(&inner[cursor..area_block.start]));
            nodes.push(compile_area_node(area_block));
            cursor = area_block.end;
        }
        nodes.extend(compile_fragment_nodes(&inner[cursor..]));
        nodes
    };

    SceneNode::Canvas {
        cols,
        rows,
        gap,
        class_name,
        children,
        source_mdx: block.source.clone(),
    }
}

fn compile_area_node(block: &ComponentBlock) -> SceneNode {
    let inner = block.inner.as_deref().unwrap_or_default();
    SceneNode::Area {
        x: attr_usize(&block.attrs, "x").unwrap_or(1),
        y: attr_usize(&block.attrs, "y").unwrap_or(1),
        w: attr_usize(&block.attrs, "w").unwrap_or(1),
        h: attr_usize(&block.attrs, "h").unwrap_or(1),
        layer: attr_usize(&block.attrs, "layer"),
        gap: attr_text(&block.attrs, "gap"),
        align: attr_text(&block.attrs, "align"),
        justify: attr_text(&block.attrs, "justify"),
        class_name: attr_class_name(&block.attrs),
        children: compile_fragment_nodes(inner),
        source_mdx: block.source.clone(),
    }
}

fn infer_scene_layout(slide: &str) -> SceneLayout {
    if let Some(canvas_block) = next_component_block(slide, 0, &["Canvas"]) {
        return SceneLayout {
            kind: "canvas".to_string(),
            cols: attr_usize(&canvas_block.attrs, "cols").or(Some(50)),
            rows: attr_usize(&canvas_block.attrs, "rows").or(Some(25)),
            gap: attr_text(&canvas_block.attrs, "gap"),
        };
    }

    let component_counts = component_counts_for_slide(slide);
    let kind = if component_counts.get("Grid").copied().unwrap_or(0) > 0 {
        "grid"
    } else if component_counts.get("Row").copied().unwrap_or(0) > 0 {
        "row"
    } else if component_counts.get("Stack").copied().unwrap_or(0) > 0 {
        "stack"
    } else if LAYOUT_COMPONENT_NAMES
        .iter()
        .any(|name| component_counts.get(*name).copied().unwrap_or(0) > 0)
    {
        "structured-layout"
    } else {
        "flow"
    };

    SceneLayout {
        kind: kind.to_string(),
        cols: None,
        rows: None,
        gap: None,
    }
}

pub(crate) fn compile_slide_nodes(slide: &str) -> Vec<SceneNode> {
    if let Some(canvas_block) = next_component_block(slide, 0, &["Canvas"]) {
        let mut nodes = Vec::<SceneNode>::new();
        nodes.extend(compile_fragment_nodes(&slide[..canvas_block.start]));
        nodes.push(compile_canvas_node(&canvas_block));
        nodes.extend(compile_fragment_nodes(&slide[canvas_block.end..]));
        return nodes;
    }
    compile_fragment_nodes(slide)
}

fn deck_class_name_from_body(body: &str) -> Option<String> {
    let main_open = Regex::new(r#"(?is)<main(?P<attrs>[^>]*)>"#).unwrap();
    let captures = main_open.captures(body)?;
    let attrs = captures.name("attrs")?.as_str();
    let parsed = parse_component_attrs(attrs);
    let class_name = attr_text(&parsed, "className").or_else(|| attr_text(&parsed, "class"))?;
    let normalized = class_name
        .split_whitespace()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn should_preserve_raw_html_fragment(fragment: &str) -> bool {
    if fragment.contains("```") {
        return false;
    }

    if fragment.contains("className=") || fragment.contains("class=") {
        return true;
    }

    let html_block_tags = [
        "<table",
        "<thead",
        "<tbody",
        "<tr",
        "<td",
        "<th",
        "<figure",
        "<figcaption",
    ];
    fragment.trim_start().starts_with('<')
        && html_block_tags.iter().any(|tag| fragment.contains(tag))
}

pub(crate) fn validate_scene_slide_contract(slide: &str, index: usize) -> Result<(), String> {
    let component_counts = component_counts_for_slide(slide);
    if !uses_spatial_canvas(&component_counts) {
        return Err(format!(
            "Slide {} is not on the 2.0 spatial contract. Use one Canvas with Area regions before compiling a scene.",
            index + 1
        ));
    }
    if split_class_re().is_match(slide) {
        return Err(format!(
            "Slide {} still uses legacy `split` layout. Replace it with Canvas and Area before compiling a scene.",
            index + 1
        ));
    }
    Ok(())
}

pub(crate) fn load_project_scene_source(project_path: &Path) -> Result<ProjectSceneSource, String> {
    let canonical_project = normalize_existing_project_directory(&path_to_string(project_path))?;
    let source = read_page_mdx(&canonical_project)?;
    let (frontmatter, body) = extract_frontmatter(&source);
    let slides = extract_slides(&body);
    let metadata = frontmatter.unwrap_or_default();

    for (index, slide) in slides.iter().enumerate() {
        validate_scene_slide_contract(slide, index)?;
    }

    Ok(ProjectSceneSource {
        path: path_to_string(&canonical_project),
        project: metadata.get("project").cloned(),
        title: metadata.get("title").cloned(),
        subtitle: metadata.get("subtitle").cloned(),
        date: metadata.get("date").cloned(),
        deck_class_name: deck_class_name_from_body(&body),
        slides,
    })
}

fn build_scene_slide(slide: &str, index: usize) -> SceneSlide {
    SceneSlide {
        index,
        title: slide_title_for(slide, index),
        layout: infer_scene_layout(slide),
        nodes: compile_slide_nodes(slide),
        source_mdx: slide.trim().to_string(),
    }
}

fn build_scene_slide_manifest(slide: &str, index: usize) -> SceneSlideManifest {
    SceneSlideManifest {
        index,
        title: slide_title_for(slide, index),
        layout: infer_scene_layout(slide),
    }
}

pub(super) fn build_project_scene_manifest_from_source(
    source: &ProjectSceneSource,
) -> ProjectSceneManifest {
    ProjectSceneManifest {
        path: source.path.clone(),
        project: source.project.clone(),
        title: source.title.clone(),
        subtitle: source.subtitle.clone(),
        date: source.date.clone(),
        deck_class_name: source.deck_class_name.clone(),
        slide_count: source.slides.len(),
        slides: source
            .slides
            .iter()
            .enumerate()
            .map(|(index, slide)| build_scene_slide_manifest(slide, index))
            .collect(),
    }
}

pub(super) fn try_build_scene_slide(slides: &[String], index: usize) -> Result<SceneSlide, String> {
    let slide = slides.get(index).ok_or_else(|| {
        format!(
            "Slide {} is out of range for this deck ({} slides).",
            index + 1,
            slides.len()
        )
    })?;

    catch_unwind(AssertUnwindSafe(|| build_scene_slide(slide, index))).map_err(|panic| match panic
        .downcast_ref::<String>(
    ) {
        Some(message) => message.clone(),
        None => match panic.downcast_ref::<&str>() {
            Some(message) => (*message).to_string(),
            None => format!("Slide {} panicked during scene compilation.", index + 1),
        },
    })
}

pub(crate) fn build_project_scene(project_path: &Path) -> Result<ProjectScene, String> {
    let ProjectSceneSource {
        path,
        project,
        title,
        subtitle,
        date,
        deck_class_name,
        slides,
    } = load_project_scene_source(project_path)?;

    let slide_count = slides.len();
    let compiled_slides = slides
        .iter()
        .enumerate()
        .map(|(index, slide)| build_scene_slide(slide, index))
        .collect();

    Ok(ProjectScene {
        path,
        project,
        title,
        subtitle,
        date,
        deck_class_name,
        slide_count,
        slides: compiled_slides,
    })
}

pub(crate) fn build_project_scene_manifest(
    project_path: &Path,
) -> Result<ProjectSceneManifest, String> {
    let source = load_project_scene_source(project_path)?;
    Ok(build_project_scene_manifest_from_source(&source))
}

pub(crate) fn build_project_scene_slide(
    project_path: &Path,
    index: usize,
) -> Result<SceneSlide, String> {
    let ProjectSceneSource { slides, .. } = load_project_scene_source(project_path)?;
    let slide = slides.get(index).ok_or_else(|| {
        format!(
            "Slide {} is out of range for this deck ({} slides).",
            index + 1,
            slides.len()
        )
    })?;
    Ok(build_scene_slide(slide, index))
}
