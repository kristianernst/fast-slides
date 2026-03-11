use crate::config::{expand_user_path, path_to_string};
use regex::Regex;
use serde::Serialize;
use std::{env, fs, path::PathBuf, process::Command};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CodexInstallStatus {
    Installed,
    Updated,
    Unchanged,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CodexMcpInstallResponse {
    pub(crate) ok: bool,
    pub(crate) status: CodexInstallStatus,
    pub(crate) config_path: String,
    pub(crate) server_name: String,
    pub(crate) url: String,
}

fn ensure_zip_destination(path_str: &str) -> PathBuf {
    let path = expand_user_path(path_str);
    let has_zip_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"));
    if has_zip_extension {
        path
    } else {
        path.with_extension("zip")
    }
}

fn resolve_fastslides_skill_directory() -> Result<PathBuf, String> {
    let mut candidates = Vec::<PathBuf>::new();

    if let Ok(explicit) = env::var("FASTSLIDES_SKILL_DIR") {
        let explicit = explicit.trim();
        if !explicit.is_empty() {
            candidates.push(expand_user_path(explicit));
        }
    }

    if let Ok(home) = env::var("HOME") {
        let home_path = PathBuf::from(home);
        candidates.push(home_path.join(".agents").join("skills").join("fastslides"));
        candidates.push(home_path.join(".codex").join("skills").join("fastslides"));
    }

    let mut checked = Vec::<String>::new();
    for candidate in candidates {
        checked.push(path_to_string(&candidate));
        let skill_marker = candidate.join("SKILL.md");
        if candidate.is_dir() && skill_marker.is_file() {
            return candidate
                .canonicalize()
                .map_err(|error| format!("Failed to resolve {}: {error}", candidate.display()));
        }
    }

    Err(format!(
        "Could not locate FastSlides skill folder. Checked: {}",
        checked.join(", ")
    ))
}

pub(crate) fn export_fastslides_skill_archive(destination: String) -> Result<String, String> {
    let skill_dir = resolve_fastslides_skill_directory()?;
    let destination_path = ensure_zip_destination(destination.as_str());

    let parent = destination_path.parent().ok_or_else(|| {
        format!(
            "Destination path has no parent folder: {}",
            destination_path.display()
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Failed to create destination folder {}: {error}",
            parent.display()
        )
    })?;

    if destination_path.exists() {
        fs::remove_file(&destination_path).map_err(|error| {
            format!(
                "Failed to overwrite existing archive {}: {error}",
                destination_path.display()
            )
        })?;
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("ditto")
            .arg("-c")
            .arg("-k")
            .arg("--sequesterRsrc")
            .arg("--keepParent")
            .arg(&skill_dir)
            .arg(&destination_path)
            .status()
            .map_err(|error| format!("Failed to run ditto for skill export: {error}"))?;

        if !status.success() {
            return Err(format!(
                "Skill archive export failed with status {}.",
                status
            ));
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = skill_dir;
        return Err("Skill export is currently implemented for macOS only.".to_string());
    }

    Ok(path_to_string(&destination_path))
}

pub(crate) fn optional_codex_home_directory() -> Option<PathBuf> {
    env::var("CODEX_HOME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var("HOME")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .map(|home| PathBuf::from(home).join(".codex"))
        })
}

fn resolve_codex_home_directory() -> Result<PathBuf, String> {
    optional_codex_home_directory().ok_or_else(|| {
        "Could not resolve Codex home directory from CODEX_HOME or HOME.".to_string()
    })
}

fn resolve_codex_config_path() -> Result<PathBuf, String> {
    Ok(resolve_codex_home_directory()?.join("config.toml"))
}

fn fastslides_codex_mcp_block(server_name: &str, url: &str) -> String {
    format!("[mcp_servers.{server_name}]\nurl = \"{url}\"")
}

fn stitch_toml_sections(prefix: &str, block: &str, suffix: &str) -> String {
    let prefix = prefix.trim_end();
    let suffix = suffix.trim_start();

    match (prefix.is_empty(), suffix.is_empty()) {
        (true, true) => format!("{block}\n"),
        (true, false) => format!("{block}\n\n{suffix}\n"),
        (false, true) => format!("{prefix}\n\n{block}\n"),
        (false, false) => format!("{prefix}\n\n{block}\n\n{suffix}\n"),
    }
}

pub(crate) fn upsert_codex_mcp_server_block(
    source: &str,
    server_name: &str,
    url: &str,
) -> Result<(String, CodexInstallStatus), String> {
    let block = fastslides_codex_mcp_block(server_name, url);
    let header = format!("[mcp_servers.{server_name}]");
    let header_re = Regex::new(&format!(r"(?m)^{}\s*$", regex::escape(&header)))
        .map_err(|error| format!("Failed to build Codex config matcher: {error}"))?;
    let table_re = Regex::new(r"(?m)^\[[^\n]+\]\s*$")
        .map_err(|error| format!("Failed to build TOML section matcher: {error}"))?;

    if let Some(header_match) = header_re.find(source) {
        let after_header = &source[header_match.end()..];
        let next_table_start = table_re
            .find(after_header)
            .map(|matched| header_match.end() + matched.start())
            .unwrap_or(source.len());
        let current_block = source[header_match.start()..next_table_start].trim();
        if current_block == block {
            return Ok((
                stitch_toml_sections("", source.trim(), ""),
                CodexInstallStatus::Unchanged,
            ));
        }

        let updated = stitch_toml_sections(
            &source[..header_match.start()],
            &block,
            &source[next_table_start..],
        );
        return Ok((updated, CodexInstallStatus::Updated));
    }

    Ok((
        stitch_toml_sections(source, &block, ""),
        CodexInstallStatus::Installed,
    ))
}

pub(crate) fn install_codex_mcp_server_config() -> Result<CodexMcpInstallResponse, String> {
    let config_path = resolve_codex_config_path()?;
    let parent = config_path.parent().ok_or_else(|| {
        format!(
            "Codex config path has no parent directory: {}",
            config_path.display()
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Failed to create Codex config directory {}: {error}",
            parent.display()
        )
    })?;

    let existing = if config_path.exists() {
        fs::read_to_string(&config_path)
            .map_err(|error| format!("Failed to read {}: {error}", config_path.display()))?
    } else {
        String::new()
    };

    let url = crate::runtime::configured_mcp_server_url();
    let server_name = "fastslides";
    let (updated, status) = upsert_codex_mcp_server_block(&existing, server_name, &url)?;

    if status != CodexInstallStatus::Unchanged || !config_path.exists() {
        fs::write(&config_path, updated)
            .map_err(|error| format!("Failed to write {}: {error}", config_path.display()))?;
    }

    Ok(CodexMcpInstallResponse {
        ok: true,
        status,
        config_path: path_to_string(&config_path),
        server_name: server_name.to_string(),
        url,
    })
}
