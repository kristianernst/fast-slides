pub(crate) const DEFAULT_TITLE: &str = "Presentation";
pub(crate) const DEFAULT_SUBTITLE: &str = "Project Overview";
pub(crate) const DEFAULT_DATE_LABEL: &str = "Month YYYY";
pub(crate) const DEFAULT_PREVIEW_BASE_URL: &str = "http://127.0.0.1:1420";
pub(crate) const DEFAULT_AGENT_HOOK_ADDR: &str = "127.0.0.1:38473";
pub(crate) const DEFAULT_MCP_SERVER_ADDR: &str = "127.0.0.1:38474";
pub(crate) const DEFAULT_MCP_SERVER_PATH: &str = "/mcp";
pub(crate) const FASTSLIDES_MCP_STATEFUL_MODE: bool = false;
pub(crate) const MCP_TRAY_ICON_ID: &str = "fastslides.mcp.server";
pub(crate) const MENU_OPEN_SETTINGS_ID: &str = "menu.open_settings";
pub(crate) const MENU_OPEN_SETTINGS_EVENT: &str = "fastslides://open-settings";
pub(crate) const MENU_OPEN_MAIN_WINDOW_ID: &str = "menu.open_main_window";
pub(crate) const MENU_INSTALL_CODEX_MCP_ID: &str = "menu.install_codex_mcp";
pub(crate) const MENU_EXPORT_SKILL_ID: &str = "menu.export_fastslides_skill";
pub(crate) const MENU_EXPORT_SKILL_EVENT: &str = "fastslides://export-skill";
pub(crate) const SCENE_SESSION_EVENT_NAME: &str = "fastslides://scene-session-event";
pub(crate) const DEFAULT_SLIDES_CSS: &str = "\
/* ═══ slides.css ═══
   Per-project slide design tokens.
   Edit the custom properties below to customise slide appearance.
   Changes are picked up by the preview whenever settings are saved
   or the project is re-selected. Agents can also edit this file
   directly on disk.
*/

:root {
  /* ── Layout ── */
  --slide-bg: #f5f1e8;
  --slide-border: transparent;
  --slide-radius: 14px;
  --slide-padding: 36px;
  --slide-layout-gap: 18px;
  --slide-card-bg: rgba(255, 255, 255, 0.72);
  --slide-card-border: transparent;
  --slide-card-radius: 12px;
  --slide-card-padding: 16px;

  /* ── Typography ── */
  --slide-font-family: \"IBM Plex Sans\", \"Inter\", system-ui, sans-serif;
  --slide-heading-font: \"Iowan Old Style\", \"Georgia\", serif;
  --slide-code-font: \"Fira Code\", monospace;
  --slide-meta-font: var(--slide-code-font);
  --slide-meta-size: 0.72rem;

  /* ── Colors ── */
  --slide-fg: #16212b;
  --slide-h1-color: #13202c;
  --slide-h2-color: #223446;
  --slide-h3-color: #395166;
  --slide-body-color: #425467;
  --slide-meta-color: rgba(66, 84, 103, 0.82);
  --slide-accent: #1f7a78;
  --slide-link-color: var(--slide-accent);
  --slide-code-bg: rgba(23, 32, 43, 0.06);

  /* ── Palette (charts / diagrams / highlights) ── */
  --slide-palette-1: #1f7a78;
  --slide-palette-2: #739e82;
  --slide-palette-3: #d7a65d;
  --slide-palette-4: #ba6b6f;
  --slide-palette-5: #5973a9;
}
";
