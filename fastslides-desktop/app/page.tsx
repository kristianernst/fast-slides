"use client";

import {
  Children,
  isValidElement,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ComponentType,
  type CSSProperties,
  type HTMLAttributes,
  type ImgHTMLAttributes,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import * as jsxRuntime from "react/jsx-runtime";
import { THEMES, renderMermaid } from "beautiful-mermaid";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import {
  materialLight,
  nightOwl,
  oneDark,
  oneLight,
  vs,
  vscDarkPlus,
} from "react-syntax-highlighter/dist/esm/styles/prism";
import { AppSidebar } from "./components/sidebar/AppSidebar";
import { SidebarResizer } from "./components/sidebar/SidebarResizer";
import { SidebarToggleButton } from "./components/sidebar/SidebarToggleButton";
import { AssetLightbox } from "./components/overlays/AssetLightbox";
import { SettingsOverlay } from "./components/settings/SettingsOverlay";
import { PreviewWorkspace } from "./components/workspace/PreviewWorkspace";

type ProjectSummary = {
  name: string;
  path: string;
  root: string;
  slide_count: number;
  updated_at: number;
};

type ProjectDetail = {
  name: string;
  path: string;
  root: string;
  page_mdx: string;
  slide_count: number;
  updated_at: number;
};

type AppConfig = {
  projects_roots: string[];
  recent_projects: string[];
  pinned_projects: string[];
};

type AppState = {
  config: AppConfig;
  projects: ProjectSummary[];
};

type SlideOutlineEntry = {
  index: number;
  title: string;
};

type ExpandableAsset = {
  kind: "image" | "video";
  src: string;
  alt: string;
};

type LayoutGapScale = "xs" | "sm" | "md" | "lg" | "xl";
type LayoutAlign = "start" | "center" | "end" | "stretch";
type LayoutJustify = "start" | "center" | "end" | "between";
type CardTone = "default" | "accent" | "success" | "warning" | "danger";

const LAYOUT_GAP_MULTIPLIER: Record<LayoutGapScale, number> = {
  xs: 0.5,
  sm: 0.75,
  md: 1,
  lg: 1.5,
  xl: 2,
};

function normalizeLayoutGap(value: unknown): LayoutGapScale {
  if (typeof value !== "string") {
    return "md";
  }
  const normalized = value.trim().toLowerCase();
  if ((Object.keys(LAYOUT_GAP_MULTIPLIER) as LayoutGapScale[]).includes(normalized as LayoutGapScale)) {
    return normalized as LayoutGapScale;
  }
  return "md";
}

function normalizeLayoutAlign(value: unknown): LayoutAlign {
  if (typeof value !== "string") {
    return "stretch";
  }
  const normalized = value.trim().toLowerCase();
  if (normalized === "start" || normalized === "center" || normalized === "end" || normalized === "stretch") {
    return normalized;
  }
  return "stretch";
}

function normalizeLayoutJustify(value: unknown): LayoutJustify {
  if (typeof value !== "string") {
    return "start";
  }
  const normalized = value.trim().toLowerCase();
  if (normalized === "start" || normalized === "center" || normalized === "end" || normalized === "between") {
    return normalized;
  }
  return "start";
}

function normalizeGridColumns(value: unknown): 1 | 2 | 3 {
  if (typeof value === "number") {
    if (value === 1 || value === 2 || value === 3) {
      return value;
    }
    return 2;
  }
  if (typeof value === "string") {
    const parsed = Number.parseInt(value, 10);
    if (parsed === 1 || parsed === 2 || parsed === 3) {
      return parsed;
    }
  }
  return 2;
}

function normalizeCardTone(value: unknown): CardTone {
  if (typeof value !== "string") {
    return "default";
  }
  const normalized = value.trim().toLowerCase();
  if (normalized === "default" || normalized === "accent" || normalized === "success" || normalized === "warning" || normalized === "danger") {
    return normalized;
  }
  return "default";
}

function layoutGapCssValue(gap: unknown): string {
  const normalized = normalizeLayoutGap(gap);
  return `calc(var(--slide-layout-gap, 16px) * ${LAYOUT_GAP_MULTIPLIER[normalized]})`;
}

function layoutAlignCssValue(align: unknown): CSSProperties["alignItems"] {
  const normalized = normalizeLayoutAlign(align);
  if (normalized === "start") return "flex-start";
  if (normalized === "end") return "flex-end";
  if (normalized === "center") return "center";
  return "stretch";
}

function layoutJustifyCssValue(justify: unknown): CSSProperties["justifyContent"] {
  const normalized = normalizeLayoutJustify(justify);
  if (normalized === "start") return "flex-start";
  if (normalized === "end") return "flex-end";
  if (normalized === "center") return "center";
  return "space-between";
}

type MdxLayoutProps = HTMLAttributes<HTMLDivElement> & {
  gap?: LayoutGapScale | string;
  align?: LayoutAlign | string;
  justify?: LayoutJustify | string;
};

function MdxStack({
  children,
  className,
  style,
  gap = "md",
  align = "stretch",
  justify = "start",
  ...props
}: MdxLayoutProps) {
  return (
    <div
      {...props}
      className={["mdx-stack", className].filter(Boolean).join(" ")}
      style={{
        display: "flex",
        flexDirection: "column",
        minWidth: 0,
        gap: layoutGapCssValue(gap),
        alignItems: layoutAlignCssValue(align),
        justifyContent: layoutJustifyCssValue(justify),
        ...style,
      }}
    >
      {children}
    </div>
  );
}

type MdxRowProps = MdxLayoutProps & {
  nowrap?: boolean;
};

function MdxRow({
  children,
  className,
  style,
  gap = "md",
  align = "stretch",
  justify = "start",
  nowrap = false,
  ...props
}: MdxRowProps) {
  return (
    <div
      {...props}
      className={["mdx-row", className].filter(Boolean).join(" ")}
      style={{
        display: "flex",
        flexDirection: "row",
        minWidth: 0,
        gap: layoutGapCssValue(gap),
        alignItems: layoutAlignCssValue(align),
        justifyContent: layoutJustifyCssValue(justify),
        flexWrap: nowrap ? "nowrap" : "wrap",
        ...style,
      }}
    >
      {children}
    </div>
  );
}

type MdxGridProps = HTMLAttributes<HTMLDivElement> & {
  cols?: 1 | 2 | 3 | string | number;
  gap?: LayoutGapScale | string;
  align?: LayoutAlign | string;
};

function MdxGrid({
  children,
  className,
  style,
  cols = 2,
  gap = "md",
  align = "stretch",
  ...props
}: MdxGridProps) {
  const normalizedCols = normalizeGridColumns(cols);
  return (
    <div
      {...props}
      className={["mdx-grid", className].filter(Boolean).join(" ")}
      data-cols={normalizedCols}
      style={{
        display: "grid",
        gridTemplateColumns: `repeat(${normalizedCols}, minmax(0, 1fr))`,
        minWidth: 0,
        gap: layoutGapCssValue(gap),
        alignItems: layoutAlignCssValue(align),
        ...style,
      }}
    >
      {children}
    </div>
  );
}

type MdxCardProps = HTMLAttributes<HTMLDivElement> & {
  title?: ReactNode;
  subtitle?: ReactNode;
  tone?: CardTone | string;
};

function MdxCard({
  children,
  className,
  title,
  subtitle,
  tone = "default",
  ...props
}: MdxCardProps) {
  const normalizedTone = normalizeCardTone(tone);
  return (
    <article
      {...props}
      className={["mdx-card", `mdx-card--${normalizedTone}`, className].filter(Boolean).join(" ")}
    >
      {title ? <h3 className="mdx-card-title">{title}</h3> : null}
      {subtitle ? <p className="mdx-caption mdx-card-subtitle">{subtitle}</p> : null}
      <div className="mdx-card-body">{children}</div>
    </article>
  );
}

type MdxMetricProps = HTMLAttributes<HTMLDivElement> & {
  label?: ReactNode;
  value?: ReactNode;
  hint?: ReactNode;
};

function MdxMetric({
  className,
  label,
  value,
  hint,
  children,
  ...props
}: MdxMetricProps) {
  return (
    <article {...props} className={["mdx-metric", className].filter(Boolean).join(" ")}>
      {label ? <p className="mdx-caption mdx-metric-label">{label}</p> : null}
      <p className="mdx-metric-value">{value ?? children}</p>
      {hint ? <p className="mdx-caption mdx-metric-hint">{hint}</p> : null}
    </article>
  );
}

function MdxCaption({
  children,
  className,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return (
    <div {...props} className={["mdx-caption", className].filter(Boolean).join(" ")}>
      {children}
    </div>
  );
}

function isTauriRuntime(): boolean {
  if (typeof window === "undefined") {
    return false;
  }
  const runtimeWindow = window as Window & {
    __TAURI_INTERNALS__?: unknown;
    __TAURI__?: unknown;
  };
  return Boolean(runtimeWindow.__TAURI_INTERNALS__ || runtimeWindow.__TAURI__);
}

async function call<T>(command: string, payload?: Record<string, unknown>): Promise<T> {
  if (!isTauriRuntime()) {
    throw new Error("Tauri runtime not detected. Launch this screen with `npm run tauri:dev`.");
  }
  return invoke<T>(command, payload);
}

async function pickFolder(title: string): Promise<string> {
  if (!isTauriRuntime()) {
    throw new Error("Folder picker is available only in Tauri runtime.");
  }
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    directory: true,
    multiple: false,
    title,
  });
  if (typeof selected === "string") {
    return selected;
  }
  return "";
}

