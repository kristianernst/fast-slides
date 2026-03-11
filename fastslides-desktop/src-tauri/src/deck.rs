use regex::Regex;
use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    sync::OnceLock,
};

const PROJECT_NAME_PATTERN: &str = r"^[A-Za-z0-9._-]+$";
const SURFACE_COMPONENT_NAMES: &[&str] = &["Card", "Panel", "Callout", "Quote"];
pub(crate) const STRUCTURED_COMPONENT_NAMES: &[&str] = &[
    "Stack", "Row", "Grid", "Canvas", "Area", "Card", "Panel", "Callout", "Metric", "Caption",
    "Kicker", "Takeaway", "Chart", "PillRow", "Pill", "Quote", "Rule",
];
pub(crate) const LAYOUT_COMPONENT_NAMES: &[&str] =
    &["Stack", "Row", "Grid", "Canvas", "Area", "PillRow"];

pub(crate) fn project_name_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(PROJECT_NAME_PATTERN).expect("invalid project name regex"))
}

fn slide_start_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?i)<section\s+className=["']slide["']\s*>"#).expect("invalid slide regex")
    })
}

pub(crate) fn markdown_link_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"!\[[^\]]*\]\(([^)]+)\)|\[[^\]]*\]\(([^)]+)\)"#)
            .expect("invalid mdx link regex")
    })
}

pub(crate) fn attr_link_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?:src|href|poster)\s*=\s*["']([^"']+)["']"#)
            .expect("invalid attr link regex")
    })
}

fn word_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"[A-Za-z0-9][A-Za-z0-9'./-]*"#).expect("invalid word regex"))
}

pub(crate) fn bullet_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?m)^\s*(?:[-*+]\s+|\d+\.\s+)"#).expect("invalid bullet regex"))
}

pub(crate) fn import_export_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*(import|export)\s+"#).expect("invalid import/export regex")
    })
}

pub(crate) fn use_client_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*["']use client["']\s*;?\s*$"#).expect("invalid use-client regex")
    })
}

pub(crate) fn html_tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"<[^>]+>"#).expect("invalid html tag regex"))
}

fn frontmatter_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)\A---\s*\n(.*?)\n---\s*(?:\n|$)"#).expect("invalid frontmatter regex")
    })
}

fn frontmatter_line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*([A-Za-z0-9_-]+)\s*:\s*(.*?)\s*$"#)
            .expect("invalid frontmatter line regex")
    })
}

fn markdown_heading_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s{0,3}#{1,3}\s+(.+?)\s*$"#).expect("invalid heading regex")
    })
}

fn html_heading_capture_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?is)<h[1-3][^>]*>(.*?)</h[1-3]>"#).expect("invalid html heading regex")
    })
}

fn takeaway_capture_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?is)<Takeaway\b[^>]*>(.*?)</Takeaway>"#)
            .expect("invalid takeaway heading regex")
    })
}

fn mdx_component_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"<(Stack|Row|Grid|Canvas|Area|Card|Panel|Callout|Metric|Chart|Caption|Kicker|Takeaway|PillRow|Pill|Quote|Rule|Arrow)\b"#,
        )
        .expect("invalid mdx component regex")
    })
}

pub(crate) fn split_class_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"className\s*=\s*["'][^"']*\bsplit\b[^"']*["']"#)
            .expect("invalid split class regex")
    })
}

fn image_ref_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?i)<img\b|!\[[^\]]*\]\("#).expect("invalid image regex"))
}

fn video_ref_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?i)<video\b|poster\s*="#).expect("invalid video regex"))
}

pub(crate) fn sanitize_markdown_target(raw: &str) -> String {
    let trimmed = raw.trim().trim_matches('<').trim_matches('>');
    if let Some(index) = trimmed.find(' ') {
        return trimmed[..index].to_string();
    }
    trimmed.to_string()
}

