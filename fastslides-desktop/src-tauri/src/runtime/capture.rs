use crate::codex::optional_codex_home_directory;
use crate::config::path_to_string;
use crate::constants::DEFAULT_PREVIEW_BASE_URL;
use crate::scene::load_project_scene_source;
use regex::Regex;
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    env, fs,
    net::{TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use url::{form_urlencoded::Serializer as UrlQuerySerializer, Url};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SlideCaptureResponse {
    ok: bool,
    path: String,
    slide: usize,
    output_dir: String,
    image_path: String,
    preview_url: String,
}

fn preview_base_url() -> String {
    super::env_string(
        &[
            "FASTSLIDES_PREVIEW_URL",
            "NEXT_PUBLIC_FASTSLIDES_PREVIEW_URL",
        ],
        DEFAULT_PREVIEW_BASE_URL,
    )
    .trim_end_matches('/')
    .to_string()
}

pub(crate) fn is_preview_url_reachable(preview_url: &str) -> bool {
    let parsed = match Url::parse(preview_url) {
        Ok(url) => url,
        Err(_) => return false,
    };

    let host = match parsed.host_str() {
        Some(host) => host,
        None => return false,
    };
    let port = parsed.port_or_known_default().unwrap_or(80);
    let addr = format!("{host}:{port}");
    let socket_addr = match addr.to_socket_addrs() {
        Ok(mut addrs) => addrs.next(),
        Err(_) => None,
    };

    match socket_addr {
        Some(socket) => TcpStream::connect_timeout(&socket, Duration::from_millis(1200)).is_ok(),
        None => false,
    }
}

fn wait_for_preview_url(preview_url: &str, attempts: usize) -> bool {
    for attempt in 0..attempts {
        if is_preview_url_reachable(preview_url) {
            return true;
        }
        if attempt + 1 < attempts {
            thread::sleep(Duration::from_millis(350));
        }
    }
    false
}

pub(crate) fn parse_line_column(message: &str) -> (Option<u64>, Option<u64>) {
    if let Some(captures) = Regex::new(r":(\d+):(\d+)")
        .ok()
        .and_then(|re| re.captures(message))
    {
        let line = captures.get(1).and_then(|m| m.as_str().parse::<u64>().ok());
        let column = captures.get(2).and_then(|m| m.as_str().parse::<u64>().ok());
        return (line, column);
    }

    if let Some(captures) = Regex::new(r"(?i)\bline\s+(\d+)\b")
        .ok()
        .and_then(|re| re.captures(message))
    {
        let line = captures.get(1).and_then(|m| m.as_str().parse::<u64>().ok());
        return (line, None);
    }

    (None, None)
}

pub(crate) fn build_preview_url_for_path(project_path: &str) -> String {
    let mut serializer = UrlQuerySerializer::new(String::new());
    serializer.append_pair("deckPath", project_path);
    let query = serializer.finish();
    format!("{}/?{query}", preview_base_url())
}

pub(crate) fn build_slide_preview_url(preview_url: &str, slide: usize) -> Result<String, String> {
    if slide == 0 {
        return Err("Slide number must be 1 or greater.".to_string());
    }

    let parsed = Url::parse(preview_url)
        .map_err(|error| format!("Failed to parse preview URL `{preview_url}`: {error}"))?;
    let mut query: HashMap<String, String> = parsed.query_pairs().into_owned().collect();
    query.insert("slide".to_string(), slide.to_string());
    query.insert("presenter".to_string(), "1".to_string());

    let mut serializer = UrlQuerySerializer::new(String::new());
    let mut entries: Vec<_> = query.into_iter().collect();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    for (key, value) in entries {
        serializer.append_pair(key.as_str(), value.as_str());
    }
    let query = serializer.finish();
    Ok(parsed
        .join(format!("?{query}").as_str())
        .map_err(|error| format!("Failed to build slide preview URL: {error}"))?
        .to_string())
}

fn default_slide_capture_output_dir(project_path: &Path) -> PathBuf {
    if let Some(projects_dir) = project_path.parent() {
        if projects_dir
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value == "projects")
        {
            if let Some(repo_root) = projects_dir.parent() {
                return repo_root
                    .join("output")
                    .join("playwright")
                    .join("fastslides-captures");
            }
        }
    }

    if let Ok(current_dir) = env::current_dir() {
        if current_dir
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value == "fastslides-desktop")
        {
            if let Some(repo_root) = current_dir.parent() {
                return repo_root
                    .join("output")
                    .join("playwright")
                    .join("fastslides-captures");
            }
        }
        return current_dir
            .join("output")
            .join("playwright")
            .join("fastslides-captures");
    }

    env::temp_dir().join("fastslides-captures")
}