const SELECTED_STATE_KEY = "fastslides_selected_path";
const SIDEBAR_WIDTH_STATE_KEY = "fastslides_sidebar_width";
const THEME_STATE_KEY = "fastslides_theme";
const MERMAID_THEME_STATE_KEY = "fastslides_mermaid_theme";
const SYNTAX_THEME_STATE_KEY = "fastslides_syntax_theme";

const MERMAID_THEME_OPTIONS = [
  "zinc-light",
  "zinc-dark",
  "tokyo-night",
  "tokyo-night-storm",
  "tokyo-night-light",
  "catppuccin-latte",
  "catppuccin-mocha",
  "nord",
  "nord-light",
  "dracula",
  "github-light",
  "github-dark",
  "one-dark",
  "solarized-light",
  "solarized-dark",
] as const;
type MermaidThemeName = (typeof MERMAID_THEME_OPTIONS)[number];

const MERMAID_THEME_LABELS: Record<MermaidThemeName, string> = {
  "zinc-light": "Zinc Light",
  "zinc-dark": "Zinc Dark",
  "tokyo-night": "Tokyo Night",
  "tokyo-night-storm": "Tokyo Night Storm",
  "tokyo-night-light": "Tokyo Night Light",
  "catppuccin-latte": "Catppuccin Latte",
  "catppuccin-mocha": "Catppuccin Mocha",
  nord: "Nord",
  "nord-light": "Nord Light",
  dracula: "Dracula",
  "github-light": "GitHub Light",
  "github-dark": "GitHub Dark",
  "one-dark": "One Dark",
  "solarized-light": "Solarized Light",
  "solarized-dark": "Solarized Dark",
};

const SYNTAX_THEME_OPTIONS_BY_MODE = {
  dark: ["one-dark", "vsc-dark-plus", "night-owl"],
  light: ["one-light", "vs", "material-light"],
} as const;
type SyntaxThemeMode = keyof typeof SYNTAX_THEME_OPTIONS_BY_MODE;
type SyntaxThemeName =
  | (typeof SYNTAX_THEME_OPTIONS_BY_MODE)["dark"][number]
  | (typeof SYNTAX_THEME_OPTIONS_BY_MODE)["light"][number];

const SYNTAX_THEME_LABELS: Record<SyntaxThemeName, string> = {
  "one-dark": "One Dark",
  "one-light": "One Light",
  "vsc-dark-plus": "VS Code Dark+",
  vs: "VS",
  "night-owl": "Night Owl",
  "material-light": "Material Light",
};

const SYNTAX_THEME_STYLES: Record<SyntaxThemeName, Record<string, CSSProperties>> = {
  "one-dark": oneDark,
  "one-light": oneLight,
  "vsc-dark-plus": vscDarkPlus,
  vs,
  "night-owl": nightOwl,
  "material-light": materialLight,
};

type SlideTokens = {
  slideBg: string;
  slideBorder: string;
  slideRadius: string;
  slidePadding: string;
  slideLayoutGap: string;
  slideCardBg: string;
  slideCardBorder: string;
  slideCardRadius: string;
  slideCardPadding: string;
  slideFontFamily: string;
  slideHeadingFont: string;
  slideCodeFont: string;
  slideMetaFont: string;
  slideMetaSize: string;
  slideFg: string;
  slideH1Color: string;
  slideH2Color: string;
  slideH3Color: string;
  slideBodyColor: string;
  slideMetaColor: string;
  slideAccent: string;
  slideLinkColor: string;
  slideCodeBg: string;
  slidePalette1: string;
  slidePalette2: string;
  slidePalette3: string;
  slidePalette4: string;
  slidePalette5: string;
};

const DEFAULT_TOKENS: SlideTokens = {
  slideBg: "#0e0d0a",
  slideBorder: "#efefeb1f",
  slideRadius: "10px",
  slidePadding: "32px",
  slideLayoutGap: "16px",
  slideCardBg: "#ffffff08",
  slideCardBorder: "#00000000",
  slideCardRadius: "10px",
  slideCardPadding: "16px",
  slideFontFamily: '"Inter", system-ui, sans-serif',
  slideHeadingFont: "var(--slide-font-family)",
  slideCodeFont: '"Fira Code", monospace',
  slideMetaFont: "var(--slide-code-font)",
  slideMetaSize: "0.78rem",
  slideFg: "#edecec",
  slideH1Color: "#ffffff",
  slideH2Color: "#d7d6d5",
  slideH3Color: "#b0afab",
  slideBodyColor: "#c4c3bf",
  slideMetaColor: "#c4c3bfe0",
  slideAccent: "#7b9cbc",
  slideLinkColor: "var(--slide-accent)",
  slideCodeBg: "#ffffff0f",
  slidePalette1: "#7b9cbc",
  slidePalette2: "#63b18a",
  slidePalette3: "#e1b86f",
  slidePalette4: "#d68080",
  slidePalette5: "#b08cd6",
};

const TOKEN_TO_VAR: Record<keyof SlideTokens, string> = {
  slideBg: "--slide-bg",
  slideBorder: "--slide-border",
  slideRadius: "--slide-radius",
  slidePadding: "--slide-padding",
  slideLayoutGap: "--slide-layout-gap",
  slideCardBg: "--slide-card-bg",
  slideCardBorder: "--slide-card-border",
  slideCardRadius: "--slide-card-radius",
  slideCardPadding: "--slide-card-padding",
  slideFontFamily: "--slide-font-family",
  slideHeadingFont: "--slide-heading-font",
  slideCodeFont: "--slide-code-font",
  slideMetaFont: "--slide-meta-font",
  slideMetaSize: "--slide-meta-size",
  slideFg: "--slide-fg",
  slideH1Color: "--slide-h1-color",
  slideH2Color: "--slide-h2-color",
  slideH3Color: "--slide-h3-color",
  slideBodyColor: "--slide-body-color",
  slideMetaColor: "--slide-meta-color",
  slideAccent: "--slide-accent",
  slideLinkColor: "--slide-link-color",
  slideCodeBg: "--slide-code-bg",
  slidePalette1: "--slide-palette-1",
  slidePalette2: "--slide-palette-2",
  slidePalette3: "--slide-palette-3",
  slidePalette4: "--slide-palette-4",
  slidePalette5: "--slide-palette-5",
};