pub(crate) fn local_asset_path(raw: &str) -> Option<String> {
    let value = sanitize_markdown_target(raw);
    if value.is_empty() {
        return None;
    }

    let lower = value.to_ascii_lowercase();
    if value.starts_with('#')
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("data:")
        || lower.starts_with("blob:")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
    {
        return None;
    }

    let no_hash = value.split('#').next().unwrap_or_default();
    let no_query = no_hash.split('?').next().unwrap_or_default();
    if no_query.is_empty() {
        return None;
    }

    let normalized = no_query.replace('\\', "/");
    if normalized.starts_with('/') {
        let allowed = normalized == "/assets"
            || normalized == "/images"
            || normalized == "/media"
            || normalized == "/data"
            || normalized.starts_with("/assets/")
            || normalized.starts_with("/images/")
            || normalized.starts_with("/media/")
            || normalized.starts_with("/data/");
        if !allowed {
            return None;
        }
        return Some(normalized.trim_start_matches('/').to_string());
    }

    Some(normalized)
}

pub(crate) fn resolve_relative_path(base_dir: &Path, relative: &str) -> Option<PathBuf> {
    let mut output = base_dir.to_path_buf();

    for component in Path::new(relative).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => output.push(part),
            Component::ParentDir => {
                if !output.pop() {
                    return None;
                }
                if !output.starts_with(base_dir) {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(output)
}

pub(crate) fn mime_type_for_path(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "avif" => "image/avif",
        _ => "application/octet-stream",
    }
}

pub(crate) fn extract_slides(source: &str) -> Vec<String> {
    let matches: Vec<_> = slide_start_re().find_iter(source).collect();
    if matches.is_empty() {
        return Vec::new();
    }

    let mut slides = Vec::new();
    for (index, hit) in matches.iter().enumerate() {
        let start = hit.end();
        let explicit_end = source[start..]
            .find("</section>")
            .map(|offset| start + offset);
        let fallback_end = if index + 1 < matches.len() {
            matches[index + 1].start()
        } else {
            source.len()
        };
        let end = explicit_end.unwrap_or(fallback_end);
        slides.push(source[start..end].to_string());
    }
    slides
}

pub(crate) fn clean_heading_text(raw: &str) -> String {
    let without_tags = html_tag_re().replace_all(raw, " ");
    without_tags
        .replace("**", " ")
        .replace("__", " ")
        .replace(['`', '*', '_'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

pub(crate) fn slide_title_for(slide: &str, index: usize) -> String {
    if let Some(captures) = markdown_heading_re().captures(slide) {
        if let Some(raw_title) = captures
            .get(1)
            .map(|item| clean_heading_text(item.as_str()))
        {
            if !raw_title.is_empty() {
                return raw_title;
            }
        }
    }

    if let Some(captures) = takeaway_capture_re().captures(slide) {
        if let Some(raw_title) = captures
            .get(1)
            .map(|item| clean_heading_text(item.as_str()))
        {
            if !raw_title.is_empty() {
                return raw_title;
            }
        }
    }

    if let Some(captures) = html_heading_capture_re().captures(slide) {
        if let Some(raw_title) = captures
            .get(1)
            .map(|item| clean_heading_text(item.as_str()))
        {
            if !raw_title.is_empty() {
                return raw_title;
            }
        }
    }

    format!("Slide {}", index + 1)
}

pub(crate) fn increment_count(counts: &mut HashMap<String, usize>, key: &str, amount: usize) {
    *counts.entry(key.to_string()).or_insert(0) += amount;
}

fn add_count_if_present(counts: &mut HashMap<String, usize>, key: &str, amount: usize) {
    if amount > 0 {
        increment_count(counts, key, amount);
    }
}

pub(crate) fn component_counts_for_slide(slide: &str) -> HashMap<String, usize> {
    let mut counts = HashMap::<String, usize>::new();

    for captures in mdx_component_re().captures_iter(slide) {
        if let Some(name) = captures.get(1).map(|item| item.as_str()) {
            increment_count(&mut counts, name, 1);
        }
    }

    let mermaid_count = slide.matches("```mermaid").count();
    add_count_if_present(&mut counts, "mermaid", mermaid_count);
    let code_fence_count = slide.matches("```").count() / 2;
    add_count_if_present(
        &mut counts,
        "code",
        code_fence_count.saturating_sub(mermaid_count),
    );
    add_count_if_present(
        &mut counts,
        "image",
        image_ref_re().find_iter(slide).count(),
    );
    add_count_if_present(
        &mut counts,
        "video",
        video_ref_re().find_iter(slide).count(),
    );

    counts
}

pub(crate) fn uses_spatial_canvas(component_counts: &HashMap<String, usize>) -> bool {
    component_counts.get("Canvas").copied().unwrap_or(0) > 0
        && component_counts.get("Area").copied().unwrap_or(0) > 0
}

pub(crate) fn inferred_archetype(
    slide: &str,
    words: usize,
    bullets: usize,
    component_counts: &HashMap<String, usize>,
) -> String {
    let has = |name: &str| component_counts.get(name).copied().unwrap_or(0) > 0;

    if has("mermaid") {
        return "diagram".to_string();
    }
    if has("Chart") {
        return "chart".to_string();
    }
    if has("code") {
        return "code-demo".to_string();
    }
    if has("Metric") {
        return "metrics".to_string();
    }
    if has("Canvas") && has("Area") {
        return "spatial-canvas".to_string();
    }
    if has("Quote") {
        return "quote".to_string();
    }
    if has("Grid") && has("Card") {
        return "card-grid".to_string();
    }
    if SURFACE_COMPONENT_NAMES
        .iter()
        .any(|name| *name != "Quote" && has(name))
    {
        return "structured-brief".to_string();
    }
    if has("Row") && words > 45 {
        return "comparison".to_string();
    }
    if (has("image") || has("video")) && words <= 90 {
        return "visual-explainer".to_string();
    }
    if words <= 40 && bullets == 0 {
        return "hero".to_string();
    }
    if bullets >= 3 {
        return "bullet-brief".to_string();
    }
    if slide.contains("<Card")
        || slide.contains("<Panel")
        || slide.contains("<Callout")
        || slide.contains("<Metric")
    {
        return "structured-brief".to_string();
    }
    "narrative".to_string()
}

pub(crate) fn words_in_text(text: &str) -> usize {
    let plain = html_tag_re().replace_all(text, " ");
    word_re().find_iter(&plain).count()
}

pub(crate) fn max_paragraph_words(text: &str) -> usize {
    let plain = html_tag_re().replace_all(text, " ");
    let mut max_words = 0usize;
    for paragraph in plain
        .split("\n\n")
        .map(|chunk| chunk.trim())
        .filter(|chunk| !chunk.is_empty())
    {
        let count = word_re().find_iter(paragraph).count();
        if count > max_words {
            max_words = count;
        }
    }
    max_words
}

pub(crate) fn normalize_frontmatter_value(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() >= 2 {
        let first = trimmed.as_bytes()[0] as char;
        let last = trimmed.as_bytes()[trimmed.len() - 1] as char;
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            let inner = &trimmed[1..trimmed.len() - 1];
            let escaped_quote = format!(r#"\{first}"#);
            return inner
                .replace("\\\\", "\\")
                .replace(escaped_quote.as_str(), first.to_string().as_str())
                .trim()
                .to_string();
        }
    }
    trimmed.to_string()
}

pub(crate) fn extract_frontmatter(source: &str) -> (Option<HashMap<String, String>>, String) {
    let Some(captures) = frontmatter_re().captures(source) else {
        return (None, source.to_string());
    };

    let Some(full_match) = captures.get(0) else {
        return (None, source.to_string());
    };
    let block = captures
        .get(1)
        .map(|item| item.as_str())
        .unwrap_or_default();

    let mut values = HashMap::<String, String>::new();
    for raw_line in block.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(parsed) = frontmatter_line_re().captures(line) {
            let key = parsed
                .get(1)
                .map(|item| item.as_str().to_ascii_lowercase())
                .unwrap_or_default();
            let value = parsed
                .get(2)
                .map(|item| normalize_frontmatter_value(item.as_str()))
                .unwrap_or_default();
            values.insert(key, value);
        }
    }

    (Some(values), source[full_match.end()..].to_string())
}