fn resolve_slide_capture_output_dir(project_path: &Path, output_dir: Option<&str>) -> PathBuf {
    let trimmed = output_dir.map(str::trim).unwrap_or_default();
    if trimmed.is_empty() {
        return default_slide_capture_output_dir(project_path);
    }
    PathBuf::from(trimmed)
}

fn resolve_playwright_cli_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(explicit) = env::var("FASTSLIDES_PLAYWRIGHT_CLI") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            candidates.push(PathBuf::from(trimmed));
        }
    }

    if let Some(codex_home) = optional_codex_home_directory() {
        candidates.push(codex_home.join("skills/playwright/scripts/playwright_cli.sh"));
    }

    if let Ok(home) = env::var("HOME") {
        if !home.trim().is_empty() {
            candidates.push(
                PathBuf::from(home).join(".agents/skills/playwright/scripts/playwright_cli.sh"),
            );
        }
    }

    candidates.into_iter().find(|candidate| candidate.is_file())
}

pub(crate) fn build_subprocess_search_path(base: Option<&std::ffi::OsStr>) -> String {
    let mut entries: Vec<PathBuf> = base.map(env::split_paths).into_iter().flatten().collect();
    for extra in [
        "/opt/homebrew/bin",
        "/opt/homebrew/sbin",
        "/usr/local/bin",
        "/usr/local/sbin",
    ] {
        let candidate = PathBuf::from(extra);
        if entries.iter().all(|existing| existing != &candidate) {
            entries.push(candidate);
        }
    }
    env::join_paths(entries)
        .ok()
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.trim().is_empty())
        .or_else(|| base.map(|value| value.to_string_lossy().into_owned()))
        .unwrap_or_default()
}

pub(crate) fn collect_png_artifacts(
    output_dir: &Path,
) -> Result<Vec<(PathBuf, SystemTime)>, String> {
    let mut artifacts = Vec::new();
    let mut pending_dirs = VecDeque::from([output_dir.to_path_buf()]);

    while let Some(next_dir) = pending_dirs.pop_front() {
        let entries = fs::read_dir(&next_dir).map_err(|error| {
            format!(
                "Failed to read capture output directory `{}`: {error}",
                next_dir.display()
            )
        })?;

        for entry in entries {
            let entry = entry.map_err(|error| {
                format!(
                    "Failed to read capture artifact entry in `{}`: {error}",
                    next_dir.display()
                )
            })?;
            let path = entry.path();
            if path.is_dir() {
                pending_dirs.push_back(path);
                continue;
            }
            let is_png = path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("png"));
            if !is_png {
                continue;
            }
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(UNIX_EPOCH);
            artifacts.push((path, modified));
        }
    }

    Ok(artifacts)
}

pub(crate) fn pick_slide_capture_artifact(
    before: &HashSet<PathBuf>,
    after: &[(PathBuf, SystemTime)],
) -> Option<PathBuf> {
    let mut new_paths: Vec<_> = after
        .iter()
        .filter(|(path, _)| !before.contains(path))
        .cloned()
        .collect();
    new_paths.sort_by(|left, right| right.1.cmp(&left.1));
    if let Some((path, _)) = new_paths.first() {
        return Some(path.clone());
    }

    let mut fallback = after.to_vec();
    fallback.sort_by(|left, right| right.1.cmp(&left.1));
    fallback.first().map(|(path, _)| path.clone())
}

fn run_playwright_cli_command(
    pwcli: &Path,
    session_name: &str,
    output_dir: &Path,
    args: &[String],
) -> Result<String, String> {
    let mut command = Command::new(pwcli);
    command.current_dir(output_dir);
    command.arg("--session").arg(session_name);
    command.env(
        "PATH",
        build_subprocess_search_path(env::var_os("PATH").as_deref()),
    );
    for arg in args {
        command.arg(arg);
    }

    let output = command.output().map_err(|error| {
        format!(
            "Failed to launch Playwright CLI `{}`: {error}",
            pwcli.display()
        )
    })?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("exit status {}", output.status)
    };
    Err(format!("Playwright command failed: {detail}"))
}