const FONT_OPTIONS = [
  '"Inter", system-ui, sans-serif',
  '"Helvetica Neue", Helvetica, Arial, sans-serif',
  'Georgia, "Times New Roman", serif',
  '"Fira Code", monospace',
  'system-ui, sans-serif',
];

const MONO_FONT_OPTIONS = [
  '"Fira Code", monospace',
  '"SF Mono", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
  "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
];

function fontLabel(fontValue: string): string {
  if (fontValue === "var(--slide-font-family)") {
    return "Same as body";
  }
  if (fontValue === "var(--slide-code-font)") {
    return "Same as code";
  }
  const first = fontValue.split(",")[0] || fontValue;
  return first.replace(/["']/g, "").trim();
}

function parseCssToTokens(css: string): Partial<SlideTokens> {
  const result: Partial<SlideTokens> = {};
  const varToToken = Object.fromEntries(
    Object.entries(TOKEN_TO_VAR).map(([k, v]) => [v, k]),
  );
  const re = /--([\w-]+)\s*:\s*(.+?)\s*;/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(css)) !== null) {
    const varName = `--${m[1]}`;
    const tokenKey = varToToken[varName] as keyof SlideTokens | undefined;
    if (tokenKey) result[tokenKey] = m[2];
  }
  return result;
}

function tokensToCss(tokens: SlideTokens): string {
  const lines = Object.entries(TOKEN_TO_VAR).map(
    ([key, varName]) => `  ${varName}: ${tokens[key as keyof SlideTokens]};`,
  );
  return `:root {\n${lines.join("\n")}\n}\n`;
}

function isHexColor(v: string): boolean {
  return /^#[0-9a-fA-F]{3,8}$/.test(v.trim());
}

function hexForInput(v: string): string {
  const t = v.trim();
  if (/^#[0-9a-fA-F]{8}$/.test(t)) return t.slice(0, 7);
  if (/^#[0-9a-fA-F]{6}$/.test(t)) return t;
  if (/^#[0-9a-fA-F]{4}$/.test(t)) {
    const [, a, b, c] = t;
    return `#${a}${a}${b}${b}${c}${c}`;
  }
  if (/^#[0-9a-fA-F]{3}$/.test(t)) {
    const [, a, b, c] = t;
    return `#${a}${a}${b}${b}${c}${c}`;
  }
  return "#000000";
}
const SIDEBAR_MIN_WIDTH = 220;
const SIDEBAR_MAX_WIDTH = 420;
const OPEN_SETTINGS_MENU_EVENT = "fastslides://open-settings";
const EXPORT_SKILL_MENU_EVENT = "fastslides://export-skill";
const PREVIEW_ZOOM_MIN = 0.8;
const PREVIEW_ZOOM_MAX = 2.5;
const PREVIEW_ZOOM_STEP = 0.05;
const PROJECT_ROOT_ABSOLUTE_ASSET_PREFIXES = ["/assets/", "/images/", "/media/", "/data/"];
const projectAssetDataUrlCache = new Map<string, string>();
const IMAGE_ASSET_EXTENSION_RE = /\.(avif|bmp|gif|jpe?g|png|svg|webp)$/i;
const VIDEO_ASSET_EXTENSION_RE = /\.(m4v|mov|mp4|ogv|ogg|webm)$/i;

function clampPreviewZoom(zoom: number): number {
  return Math.min(PREVIEW_ZOOM_MAX, Math.max(PREVIEW_ZOOM_MIN, zoom));
}

function clampSidebarWidth(width: number): number {
  return Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, width));
}

function stripFrontmatter(source: string): string {
  return source.replace(/^---\s*\n[\s\S]*?\n---\s*(?:\n|$)/, "");
}

function normalizeDeckSource(source: string): string {
  return stripFrontmatter(source).trim();
}

function clampUnit(value: number): number {
  return Math.min(1, Math.max(0, value));
}

function isExternalAssetPath(value: string): boolean {
  const lower = value.toLowerCase();
  return (
    value.startsWith("//") ||
    lower.startsWith("http://") ||
    lower.startsWith("https://") ||
    lower.startsWith("data:") ||
    lower.startsWith("blob:") ||
    lower.startsWith("mailto:") ||
    lower.startsWith("tel:")
  );
}