fn wait_for_slide_capture_ready(
    pwcli: &Path,
    session_name: &str,
    output_dir: &Path,
    target_slide_index: usize,
) -> Result<(), String> {
    let readiness_eval = format!(
        "() => {{
  const stage = document.querySelector('.preview-stage.presenter-mode');
  const deck = document.querySelector('.embedded-preview-deck.embedded-preview-single');
  const activeSlide = document.querySelector('.embedded-preview-deck .slide[data-active=\"true\"]');
  if (!stage || !deck || !activeSlide) {{
    return 'pending';
  }}
  return activeSlide.getAttribute('data-slide-index') === '{}' ? 'ready' : 'pending';
}}",
        target_slide_index
    );

    for _attempt in 0..30 {
        let output = run_playwright_cli_command(
            pwcli,
            session_name,
            output_dir,
            &["eval".to_string(), readiness_eval.clone()],
        )?;
        if output.contains("\"ready\"") {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(350));
    }

    Err(format!(
        "Timed out waiting for slide {} to become active in preview.",
        target_slide_index + 1
    ))
}

pub(crate) fn capture_slide_image_for_project(
    project_path: &Path,
    slide: Option<usize>,
    output_dir: Option<&str>,
    headed: bool,
) -> Result<SlideCaptureResponse, String> {
    let source = load_project_scene_source(project_path)?;
    if source.slides.is_empty() {
        return Err("Project does not contain any slides to capture.".to_string());
    }

    let slide_number = slide.unwrap_or(1);
    if slide_number == 0 {
        return Err("Slide number must be 1 or greater.".to_string());
    }
    if slide_number > source.slides.len() {
        return Err(format!(
            "Requested slide {} but project only has {} slides.",
            slide_number,
            source.slides.len()
        ));
    }

    let preview_url = build_slide_preview_url(
        build_preview_url_for_path(source.path.as_str()).as_str(),
        slide_number,
    )?;
    if !wait_for_preview_url(preview_url.as_str(), 3) {
        return Err(format!(
            "Preview URL is not reachable for slide capture: {}",
            preview_url
        ));
    }

    let pwcli = resolve_playwright_cli_path().ok_or_else(|| {
        "Playwright wrapper script not found. Set FASTSLIDES_PLAYWRIGHT_CLI or install the playwright skill.".to_string()
    })?;

    let output_dir = resolve_slide_capture_output_dir(project_path, output_dir);
    fs::create_dir_all(&output_dir).map_err(|error| {
        format!(
            "Failed to create slide capture output directory `{}`: {error}",
            output_dir.display()
        )
    })?;

    let before = collect_png_artifacts(&output_dir)?;
    let before_paths: HashSet<PathBuf> = before.iter().map(|(path, _)| path.clone()).collect();
    let session_name = format!(
        "fs-{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        slide_number
    );
    let target_slide_index = slide_number - 1;

    let open_args = if headed {
        vec![
            "open".to_string(),
            preview_url.clone(),
            "--headed".to_string(),
        ]
    } else {
        vec!["open".to_string(), preview_url.clone()]
    };
    let capture_result = (|| -> Result<(), String> {
        let _ = run_playwright_cli_command(&pwcli, &session_name, &output_dir, &open_args)?;
        run_playwright_cli_command(
            &pwcli,
            &session_name,
            &output_dir,
            &["resize".to_string(), "1920".to_string(), "1080".to_string()],
        )?;
        wait_for_slide_capture_ready(&pwcli, &session_name, &output_dir, target_slide_index)?;
        run_playwright_cli_command(
            &pwcli,
            &session_name,
            &output_dir,
            &["snapshot".to_string()],
        )?;
        run_playwright_cli_command(
            &pwcli,
            &session_name,
            &output_dir,
            &["screenshot".to_string()],
        )?;
        Ok(())
    })();

    if let Err(close_error) =
        run_playwright_cli_command(&pwcli, &session_name, &output_dir, &["close".to_string()])
    {
        log::warn!(
            "Failed to close Playwright session `{}`: {}",
            session_name,
            close_error
        );
    }

    capture_result?;

    let after = collect_png_artifacts(&output_dir)?;
    let artifact = pick_slide_capture_artifact(&before_paths, &after).ok_or_else(|| {
        format!(
            "Screenshot command completed but no PNG artifact was found in `{}`.",
            output_dir.display()
        )
    })?;
    let image_path = fs::canonicalize(&artifact).unwrap_or(artifact);
    let output_dir = fs::canonicalize(&output_dir).unwrap_or(output_dir);

    Ok(SlideCaptureResponse {
        ok: true,
        path: source.path,
        slide: slide_number,
        output_dir: path_to_string(&output_dir),
        image_path: path_to_string(&image_path),
        preview_url,
    })
}