function splitAssetPathAndSuffix(raw: string): { pathOnly: string; suffix: string } {
  const match = raw.match(/^([^?#]*)(.*)$/);
  return {
    pathOnly: match?.[1] ?? raw,
    suffix: match?.[2] ?? "",
  };
}

function normalizeProjectRelativeAsset(raw: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed || trimmed.startsWith("#") || isExternalAssetPath(trimmed)) {
    return null;
  }

  const { pathOnly } = splitAssetPathAndSuffix(trimmed);
  if (!pathOnly) {
    return null;
  }

  let relative = pathOnly.replace(/\\/g, "/");
  if (relative.startsWith("/")) {
    const supportedRootRelative = PROJECT_ROOT_ABSOLUTE_ASSET_PREFIXES.some(
      (prefix) => relative === prefix.slice(0, -1) || relative.startsWith(prefix),
    );
    if (!supportedRootRelative) {
      return null;
    }
    relative = relative.slice(1);
  }

  const normalizedParts: string[] = [];
  for (const part of relative.split("/")) {
    if (!part || part === ".") {
      continue;
    }
    if (part === "..") {
      if (normalizedParts.length === 0) {
        return null;
      }
      normalizedParts.pop();
      continue;
    }
    normalizedParts.push(part);
  }

  if (normalizedParts.length === 0) {
    return null;
  }

  return normalizedParts.join("/");
}

function inferAssetKind(rawValue: string): ExpandableAsset["kind"] | null {
  const value = rawValue.trim();
  if (!value) {
    return null;
  }
  const { pathOnly } = splitAssetPathAndSuffix(value);
  if (IMAGE_ASSET_EXTENSION_RE.test(pathOnly)) {
    return "image";
  }
  if (VIDEO_ASSET_EXTENSION_RE.test(pathOnly)) {
    return "video";
  }
  return null;
}

function isMermaidThemeName(value: string): value is MermaidThemeName {
  return (MERMAID_THEME_OPTIONS as readonly string[]).includes(value);
}

function isSyntaxThemeName(value: string): value is SyntaxThemeName {
  return (
    (SYNTAX_THEME_OPTIONS_BY_MODE.dark as readonly string[]).includes(value) ||
    (SYNTAX_THEME_OPTIONS_BY_MODE.light as readonly string[]).includes(value)
  );
}

function syntaxThemeModeForUiTheme(uiTheme: "dark" | "light"): SyntaxThemeMode {
  return uiTheme === "light" ? "light" : "dark";
}

function flattenText(value: unknown): string {
  if (typeof value === "string") {
    return value;
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  if (Array.isArray(value)) {
    return value.map((item) => flattenText(item)).join("");
  }
  if (isValidElement(value)) {
    return flattenText((value.props as { children?: unknown }).children);
  }
  return "";
}

function extractFencedCode(children: ReactNode): { language: string; code: string } | null {
  const nodes = Children.toArray(children);
  const codeNode = nodes.find((node) => isValidElement(node) && node.type === "code");
  if (!codeNode || !isValidElement(codeNode)) {
    return null;
  }

  const props = codeNode.props as {
    className?: string;
    children?: ReactNode;
  };
  const className = props.className || "";
  const languageMatch = className.match(/language-([\w-]+)/i);
  const language = languageMatch?.[1]?.toLowerCase() ?? "";
  const code = flattenText(props.children).replace(/\n$/, "");

  return { language, code };
}

function MermaidDiagram({
  code,
  mermaidThemeName,
}: {
  code: string;
  mermaidThemeName: MermaidThemeName;
}) {
  const [svg, setSvg] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    let cancelled = false;
    setSvg("");
    setError("");

    const theme = THEMES[mermaidThemeName as keyof typeof THEMES] ?? THEMES["zinc-dark"];
    renderMermaid(code, {
      ...theme,
      transparent: true,
      font: "Inter",
    })
      .then((nextSvg) => {
        if (!cancelled) {
          setSvg(nextSvg);
        }
      })
      .catch((cause) => {
        if (!cancelled) {
          const message = cause instanceof Error ? cause.message : "Failed to render Mermaid diagram.";
          setError(message);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [code, mermaidThemeName]);

  if (error) {
    return (
      <pre className="mermaid-render-error">
        <code>{code}</code>
      </pre>
    );
  }

  if (!svg) {
    return <div className="mermaid-preview-loading">Rendering Mermaid…</div>;
  }

  return (
    <div className="mermaid-preview-block">
      <div
        className="mermaid-preview-svg"
        dangerouslySetInnerHTML={{ __html: svg }}
      />
    </div>
  );
}

function MdxPreBlock({
  children,
  mermaidThemeName,
  syntaxThemeName,
  uiTheme,
}: HTMLAttributes<HTMLPreElement> & {
  children?: ReactNode;
  mermaidThemeName: MermaidThemeName;
  syntaxThemeName: SyntaxThemeName;
  uiTheme: "dark" | "light";
}) {
  const parsed = extractFencedCode(children);
  if (!parsed) {
    return <pre>{children}</pre>;
  }

  if (parsed.language === "mermaid") {
    return <MermaidDiagram code={parsed.code} mermaidThemeName={mermaidThemeName} />;
  }

  const syntaxTheme =
    SYNTAX_THEME_STYLES[syntaxThemeName] || (uiTheme === "light" ? oneLight : oneDark);

  return (
    <SyntaxHighlighter
      className="slide-syntax-block"
      language={parsed.language || "text"}
      style={syntaxTheme}
      customStyle={{
        margin: 0,
        borderRadius: 10,
        padding: "14px 16px",
        background: "var(--slide-code-bg, rgba(255, 255, 255, 0.06))",
        border: "1px solid var(--slide-card-border, transparent)",
      }}
      codeTagProps={{
        style: {
          fontFamily: "var(--slide-code-font, var(--font-mono))",
          fontSize: "0.88rem",
          lineHeight: "1.5",
        },
      }}
      wrapLongLines
      showLineNumbers={parsed.code.split("\n").length > 6}
      lineNumberStyle={{
        color: "var(--color-text-tertiary)",
        opacity: 0.85,
        paddingRight: "0.85rem",
      }}
    >
      {parsed.code}
    </SyntaxHighlighter>
  );
}

async function resolveProjectAssetSource(projectPath: string, rawSrc: string): Promise<string> {
  const normalizedRelative = normalizeProjectRelativeAsset(rawSrc);
  if (!normalizedRelative || !projectPath || !isTauriRuntime()) {
    return rawSrc;
  }

  const cacheKey = `${projectPath}::${normalizedRelative}`;
  const cached = projectAssetDataUrlCache.get(cacheKey);
  if (cached) {
    return cached;
  }

  const nextSource = await call<string>("resolve_project_asset_data_url", {
    projectPath,
    rawSrc,
  });
  projectAssetDataUrlCache.set(cacheKey, nextSource);
  return nextSource;
}

function ProjectAssetImage({
  projectPath,
  ...props
}: ImgHTMLAttributes<HTMLImageElement> & { projectPath: string }) {
  const source = typeof props.src === "string" ? props.src : "";
  const [resolvedSrc, setResolvedSrc] = useState(source);

  useEffect(() => {
    if (typeof props.src !== "string") {
      return;
    }

    let cancelled = false;
    setResolvedSrc(props.src);
    resolveProjectAssetSource(projectPath, props.src)
      .then((nextSource) => {
        if (cancelled) {
          return;
        }
        setResolvedSrc(nextSource);
      })
      .catch(() => {
        if (!cancelled) {
          setResolvedSrc(source);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [projectPath, props.src]);

  return <img {...props} src={resolvedSrc || source} loading={props.loading ?? "lazy"} />;
}

function EmbeddedDeckPreview({
  source,
  projectPath,
  mermaidThemeName,
  syntaxThemeName,
  uiTheme,
  presenterMode,
  activeSlideIndex,
  onSlideCountChange,
  onActiveSlidePick,
  onSlideOutlineChange,
  onAssetOpen,
}: {
  source: string;
  projectPath: string;
  mermaidThemeName: MermaidThemeName;
  syntaxThemeName: SyntaxThemeName;
  uiTheme: "dark" | "light";
  presenterMode: boolean;
  activeSlideIndex: number;
  onSlideCountChange: (count: number) => void;
  onActiveSlidePick: (index: number) => void;
  onSlideOutlineChange: (slides: SlideOutlineEntry[]) => void;
  onAssetOpen: (asset: ExpandableAsset) => void;
}) {
  const [CompiledDeck, setCompiledDeck] = useState<ComponentType<Record<string, unknown>> | null>(null);
  const [error, setError] = useState("");
  const previewRootRef = useRef<HTMLDivElement | null>(null);
  const slideElementsRef = useRef<HTMLElement[]>([]);
  const previousActiveSlideRef = useRef(-1);
  const mdxComponents = useMemo(
    () => ({
      img: (props: ImgHTMLAttributes<HTMLImageElement>) => {
        return <ProjectAssetImage {...props} projectPath={projectPath} />;
      },
      pre: (props: HTMLAttributes<HTMLPreElement>) => (
        <MdxPreBlock
          {...props}
          mermaidThemeName={mermaidThemeName}
          syntaxThemeName={syntaxThemeName}
          uiTheme={uiTheme}
        />
      ),
      Stack: MdxStack,
      Row: MdxRow,
      Grid: MdxGrid,
      Card: MdxCard,
      Metric: MdxMetric,
      Caption: MdxCaption,
    }),
    [mermaidThemeName, projectPath, syntaxThemeName, uiTheme],
  );

  useEffect(() => {
    let cancelled = false;

    async function compileSource() {
      if (!source.trim()) {
        setError("Project has an empty page.mdx.");
        setCompiledDeck(null);
        return;
      }

      setError("");
      setCompiledDeck(null);

      try {
        const { evaluate } = await import("@mdx-js/mdx");
        const module = await evaluate(source, {
          ...jsxRuntime,
          development: false,
        });

        if (!cancelled) {
          setCompiledDeck(() => module.default as ComponentType<Record<string, unknown>>);
        }
      } catch (cause) {
        if (cancelled) {
          return;
        }
        const message =
          cause instanceof Error ? cause.message : "Failed to compile page.mdx for embedded preview.";
        setError(message);
      }
    }

    void compileSource();

    return () => {
      cancelled = true;
    };
  }, [source]);

  useEffect(() => {
    const root = previewRootRef.current;
    if (!root || !CompiledDeck) {
      slideElementsRef.current = [];
      previousActiveSlideRef.current = -1;
      onSlideCountChange(0);
      onSlideOutlineChange([]);
      return;
    }

    const slides = Array.from(root.querySelectorAll<HTMLElement>(".slide"));
    slideElementsRef.current = slides;
    previousActiveSlideRef.current = -1;
    onSlideCountChange(slides.length);
    root.classList.remove("embedded-preview-single");

    const nextOutline: SlideOutlineEntry[] = [];
    for (let index = 0; index < slides.length; index += 1) {
      const slide = slides[index];
      slide.dataset.slideIndex = String(index);
      const heading = slide.querySelector("h1, h2, h3");
      const headingText = heading?.textContent?.trim();
      nextOutline.push({
        index,
        title: headingText && headingText.length > 0 ? headingText : `Slide ${index + 1}`,
      });
    }
    onSlideOutlineChange(nextOutline);

    if (slides.length === 0) {
      return;
    }

    for (let index = 0; index < slides.length; index += 1) {
      slides[index].dataset.active = "false";
      slides[index].setAttribute("aria-hidden", "false");
    }
  }, [CompiledDeck, onSlideCountChange, onSlideOutlineChange]);

  useEffect(() => {
    const root = previewRootRef.current;
    const slides = slideElementsRef.current;
    if (!root || slides.length === 0) {
      return;
    }

    const safeIndex = Math.min(activeSlideIndex, slides.length - 1);
    const wasSingleMode = root.classList.contains("embedded-preview-single");
    root.classList.toggle("embedded-preview-single", presenterMode);

    if (!presenterMode) {
      if (wasSingleMode) {
        for (let index = 0; index < slides.length; index += 1) {
          slides[index].setAttribute("aria-hidden", "false");
        }
      }
      const previous = previousActiveSlideRef.current;
      if (previous >= 0 && previous < slides.length && previous !== safeIndex) {
        slides[previous].dataset.active = "false";
      }
      slides[safeIndex].dataset.active = "true";
      previousActiveSlideRef.current = safeIndex;
      return;
    }

    const previous = previousActiveSlideRef.current;
    if (previous >= 0 && previous < slides.length && previous !== safeIndex) {
      slides[previous].dataset.active = "false";
      slides[previous].setAttribute("aria-hidden", "true");
    }

    slides[safeIndex].dataset.active = "true";
    slides[safeIndex].setAttribute("aria-hidden", "false");

    if (!wasSingleMode) {
      for (let index = 0; index < slides.length; index += 1) {
        if (index !== safeIndex) {
          slides[index].setAttribute("aria-hidden", "true");
        }
      }
    }

    previousActiveSlideRef.current = safeIndex;
  }, [activeSlideIndex, presenterMode]);

  useEffect(() => {
    const root = previewRootRef.current;
    if (!root) {
      return;
    }

    const onClick = (event: MouseEvent): void => {
      const target = event.target as HTMLElement | null;

      const image = target?.closest("img") as HTMLImageElement | null;
      if (image) {
        const rawSource = image.currentSrc || image.src || image.getAttribute("src") || "";
        if (rawSource) {
          event.preventDefault();
          event.stopPropagation();
          onAssetOpen({
            kind: "image",
            src: rawSource,
            alt: image.alt || image.getAttribute("title") || "Slide image",
          });
          return;
        }
      }

      const video = target?.closest("video") as HTMLVideoElement | null;
      if (video) {
        const sourceNode = video.querySelector<HTMLSourceElement>("source[src]");
        const rawSource =
          video.currentSrc ||
          video.src ||
          video.getAttribute("src") ||
          sourceNode?.src ||
          sourceNode?.getAttribute("src") ||
          "";
        if (rawSource) {
          event.preventDefault();
          event.stopPropagation();
          onAssetOpen({
            kind: "video",
            src: rawSource,
            alt: video.getAttribute("aria-label") || video.getAttribute("title") || "Slide video",
          });
          return;
        }
      }

      const anchor = target?.closest("a[href]") as HTMLAnchorElement | null;
      if (anchor) {
        const rawHref = anchor.getAttribute("href") || anchor.href || "";
        const kind = inferAssetKind(rawHref);
        if (kind) {
          event.preventDefault();
          event.stopPropagation();
          onAssetOpen({
            kind,
            src: rawHref,
            alt: anchor.textContent?.trim() || "Slide asset",
          });
          return;
        }
      }

      if (presenterMode) {
        return;
      }
      const slide = target?.closest(".slide") as HTMLElement | null;
      if (!slide) {
        return;
      }
      const slides = Array.from(root.querySelectorAll<HTMLElement>(".slide"));
      const clickedIndex = slides.indexOf(slide);
      if (clickedIndex >= 0) {
        onActiveSlidePick(clickedIndex);
      }
    };

    root.addEventListener("click", onClick);
    return () => {
      root.removeEventListener("click", onClick);
    };
  }, [onActiveSlidePick, onAssetOpen, presenterMode]);

  if (error) {
    return <div className="embedded-preview-error">{error}</div>;
  }

  if (!CompiledDeck) {
    return <div className="embedded-preview-loading">Loading deck preview…</div>;
  }

  return (
    <div ref={previewRootRef} className="embedded-preview-deck">
      <CompiledDeck components={mdxComponents} />
    </div>
  );
}

export default function Home() {
  const [appState, setAppState] = useState<AppState | null>(null);
  const [selectedPath, setSelectedPath] = useState("");
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [sidebarWidth, setSidebarWidth] = useState(280);
  const [sidebarDragging, setSidebarDragging] = useState(false);
  const [selectedProjectDetail, setSelectedProjectDetail] = useState<ProjectDetail | null>(null);
  const [presenterMode, setPresenterMode] = useState(false);
  const [activeSlideIndex, setActiveSlideIndex] = useState(0);
  const [embeddedSlideCount, setEmbeddedSlideCount] = useState(0);
  const [slideOutline, setSlideOutline] = useState<SlideOutlineEntry[]>([]);
  const [previewDockVisible, setPreviewDockVisible] = useState(true);
  const [previewZoom, setPreviewZoom] = useState(1);
  const [busy, setBusy] = useState(false);
  const [theme, setTheme] = useState<"dark" | "light">(() => {
    if (typeof window !== "undefined") {
      const stored = localStorage.getItem(THEME_STATE_KEY);
      if (stored === "light") return "light";
    }
    return "dark";
  });
  const [mermaidThemeName, setMermaidThemeName] = useState<MermaidThemeName>(() => {
    if (typeof window !== "undefined") {
      const stored = localStorage.getItem(MERMAID_THEME_STATE_KEY);
      if (stored && isMermaidThemeName(stored)) {
        return stored;
      }
      const storedUiTheme = localStorage.getItem(THEME_STATE_KEY);
      if (storedUiTheme === "light") {
        return "github-light";
      }
    }
    return "zinc-dark";
  });
  const [syntaxThemeName, setSyntaxThemeName] = useState<SyntaxThemeName>(() => {
    if (typeof window !== "undefined") {
      const stored = localStorage.getItem(SYNTAX_THEME_STATE_KEY);
      if (stored && isSyntaxThemeName(stored)) {
        return stored;
      }
      const storedUiTheme = localStorage.getItem(THEME_STATE_KEY) === "light" ? "light" : "dark";
      const mode = syntaxThemeModeForUiTheme(storedUiTheme);
      return SYNTAX_THEME_OPTIONS_BY_MODE[mode][0];
    }
    return SYNTAX_THEME_OPTIONS_BY_MODE.dark[0];
  });
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [expandedAsset, setExpandedAsset] = useState<ExpandableAsset | null>(null);
  const [projectCss, setProjectCss] = useState("");
  const [cssEditorValue, setCssEditorValue] = useState("");
  const [slideTokens, setSlideTokens] = useState<SlideTokens>({ ...DEFAULT_TOKENS });
  const sidebarResizeCleanupRef = useRef<(() => void) | null>(null);
  const previewDockHideTimerRef = useRef<number | null>(null);
  const previewDockHoveringRef = useRef(false);
  const previewSurfaceRef = useRef<HTMLDivElement | null>(null);

  // Restore selection on load
  useEffect(() => {
    if (typeof window !== "undefined") {
      const stored = localStorage.getItem(SELECTED_STATE_KEY);
      if (stored) {
        setSelectedPath(stored);
      }
    }
  }, []);

  // Sync selection
  useEffect(() => {
    if (typeof window !== "undefined" && selectedPath) {
      localStorage.setItem(SELECTED_STATE_KEY, selectedPath);
    }
  }, [selectedPath]);

  useEffect(() => {
    if (typeof window !== "undefined") {
      const storedWidth = Number(localStorage.getItem(SIDEBAR_WIDTH_STATE_KEY));
      if (Number.isFinite(storedWidth)) {
        setSidebarWidth(clampSidebarWidth(storedWidth));
      }
    }
  }, []);

  useEffect(() => {
    if (typeof window !== "undefined") {
      localStorage.setItem(SIDEBAR_WIDTH_STATE_KEY, String(sidebarWidth));
    }
  }, [sidebarWidth]);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    if (typeof window !== "undefined") {
      localStorage.setItem(THEME_STATE_KEY, theme);
    }
  }, [theme]);

  useEffect(() => {
    if (typeof window !== "undefined") {
      localStorage.setItem(MERMAID_THEME_STATE_KEY, mermaidThemeName);
    }
  }, [mermaidThemeName]);

  useEffect(() => {
    if (typeof window !== "undefined") {
      localStorage.setItem(SYNTAX_THEME_STATE_KEY, syntaxThemeName);
    }
  }, [syntaxThemeName]);

  const projects = appState?.projects || [];

  const selectedProject = useMemo(() => {
    if (!selectedPath) {
      return null;
    }
    return projects.find((project) => project.path === selectedPath) || null;
  }, [projects, selectedPath]);

  const selectedProjectEmbeddedSource = useMemo(
    () => normalizeDeckSource(selectedProjectDetail?.page_mdx || ""),
    [selectedProjectDetail?.page_mdx],
  );

  const visibleSlideCount = useMemo(() => {
    const fallbackCount = selectedProjectDetail?.slide_count || selectedProject?.slide_count || 0;
    return embeddedSlideCount > 0 ? embeddedSlideCount : fallbackCount;
  }, [embeddedSlideCount, selectedProject?.slide_count, selectedProjectDetail?.slide_count]);

  const maxSlideIndex = Math.max(visibleSlideCount - 1, 0);
  const slideTocEntries = useMemo<SlideOutlineEntry[]>(() => {
    if (slideOutline.length > 0) {
      return slideOutline;
    }
    return Array.from({ length: visibleSlideCount }, (_, index) => ({
      index,
      title: `Slide ${index + 1}`,
    }));
  }, [slideOutline, visibleSlideCount]);

  useEffect(() => {
    setActiveSlideIndex(0);
    setEmbeddedSlideCount(0);
    setSlideOutline([]);
    setPresenterMode(false);
  }, [selectedProject?.path]);

  useEffect(() => {
    if (!selectedProject) {
      setPreviewDockVisible(false);
      clearPreviewDockHideTimer();
      return;
    }
    revealPreviewDock(1800);
  }, [selectedProject?.path]);

  useEffect(() => {
    setActiveSlideIndex((previous) => Math.min(previous, maxSlideIndex));
  }, [maxSlideIndex]);

  useEffect(() => {
    if (!selectedProject) {
      setSelectedProjectDetail(null);
      return;
    }

    let cancelled = false;
    call<ProjectDetail>("load_project", { path: selectedProject.path })
      .then((detail) => {
        if (!cancelled) {
          setSelectedProjectDetail(detail);
        }
      })
      .catch((cause) => {
        if (!cancelled) {
          const message = cause instanceof Error ? cause.message : "Failed to load project source.";
          console.log(message);
          setSelectedProjectDetail(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [selectedProject?.path]);

  useEffect(() => {
    if (!selectedProject) {
      setProjectCss("");
      setCssEditorValue("");
      return;
    }
    let cancelled = false;
    call<string>("read_project_css", { path: selectedProject.path })
      .then((css) => {
        if (!cancelled) {
          setProjectCss(css);
          setCssEditorValue(css);
          setSlideTokens({ ...DEFAULT_TOKENS, ...parseCssToTokens(css) });
        }
      })
      .catch(() => {
        if (!cancelled) {
          setProjectCss("");
          setCssEditorValue("");
          setSlideTokens({ ...DEFAULT_TOKENS });
        }
      });
    return () => { cancelled = true; };
  }, [selectedProject?.path]);

  useEffect(() => {
    const id = "fastslides-project-css";
    let style = document.getElementById(id) as HTMLStyleElement | null;
    if (!style) {
      style = document.createElement("style");
      style.id = id;
      document.head.appendChild(style);
    }
    style.textContent = projectCss;
    return () => { style?.remove(); };
  }, [projectCss]);

  useEffect(() => {
    if (previewDockHideTimerRef.current !== null) {
      window.clearTimeout(previewDockHideTimerRef.current);
      previewDockHideTimerRef.current = null;
    }
    return () => {
      if (previewDockHideTimerRef.current !== null) {
        window.clearTimeout(previewDockHideTimerRef.current);
      }
    };
  }, []);

  function getCurrentCenteredSlideIndex(): number {
    const container = previewSurfaceRef.current;
    if (!container) {
      return -1;
    }
    const slides = Array.from(container.querySelectorAll<HTMLElement>(".embedded-preview-deck .slide"));
    if (slides.length === 0) {
      return -1;
    }

    const viewportCenterY = container.getBoundingClientRect().top + container.clientHeight / 2;
    let nextIndex = 0;
    let minDistance = Number.POSITIVE_INFINITY;

    for (let position = 0; position < slides.length; position += 1) {
      const slide = slides[position];
      const rect = slide.getBoundingClientRect();
      const slideCenter = rect.top + rect.height / 2;
      const distance = Math.abs(slideCenter - viewportCenterY);
      if (distance < minDistance) {
        minDistance = distance;
        const explicitIndex = Number(slide.dataset.slideIndex);
        nextIndex = Number.isFinite(explicitIndex) ? explicitIndex : position;
      }
    }

    return nextIndex;
  }

  function scrollListToSlide(index: number, behavior: ScrollBehavior = "smooth"): void {
    const container = previewSurfaceRef.current;
    if (!container) {
      return;
    }
    const target = container.querySelector<HTMLElement>(`.embedded-preview-deck .slide[data-slide-index="${index}"]`);
    if (!target) {
      return;
    }
    target.scrollIntoView({
      behavior,
      block: "center",
      inline: "nearest",
    });
  }

  function handleTocSelect(index: number): void {
    setActiveSlideIndex(index);
    if (!presenterMode) {
      scrollListToSlide(index);
    }
    revealPreviewDock();
  }

  useEffect(() => {
    if (!selectedProject || presenterMode) {
      return;
    }

    const container = previewSurfaceRef.current;
    if (!container) {
      return;
    }

    let rafId = 0;
    const updateActiveSlide = (): void => {
      rafId = 0;
      const nextIndex = getCurrentCenteredSlideIndex();
      if (nextIndex >= 0) {
        setActiveSlideIndex((previous) => (previous === nextIndex ? previous : nextIndex));
      }
    };

    const requestUpdate = (): void => {
      if (rafId !== 0) {
        return;
      }
      rafId = window.requestAnimationFrame(updateActiveSlide);
    };

    container.addEventListener("scroll", requestUpdate, { passive: true });
    window.addEventListener("resize", requestUpdate);
    requestUpdate();

    return () => {
      if (rafId !== 0) {
        window.cancelAnimationFrame(rafId);
      }
      container.removeEventListener("scroll", requestUpdate);
      window.removeEventListener("resize", requestUpdate);
    };
  }, [embeddedSlideCount, presenterMode, selectedProject]);

  async function refreshState(preferredPath = ""): Promise<string> {
    const nextState = await call<AppState>("get_app_state");
    setAppState(nextState);

    const fallbackPath = nextState.projects[0]?.path || "";
    const hasPreferred = preferredPath && nextState.projects.some((item) => item.path === preferredPath);
    const nextSelection = hasPreferred ? preferredPath : fallbackPath;
    setSelectedPath(nextSelection);
    return nextSelection;
  }

  useEffect(() => {
    let cancelled = false;

    async function boot() {
      try {
        await refreshState();
        if (!cancelled) {
          console.log("Ready");
        }
      } catch (error) {
        if (!cancelled) {
          const message = error instanceof Error ? error.message : "Failed to load app state.";
          console.log(message);
        }
      }
    }

    void boot();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    return () => {
      sidebarResizeCleanupRef.current?.();
      document.body.classList.remove("sidebar-resize-active");
    };
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let disposed = false;
    let unlisten: (() => void) | null = null;

    async function registerMenuListener() {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const cleanupOpenSettings = await listen(OPEN_SETTINGS_MENU_EVENT, () => {
          openSettingsPanel();
        });
        const cleanupExportSkill = await listen(EXPORT_SKILL_MENU_EVENT, () => {
          void handleExportSkillArchive();
        });

        if (disposed) {
          cleanupOpenSettings();
          cleanupExportSkill();
          return;
        }

        unlisten = () => {
          cleanupOpenSettings();
          cleanupExportSkill();
        };
      } catch (error) {
        const message =
          error instanceof Error ? error.message : "Failed to register application menu listener.";
        console.log(message);
      }
    }

    void registerMenuListener();

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      const usingCommand = event.metaKey || event.ctrlKey;
      if (usingCommand && (event.key === "," || event.code === "Comma")) {
        event.preventDefault();
        openSettingsPanel();
        return;
      }

      const target = event.target as HTMLElement | null;
      const targetTag = target?.tagName;
      const isTypingTarget =
        Boolean(target?.isContentEditable) || targetTag === "INPUT" || targetTag === "TEXTAREA" || targetTag === "SELECT";
      if (isTypingTarget) {
        return;
      }

      if (usingCommand && (event.key === "=" || event.key === "+" || event.key === "Add")) {
        event.preventDefault();
        setPreviewZoom((previous) => clampPreviewZoom(previous + PREVIEW_ZOOM_STEP));
        return;
      }

      if (usingCommand && (event.key === "-" || event.key === "_" || event.key === "Subtract")) {
        event.preventDefault();
        setPreviewZoom((previous) => clampPreviewZoom(previous - PREVIEW_ZOOM_STEP));
        return;
      }

      if (usingCommand && (event.key === "b" || event.key === "B")) {
        event.preventDefault();
        setSidebarOpen((open) => !open);
        return;
      }

      if (event.key === "Escape" && settingsOpen) {
        event.preventDefault();
        setSettingsOpen(false);
        return;
      }

      if (event.key === "Escape" && expandedAsset) {
        event.preventDefault();
        setExpandedAsset(null);
        return;
      }

      if (event.key === "Escape" && presenterMode) {
        event.preventDefault();
        setPresenterMode(false);
        window.requestAnimationFrame(() => {
          scrollListToSlide(activeSlideIndex, "instant");
        });
        revealPreviewDock();
        return;
      }

      if (expandedAsset) {
        return;
      }

      if (!presenterMode || !selectedProject) {
        return;
      }

      if (event.key === "ArrowRight" || event.key === "PageDown" || event.key === " ") {
        event.preventDefault();
        setActiveSlideIndex((previous) => Math.min(previous + 1, maxSlideIndex));
        return;
      }

      if (event.key === "ArrowLeft" || event.key === "PageUp") {
        event.preventDefault();
        setActiveSlideIndex((previous) => Math.max(previous - 1, 0));
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [activeSlideIndex, expandedAsset, maxSlideIndex, presenterMode, selectedProject, settingsOpen]);

  function clearPreviewDockHideTimer(): void {
    if (previewDockHideTimerRef.current !== null) {
      window.clearTimeout(previewDockHideTimerRef.current);
      previewDockHideTimerRef.current = null;
    }
  }

  function schedulePreviewDockHide(delayMs: number): void {
    clearPreviewDockHideTimer();
    previewDockHideTimerRef.current = window.setTimeout(() => {
      if (!previewDockHoveringRef.current) {
        setPreviewDockVisible(false);
      }
      previewDockHideTimerRef.current = null;
    }, delayMs);
  }

  function revealPreviewDock(hideDelayMs = 1400): void {
    setPreviewDockVisible(true);
    schedulePreviewDockHide(hideDelayMs);
  }

  function togglePresenterMode(): void {
    if (presenterMode) {
      setPresenterMode(false);
      window.requestAnimationFrame(() => {
        scrollListToSlide(activeSlideIndex, "instant");
      });
      revealPreviewDock();
      return;
    }

    const centeredSlide = getCurrentCenteredSlideIndex();
    if (centeredSlide >= 0) {
      setActiveSlideIndex(centeredSlide);
    }
    setPresenterMode(true);
    revealPreviewDock();
  }

  function handlePreviewStagePointerMove(event: ReactPointerEvent<HTMLDivElement>): void {
    const stageBounds = event.currentTarget.getBoundingClientRect();
    const distanceFromBottom = stageBounds.bottom - event.clientY;
    if (distanceFromBottom <= 148) {
      revealPreviewDock();
      return;
    }
    if (!previewDockHoveringRef.current) {
      schedulePreviewDockHide(220);
    }
  }

  function handlePreviewStagePointerLeave(): void {
    if (!previewDockHoveringRef.current) {
      schedulePreviewDockHide(140);
    }
  }

  function handlePreviewDockPointerEnter(): void {
    previewDockHoveringRef.current = true;
    clearPreviewDockHideTimer();
    setPreviewDockVisible(true);
  }

  function handlePreviewDockPointerLeave(): void {
    previewDockHoveringRef.current = false;
    schedulePreviewDockHide(180);
  }

  async function handleOpenAsset(asset: ExpandableAsset): Promise<void> {
    const projectPath = selectedProject?.path || "";
    let source = asset.src;
    try {
      source = await resolveProjectAssetSource(projectPath, asset.src);
    } catch {
      source = asset.src;
    }
    setExpandedAsset({
      ...asset,
      src: source || asset.src,
    });
  }

  async function withBusy(task: () => Promise<void>): Promise<void> {
    setBusy(true);
    try {
      await task();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Action failed.";
      console.log(message);
    } finally {
      setBusy(false);
    }
  }

  async function handleOpenProjectFolder(): Promise<void> {
    await withBusy(async () => {
      const folder = await pickFolder("Select a project folder containing page.mdx");
      if (!folder) {
        console.log("Open project cancelled.");
        return;
      }

      const opened = await call<ProjectDetail>("open_project", { path: folder });
      await refreshState(opened.path);
      console.log(`Opened project ${opened.name}.`);
    });
  }

  async function handleRemoveProject(path: string): Promise<void> {
    try {
      const nextState = await call<AppState>("remove_project", { path });
      setAppState(nextState);

      const fallbackPath = nextState.projects[0]?.path || "";
      const preferredPath = selectedPath === path ? "" : selectedPath;
      const hasPreferred = preferredPath && nextState.projects.some((project) => project.path === preferredPath);
      const nextSelection = hasPreferred ? preferredPath : fallbackPath;
      setSelectedPath(nextSelection);

      console.log("Removed project from tracked list.");
    } catch (error) {
      console.error("Failed to remove project:", error);
    }
  }

  async function handleTogglePin(path: string): Promise<void> {
    try {
      const nextState = await call<AppState>("toggle_project_pin", { path });
      setAppState(nextState);
    } catch (error) {
      console.error("Failed to toggle pin state:", error);
    }
  }

  function openSettingsPanel(): void {
    setExpandedAsset(null);
    setSettingsOpen(true);
    setSidebarOpen(true);
  }

  function updateToken<K extends keyof SlideTokens>(key: K, value: string): void {
    setSlideTokens((prev) => {
      const next = { ...prev, [key]: value };
      const css = tokensToCss(next);
      setProjectCss(css);
      setCssEditorValue(css);
      return next;
    });
  }

  async function handleSaveCss(): Promise<void> {
    if (!selectedProject) return;
    const css = tokensToCss(slideTokens);
    await withBusy(async () => {
      await call<void>("save_project_css", { path: selectedProject.path, css });
      setProjectCss(css);
      setCssEditorValue(css);
    });
  }

  async function handleExportSkillArchive(): Promise<void> {
    await withBusy(async () => {
      if (!isTauriRuntime()) {
        throw new Error("Skill export is available only in Tauri runtime.");
      }

      const { save } = await import("@tauri-apps/plugin-dialog");
      const destination = await save({
        title: "Download FastSlides Skill",
        defaultPath: "fastslides-skill.zip",
        canCreateDirectories: true,
        filters: [
          {
            name: "ZIP archive",
            extensions: ["zip"],
          },
        ],
      });

      if (!destination || typeof destination !== "string") {
        console.log("Skill export cancelled.");
        return;
      }

      const exportedPath = await call<string>("export_fastslides_skill", {
        destination,
      });
      console.log(`FastSlides skill exported to ${exportedPath}.`);
    });
  }

  function handleSidebarResizeStart(event: ReactPointerEvent<HTMLDivElement>): void {
    if (!sidebarOpen) {
      return;
    }

    event.preventDefault();

    const startX = event.clientX;
    const startWidth = sidebarWidth;
    const pointerId = event.pointerId;
    const resizeHandle = event.currentTarget;

    try {
      resizeHandle.setPointerCapture(pointerId);
    } catch {
      // Ignore capture failures; window listeners still provide fallback behavior.
    }

    setSidebarDragging(true);
    document.body.classList.add("sidebar-resize-active");

    const handlePointerMove = (moveEvent: PointerEvent): void => {
      const deltaX = moveEvent.clientX - startX;
      setSidebarWidth(clampSidebarWidth(startWidth + deltaX));
    };

    const stopResize = (): void => {
      setSidebarDragging(false);
      document.body.classList.remove("sidebar-resize-active");
      try {
        if (resizeHandle.hasPointerCapture(pointerId)) {
          resizeHandle.releasePointerCapture(pointerId);
        }
      } catch {
        // Ignore release failures.
      }
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", stopResize);
      window.removeEventListener("pointercancel", stopResize);
      sidebarResizeCleanupRef.current = null;
    };

    sidebarResizeCleanupRef.current?.();
    sidebarResizeCleanupRef.current = stopResize;

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", stopResize);
    window.addEventListener("pointercancel", stopResize);
  }

  const shellStyle = useMemo(
    () =>
      ({
        "--sidebar-runtime-width": `${sidebarWidth}px`,
      }) as CSSProperties,
    [sidebarWidth],
  );

  const previewSurfaceStyle = useMemo(
    () =>
      ({
        "--preview-zoom": `${previewZoom}`,
      }) as CSSProperties,
    [previewZoom],
  );

  return (
    <main
      className={`app-shell ${sidebarOpen ? "sidebar-open" : "sidebar-closed"} ${sidebarDragging ? "sidebar-resizing" : ""}`}
      style={shellStyle}
    >
      <SidebarToggleButton
        sidebarOpen={sidebarOpen}
        onToggle={() => setSidebarOpen((open) => !open)}
      />

      <AppSidebar
        busy={busy}
        sidebarOpen={sidebarOpen}
        settingsOpen={settingsOpen}
        projectsCount={projects.length}
        projects={projects}
        pinnedPaths={appState?.config.pinned_projects || []}
        selectedPath={selectedPath}
        onBackToApp={() => setSettingsOpen(false)}
        onOpenProject={() => {
          void handleOpenProjectFolder();
        }}
        onSelectProject={setSelectedPath}
        onRemoveProject={(path) => {
          void handleRemoveProject(path);
        }}
        onTogglePin={(path) => {
          void handleTogglePin(path);
        }}
        onOpenSettings={openSettingsPanel}
      />

      <SidebarResizer
        sidebarWidth={sidebarWidth}
        sidebarOpen={sidebarOpen}
        minWidth={SIDEBAR_MIN_WIDTH}
        maxWidth={SIDEBAR_MAX_WIDTH}
        onPointerDown={handleSidebarResizeStart}
      />

      <PreviewWorkspace
        settingsOpen={settingsOpen}
        hasSelectedProject={Boolean(selectedProject)}
        presenterMode={presenterMode}
        previewSurfaceStyle={previewSurfaceStyle}
        previewSurfaceRef={previewSurfaceRef}
        deckPreview={
          selectedProject ? (
            <EmbeddedDeckPreview
              source={selectedProjectEmbeddedSource}
              projectPath={selectedProject.path}
              mermaidThemeName={mermaidThemeName}
              syntaxThemeName={syntaxThemeName}
              uiTheme={theme}
              presenterMode={presenterMode}
              activeSlideIndex={activeSlideIndex}
              onSlideCountChange={setEmbeddedSlideCount}
              onActiveSlidePick={setActiveSlideIndex}
              onSlideOutlineChange={setSlideOutline}
              onAssetOpen={(asset) => {
                void handleOpenAsset(asset);
              }}
            />
          ) : null
        }
        slideTocEntries={slideTocEntries}
        activeSlideIndex={activeSlideIndex}
        onTocSelect={handleTocSelect}
        previewDockVisible={previewDockVisible}
        onPreviewDockPointerEnter={handlePreviewDockPointerEnter}
        onPreviewDockPointerLeave={handlePreviewDockPointerLeave}
        onTogglePresenterMode={togglePresenterMode}
        onPreviewStagePointerMove={handlePreviewStagePointerMove}
        onPreviewStagePointerLeave={handlePreviewStagePointerLeave}
      />

      <AssetLightbox
        asset={expandedAsset}
        onClose={() => {
          setExpandedAsset(null);
        }}
      />

      <SettingsOverlay
        open={settingsOpen}
        busy={busy}
        theme={theme}
        onThemeChange={(nextTheme) => setTheme(nextTheme)}
        mermaidThemeName={mermaidThemeName}
        mermaidThemeOptions={MERMAID_THEME_OPTIONS}
        mermaidThemeLabels={MERMAID_THEME_LABELS}
        onMermaidThemeChange={(value) => {
          if (isMermaidThemeName(value)) {
            setMermaidThemeName(value);
          }
        }}
        syntaxThemeName={syntaxThemeName}
        syntaxThemeOptionsByMode={SYNTAX_THEME_OPTIONS_BY_MODE}
        syntaxThemeLabels={SYNTAX_THEME_LABELS}
        onSyntaxThemeChange={(value) => {
          if (isSyntaxThemeName(value)) {
            setSyntaxThemeName(value);
          }
        }}
        selectedProject={Boolean(selectedProject)}
        slideTokens={slideTokens}
        onUpdateToken={(key, value) => updateToken(key as keyof SlideTokens, value)}
        onSaveCss={() => {
          void handleSaveCss();
        }}
        fontOptions={FONT_OPTIONS}
        monoFontOptions={MONO_FONT_OPTIONS}
        fontLabel={fontLabel}
        hexForInput={hexForInput}
        isHexColor={isHexColor}
      />
    </main>
  );
}
