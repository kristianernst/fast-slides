"use client";

import {
  createContext,
  startTransition,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type HTMLAttributes,
  type ImgHTMLAttributes,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
  type VideoHTMLAttributes,
} from "react";
import { invoke } from "@tauri-apps/api/core";
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

type SettingsTab = "theme" | "library";

type ComponentCatalogEntry = {
  name: string;
  family: string;
  kind: string;
  scope: string;
  summary: string;
  tags: string[];
};

type ComponentCatalog = {
  version: string;
  items: ComponentCatalogEntry[];
};

type DesignTemplate = {
  kind: string;
  name: string;
  mdx: string;
  notes: string[];
};

type ProjectScene = {
  path: string;
  project: string | null;
  title: string | null;
  subtitle: string | null;
  date: string | null;
  deck_class_name: string | null;
  slide_count: number;
  slides: SceneSlide[];
};

type SceneSlide = {
  index: number;
  title: string;
  layout: SceneLayout;
  nodes: SceneNode[];
  source_mdx: string;
};

type SceneSlideManifest = {
  index: number;
  title: string;
  layout: SceneLayout;
};

type ProjectSceneManifest = Omit<ProjectScene, "slides"> & {
  slides: SceneSlideManifest[];
};

type SceneLayout = {
  kind: string;
  cols: number | null;
  rows: number | null;
  gap: string | null;
};

type SceneSlideLoadStatus = "loading" | "ready" | "error";

type PreviewSceneSlide = SceneSlideManifest & {
  nodes: SceneNode[];
  source_mdx: string;
  status: SceneSlideLoadStatus;
  error: string | null;
};

type PreviewProjectScene = Omit<ProjectScene, "slides"> & {
  slides: PreviewSceneSlide[];
};

type ProjectSceneSessionHandle = {
  session_id: string;
  path: string;
  slide_count: number;
};

type ProjectSceneSessionEvent =
  | {
      session_id: string;
      sequence: number;
      kind: "manifest";
      scene: ProjectSceneManifest;
    }
  | {
      session_id: string;
      sequence: number;
      kind: "slide-ready";
      slide: SceneSlide;
    }
  | {
      session_id: string;
      sequence: number;
      kind: "slide-error";
      index: number;
      error: string;
    }
  | {
      session_id: string;
      sequence: number;
      kind: "complete";
      ready_count: number;
      error_count: number;
    };

type SceneCanvasNode = {
  kind: "canvas";
  cols: number;
  rows: number;
  gap: string | null;
  class_name: string | null;
  children: SceneNode[];
  source_mdx: string;
};

type SceneAreaNode = {
  kind: "area";
  x: number;
  y: number;
  w: number;
  h: number;
  layer: number | null;
  gap: string | null;
  align: string | null;
  justify: string | null;
  class_name: string | null;
  children: SceneNode[];
  source_mdx: string;
};

type SceneLayoutGroupNode = {
  kind: "layout-group";
  component: string;
  cols: number | null;
  gap: string | null;
  align: string | null;
  justify: string | null;
  nowrap: boolean | null;
  class_name: string | null;
  children: SceneNode[];
  source_mdx: string;
};

type SceneSurfaceNode = {
  kind: "surface";
  component: string;
  tone: string | null;
  title: string | null;
  kicker: string | null;
  subtitle: string | null;
  foot: string | null;
  attribution: string | null;
  class_name: string | null;
  children: SceneNode[];
  source_mdx: string;
};

type SceneMetricNode = {
  kind: "metric";
  label: string | null;
  value: string | null;
  hint: string | null;
  class_name: string | null;
  source_mdx: string;
};

type SceneChartDatum = {
  label: string;
  value: number;
};

type SceneChartNode = {
  kind: "chart";
  chart_type: string;
  title: string | null;
  tone: string | null;
  value_suffix: string | null;
  highlight: string | null;
  data: SceneChartDatum[];
  class_name: string | null;
  source_mdx: string;
};

type SceneTextNode = {
  kind: "text";
  role: string;
  text: string;
  level: number | null;
  class_name: string | null;
};

type SceneListNode = {
  kind: "list";
  ordered: boolean;
  items: string[];
};

type SceneMediaNode = {
  kind: "media";
  media_kind: string;
  src: string;
  alt: string | null;
};

type SceneCodeBlockNode = {
  kind: "code-block";
  language: string | null;
  code: string;
};

type ScenePillNode = {
  kind: "pill";
  tone: string | null;
  text: string;
  class_name: string | null;
};

type SceneRuleNode = {
  kind: "rule";
  class_name: string | null;
};

type SceneArrowNode = {
  kind: "arrow";
  direction: string | null;
  tone: string | null;
  label: string | null;
  class_name: string | null;
  source_mdx: string;
};

type SceneRawNode = {
  kind: "raw";
  format: string;
  text: string;
};

type SceneNode =
  | SceneCanvasNode
  | SceneAreaNode
  | SceneLayoutGroupNode
  | SceneSurfaceNode
  | SceneMetricNode
  | SceneChartNode
  | SceneTextNode
  | SceneListNode
  | SceneMediaNode
  | SceneCodeBlockNode
  | ScenePillNode
  | SceneRuleNode
  | SceneArrowNode
  | SceneRawNode;

type SlideOutlineEntry = {
  index: number;
  title: string;
  status?: SceneSlideLoadStatus;
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
  if (
    (Object.keys(LAYOUT_GAP_MULTIPLIER) as LayoutGapScale[]).includes(
      normalized as LayoutGapScale,
    )
  ) {
    return normalized as LayoutGapScale;
  }
  return "md";
}

function normalizeLayoutAlign(value: unknown): LayoutAlign {
  if (typeof value !== "string") {
    return "stretch";
  }
  const normalized = value.trim().toLowerCase();
  if (
    normalized === "start" ||
    normalized === "center" ||
    normalized === "end" ||
    normalized === "stretch"
  ) {
    return normalized;
  }
  return "stretch";
}

function normalizeLayoutJustify(value: unknown): LayoutJustify {
  if (typeof value !== "string") {
    return "start";
  }
  const normalized = value.trim().toLowerCase();
  if (
    normalized === "start" ||
    normalized === "center" ||
    normalized === "end" ||
    normalized === "between"
  ) {
    return normalized;
  }
  return "start";
}

function normalizeGridColumns(value: unknown): 1 | 2 | 3 | 4 {
  if (typeof value === "number") {
    if (value === 1 || value === 2 || value === 3 || value === 4) {
      return value;
    }
    return 2;
  }
  if (typeof value === "string") {
    const parsed = Number.parseInt(value, 10);
    if (parsed === 1 || parsed === 2 || parsed === 3 || parsed === 4) {
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
  if (
    normalized === "default" ||
    normalized === "accent" ||
    normalized === "success" ||
    normalized === "warning" ||
    normalized === "danger"
  ) {
    return normalized;
  }
  return "default";
}

function layoutGapCssValue(gap: unknown): string {
  if (typeof gap === "number" && Number.isFinite(gap) && gap >= 0) {
    return `${gap}px`;
  }
  if (typeof gap === "string") {
    const trimmed = gap.trim();
    if (trimmed.length > 0) {
      if (
        (Object.keys(LAYOUT_GAP_MULTIPLIER) as LayoutGapScale[]).includes(
          trimmed as LayoutGapScale,
        )
      ) {
        return `calc(var(--slide-layout-gap, 16px) * ${LAYOUT_GAP_MULTIPLIER[trimmed as LayoutGapScale]})`;
      }
      return trimmed;
    }
  }
  const normalized = normalizeLayoutGap(gap);
  return `calc(var(--slide-layout-gap, 16px) * ${LAYOUT_GAP_MULTIPLIER[normalized]})`;
}

function normalizePositiveInt(
  value: unknown,
  fallback: number,
  min = 1,
  max = 100,
): number {
  if (typeof value === "number" && Number.isFinite(value)) {
    return Math.min(max, Math.max(min, Math.round(value)));
  }
  if (typeof value === "string") {
    const parsed = Number.parseInt(value, 10);
    if (Number.isFinite(parsed)) {
      return Math.min(max, Math.max(min, parsed));
    }
  }
  return fallback;
}

function layoutAlignCssValue(align: unknown): CSSProperties["alignItems"] {
  const normalized = normalizeLayoutAlign(align);
  if (normalized === "start") return "flex-start";
  if (normalized === "end") return "flex-end";
  if (normalized === "center") return "center";
  return "stretch";
}

function layoutJustifyCssValue(
  justify: unknown,
): CSSProperties["justifyContent"] {
  const normalized = normalizeLayoutJustify(justify);
  if (normalized === "start") return "flex-start";
  if (normalized === "end") return "flex-end";
  if (normalized === "center") return "center";
  return "space-between";
}

type AreaShape = "hero" | "banner" | "rail" | "panel" | "tile";
type AreaDensity = "tight" | "balanced" | "roomy";
type MetricVariant = "compact" | "standard" | "feature";
type CalloutVariant = "compact" | "rail" | "standard";
type TakeawayScale = "compact" | "balanced" | "hero";
type PanelVariant = "compact" | "standard" | "roomy";

type LayoutScope = {
  canvasCols: number;
  canvasRows: number;
  areaCols: number | null;
  areaRows: number | null;
  areaShape: AreaShape;
  areaDensity: AreaDensity;
  gridCols: number | null;
};

const DEFAULT_LAYOUT_SCOPE: LayoutScope = {
  canvasCols: 50,
  canvasRows: 25,
  areaCols: null,
  areaRows: null,
  areaShape: "panel",
  areaDensity: "balanced",
  gridCols: null,
};

const LayoutScopeContext = createContext<LayoutScope>(DEFAULT_LAYOUT_SCOPE);

function inferAreaShape(cols: number, rows: number): AreaShape {
  if (cols >= 28 && rows <= 6) {
    return "hero";
  }
  if (cols >= 20 && rows <= 4) {
    return "banner";
  }
  if (cols <= 15 && rows >= 8) {
    return "rail";
  }
  if (cols <= 12 && rows <= 6) {
    return "tile";
  }
  return "panel";
}

function inferAreaDensity(cols: number, rows: number): AreaDensity {
  const footprint = cols * rows;
  if (footprint <= 70) {
    return "tight";
  }
  if (footprint >= 150) {
    return "roomy";
  }
  return "balanced";
}

function inferTakeawayScale(
  scope: LayoutScope,
  textLength: number,
): TakeawayScale {
  const areaCols = scope.areaCols ?? 30;
  const areaRows = scope.areaRows ?? 5;
  if (
    scope.areaShape === "banner" ||
    areaRows <= 4 ||
    (areaCols <= 30 && textLength >= 90)
  ) {
    return "compact";
  }
  if (scope.areaShape === "hero" && areaCols >= 30 && textLength <= 72) {
    return "hero";
  }
  return "balanced";
}

function inferMetricVariant(
  scope: LayoutScope,
  valueLength: number,
  hintLength: number,
): MetricVariant {
  const areaCols = scope.areaCols ?? 10;
  const areaRows = scope.areaRows ?? 5;
  const gridCols = scope.gridCols ?? 1;
  if (
    gridCols >= 3 ||
    areaCols <= 10 ||
    areaRows <= 5 ||
    valueLength >= 9 ||
    hintLength > 22
  ) {
    return "compact";
  }
  if (areaCols >= 12 && areaRows >= 6 && valueLength <= 7) {
    return "feature";
  }
  return "standard";
}

function inferCalloutVariant(
  scope: LayoutScope,
  bodyLength: number,
): CalloutVariant {
  const areaCols = scope.areaCols ?? 14;
  const areaRows = scope.areaRows ?? 10;
  if (areaCols <= 13 || areaRows <= 9 || bodyLength >= 120) {
    return "compact";
  }
  if (scope.areaShape === "rail" || areaCols <= 15) {
    return "rail";
  }
  return "standard";
}

function inferPanelVariant(
  scope: LayoutScope,
  bodyLength: number,
): PanelVariant {
  const areaCols = scope.areaCols ?? 18;
  const areaRows = scope.areaRows ?? 11;
  if (areaCols <= 20 || areaRows <= 11 || bodyLength >= 120) {
    return "compact";
  }
  if (areaCols >= 24 && areaRows >= 12 && bodyLength <= 88) {
    return "roomy";
  }
  return "standard";
}

function useLayoutScope(): LayoutScope {
  return useContext(LayoutScopeContext);
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
  cols?: 1 | 2 | 3 | 4 | string | number;
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
  const parentScope = useLayoutScope();
  const normalizedCols = normalizeGridColumns(cols);
  const density: AreaDensity =
    normalizedCols >= 3 && (parentScope.areaCols ?? 0) <= 32
      ? "tight"
      : "balanced";
  const scope = useMemo(
    () => ({
      ...parentScope,
      gridCols: normalizedCols,
    }),
    [parentScope, normalizedCols],
  );
  return (
    <LayoutScopeContext.Provider value={scope}>
      <div
        {...props}
        className={["mdx-grid", className].filter(Boolean).join(" ")}
        data-cols={normalizedCols}
        data-density={density}
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
    </LayoutScopeContext.Provider>
  );
}

type MdxCanvasProps = HTMLAttributes<HTMLDivElement> & {
  cols?: number | string;
  rows?: number | string;
  gap?: LayoutGapScale | string | number;
};

function MdxCanvas({
  children,
  className,
  style,
  cols = 50,
  rows = 25,
  gap = "1px",
  ...props
}: MdxCanvasProps) {
  const normalizedCols = normalizePositiveInt(cols, 50, 1, 100);
  const normalizedRows = normalizePositiveInt(rows, 25, 1, 100);
  const scope = useMemo(
    () => ({
      ...DEFAULT_LAYOUT_SCOPE,
      canvasCols: normalizedCols,
      canvasRows: normalizedRows,
    }),
    [normalizedCols, normalizedRows],
  );

  return (
    <LayoutScopeContext.Provider value={scope}>
      <div
        {...props}
        className={["mdx-canvas", className].filter(Boolean).join(" ")}
        data-cols={normalizedCols}
        data-rows={normalizedRows}
        style={{
          display: "grid",
          gridTemplateColumns: `repeat(${normalizedCols}, minmax(0, 1fr))`,
          gridTemplateRows: `repeat(${normalizedRows}, minmax(0, 1fr))`,
          minWidth: 0,
          minHeight: 0,
          width: "100%",
          height: "100%",
          flex: "1 1 0%",
          gap: layoutGapCssValue(gap),
          ...style,
        }}
      >
        {children}
      </div>
    </LayoutScopeContext.Provider>
  );
}

type MdxAreaProps = HTMLAttributes<HTMLDivElement> & {
  x?: number | string;
  y?: number | string;
  w?: number | string;
  h?: number | string;
  layer?: number | string;
  gap?: LayoutGapScale | string | number;
  align?: LayoutAlign | string;
  justify?: LayoutJustify | string;
};

function MdxArea({
  children,
  className,
  style,
  x = 1,
  y = 1,
  w = 1,
  h = 1,
  layer = 1,
  gap = "md",
  align = "stretch",
  justify = "start",
  ...props
}: MdxAreaProps) {
  const parentScope = useLayoutScope();
  const startColumn = normalizePositiveInt(x, 1, 1, 100);
  const startRow = normalizePositiveInt(y, 1, 1, 100);
  const spanColumns = normalizePositiveInt(w, 1, 1, 100);
  const spanRows = normalizePositiveInt(h, 1, 1, 100);
  const zIndex = normalizePositiveInt(layer, 1, 1, 10);
  const areaShape = inferAreaShape(spanColumns, spanRows);
  const areaDensity = inferAreaDensity(spanColumns, spanRows);
  const areaStyle: CSSProperties = {
    gridColumn: `${startColumn} / span ${spanColumns}`,
    gridRow: `${startRow} / span ${spanRows}`,
    minWidth: 0,
    minHeight: 0,
    display: "flex",
    flexDirection: "column",
    gap: layoutGapCssValue(gap),
    alignItems: layoutAlignCssValue(align),
    justifyContent: layoutJustifyCssValue(justify),
    zIndex,
    ...style,
  };
  const areaStyleWithVars = areaStyle as CSSProperties & Record<string, string>;
  areaStyleWithVars["--fs-area-cols"] = String(spanColumns);
  areaStyleWithVars["--fs-area-rows"] = String(spanRows);
  const scope = useMemo(
    () => ({
      ...parentScope,
      areaCols: spanColumns,
      areaRows: spanRows,
      areaShape,
      areaDensity,
      gridCols: null,
    }),
    [parentScope, spanColumns, spanRows, areaShape, areaDensity],
  );

  return (
    <LayoutScopeContext.Provider value={scope}>
      <div
        {...props}
        className={["mdx-area", className].filter(Boolean).join(" ")}
        data-area-shape={areaShape}
        data-area-density={areaDensity}
        data-area-cols={spanColumns}
        data-area-rows={spanRows}
        style={areaStyleWithVars}
      >
        {children}
      </div>
    </LayoutScopeContext.Provider>
  );
}

function cssLengthValue(
  value: string | number | undefined,
): string | undefined {
  if (typeof value === "number" && Number.isFinite(value)) {
    return `${value}px`;
  }
  if (typeof value === "string" && value.trim().length > 0) {
    return value.trim();
  }
  return undefined;
}

function formatChartValue(value: number, suffix?: string | null): string {
  const rounded =
    Math.abs(value % 1) < 0.001
      ? String(Math.round(value))
      : value.toFixed(Math.abs(value) >= 10 ? 0 : 1).replace(/\.0$/, "");
  return `${rounded}${suffix ?? ""}`;
}

function chartColorForTone(
  tone: CardTone,
  index: number,
  highlighted: boolean,
): string {
  if (highlighted) {
    if (tone === "success")
      return "var(--slide-palette-2, var(--slide-accent, #7b9cbc))";
    if (tone === "warning")
      return "var(--slide-palette-3, var(--slide-accent, #7b9cbc))";
    if (tone === "danger")
      return "var(--slide-palette-4, var(--slide-accent, #7b9cbc))";
    return "var(--slide-accent, var(--slide-palette-1, #7b9cbc))";
  }
  const paletteIndex = (index % 5) + 1;
  return `var(--slide-palette-${paletteIndex}, var(--slide-accent, #7b9cbc))`;
}

function MdxKicker({
  children,
  className,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      {...props}
      className={["mdx-kicker", className].filter(Boolean).join(" ")}
    >
      {children}
    </div>
  );
}

type TakeawayTag = "h1" | "h2" | "h3";

type MdxTakeawayProps = HTMLAttributes<HTMLHeadingElement> & {
  as?: TakeawayTag;
  maxWidth?: string | number;
  textLength?: number;
};

function MdxTakeaway({
  children,
  className,
  style,
  as = "h1",
  maxWidth,
  textLength,
  ...props
}: MdxTakeawayProps) {
  const layoutScope = useLayoutScope();
  const scale = inferTakeawayScale(layoutScope, textLength ?? 0);
  const Tag = as;
  return (
    <Tag
      {...props}
      className={["mdx-takeaway", className].filter(Boolean).join(" ")}
      data-scale={scale}
      style={{
        maxWidth: cssLengthValue(maxWidth),
        ...style,
      }}
    >
      {children}
    </Tag>
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
      className={["mdx-card", `mdx-card--${normalizedTone}`, className]
        .filter(Boolean)
        .join(" ")}
    >
      {title ? <h3 className="mdx-card-title">{title}</h3> : null}
      {subtitle ? (
        <p className="mdx-caption mdx-card-subtitle">{subtitle}</p>
      ) : null}
      <div className="mdx-card-body">{children}</div>
    </article>
  );
}

type MdxPanelProps = HTMLAttributes<HTMLDivElement> & {
  kicker?: ReactNode;
  title?: ReactNode;
  subtitle?: ReactNode;
  foot?: ReactNode;
  tone?: CardTone | string;
  bodyLength?: number;
};

function MdxPanel({
  children,
  className,
  kicker,
  title,
  subtitle,
  foot,
  tone = "default",
  bodyLength = 0,
  ...props
}: MdxPanelProps) {
  const layoutScope = useLayoutScope();
  const variant = inferPanelVariant(layoutScope, bodyLength);
  const normalizedTone = normalizeCardTone(tone);
  return (
    <section
      {...props}
      className={["mdx-panel", `mdx-panel--${normalizedTone}`, className]
        .filter(Boolean)
        .join(" ")}
      data-variant={variant}
      data-area-shape={layoutScope.areaShape}
      data-area-density={layoutScope.areaDensity}
    >
      {kicker || title || subtitle ? (
        <div className="mdx-panel-head">
          {kicker ? <div className="mdx-kicker">{kicker}</div> : null}
          {title ? <h3 className="mdx-panel-title">{title}</h3> : null}
          {subtitle ? <p className="mdx-panel-subtitle">{subtitle}</p> : null}
        </div>
      ) : null}
      <div className="mdx-panel-body">{children}</div>
      {foot ? <div className="mdx-panel-foot">{foot}</div> : null}
    </section>
  );
}

type MdxCalloutProps = HTMLAttributes<HTMLDivElement> & {
  title?: ReactNode;
  tone?: CardTone | string;
  bodyLength?: number;
};

function MdxCallout({
  children,
  className,
  title,
  tone = "default",
  bodyLength = 0,
  ...props
}: MdxCalloutProps) {
  const layoutScope = useLayoutScope();
  const variant = inferCalloutVariant(layoutScope, bodyLength);
  const normalizedTone = normalizeCardTone(tone);
  return (
    <aside
      {...props}
      className={["mdx-callout", `mdx-callout--${normalizedTone}`, className]
        .filter(Boolean)
        .join(" ")}
      data-variant={variant}
      data-area-shape={layoutScope.areaShape}
      data-area-density={layoutScope.areaDensity}
    >
      {title ? <div className="mdx-callout-title">{title}</div> : null}
      <div className="mdx-callout-body">{children}</div>
    </aside>
  );
}

type MdxMetricProps = HTMLAttributes<HTMLDivElement> & {
  label?: ReactNode;
  value?: ReactNode;
  hint?: ReactNode;
  valueLength?: number;
  hintLength?: number;
};

function MdxMetric({
  className,
  label,
  value,
  hint,
  children,
  valueLength = 0,
  hintLength = 0,
  ...props
}: MdxMetricProps) {
  const layoutScope = useLayoutScope();
  const variant = inferMetricVariant(layoutScope, valueLength, hintLength);
  return (
    <article
      {...props}
      className={["mdx-metric", className].filter(Boolean).join(" ")}
      data-variant={variant}
      data-area-shape={layoutScope.areaShape}
      data-area-density={layoutScope.areaDensity}
    >
      {label ? <p className="mdx-caption mdx-metric-label">{label}</p> : null}
      <p className="mdx-metric-value">{value ?? children}</p>
      {hint ? <p className="mdx-caption mdx-metric-hint">{hint}</p> : null}
    </article>
  );
}

type MdxChartProps = HTMLAttributes<HTMLDivElement> & {
  chartType?: string | null;
  title?: ReactNode;
  tone?: CardTone | string;
  valueSuffix?: string | null;
  highlight?: string | null;
  data: SceneChartDatum[];
};

function MdxChart({
  className,
  chartType,
  title,
  tone = "default",
  valueSuffix,
  highlight,
  data,
  style,
  ...props
}: MdxChartProps) {
  const layoutScope = useLayoutScope();
  const normalizedTone = normalizeCardTone(tone);
  const normalizedType =
    chartType?.trim().toLowerCase() === "trend" ? "trend" : "bar";
  const safeData = data.filter(
    (item) =>
      typeof item.label === "string" &&
      item.label.trim().length > 0 &&
      Number.isFinite(item.value),
  );
  const maxValue = safeData.reduce(
    (max, item) => Math.max(max, Math.abs(item.value)),
    0,
  );
  const trendPoints = safeData.map((item, index) => {
    const width = 100;
    const height = 58;
    const left = 6;
    const right = 96;
    const top = 6;
    const bottom = 52;
    const span = Math.max(maxValue, 1);
    const x =
      safeData.length <= 1
        ? (left + right) / 2
        : left + ((right - left) * index) / (safeData.length - 1);
    const y = bottom - (item.value / span) * (bottom - top);
    return { ...item, x, y };
  });
  const trendPath = trendPoints
    .map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`)
    .join(" ");
  const trendAreaPath =
    trendPoints.length > 1
      ? `${trendPath} L ${trendPoints[trendPoints.length - 1].x} 58 L ${trendPoints[0].x} 58 Z`
      : "";
  const chartAccent = chartColorForTone(normalizedTone, 0, true);
  const chartStyle = {
    ...(style ?? {}),
    "--mdx-chart-accent": chartAccent,
  } as CSSProperties;

  return (
    <section
      {...props}
      className={["mdx-chart", className].filter(Boolean).join(" ")}
      style={chartStyle}
      data-chart-type={normalizedType}
      data-tone={normalizedTone}
      data-area-shape={layoutScope.areaShape}
      data-area-density={layoutScope.areaDensity}
    >
      {title ? <div className="mdx-chart-title">{title}</div> : null}
      {normalizedType === "trend" ? (
        <div className="mdx-chart-trend">
          <svg
            className="mdx-chart-svg"
            viewBox="0 0 100 64"
            preserveAspectRatio="none"
            aria-hidden="true"
          >
            <line x1="6" y1="52" x2="96" y2="52" className="mdx-chart-grid" />
            <line x1="6" y1="29" x2="96" y2="29" className="mdx-chart-grid" />
            <line x1="6" y1="6" x2="96" y2="6" className="mdx-chart-grid" />
            {trendAreaPath ? (
              <path d={trendAreaPath} className="mdx-chart-area" />
            ) : null}
            {trendPath ? (
              <path
                d={trendPath}
                className="mdx-chart-line"
                style={{
                  stroke: chartAccent,
                }}
              />
            ) : null}
            {trendPoints.map((point, index) => {
              const emphasized =
                point.label === highlight || index === trendPoints.length - 1;
              return (
                <circle
                  key={`${point.label}-${index}`}
                  cx={point.x}
                  cy={point.y}
                  r={emphasized ? 2.4 : 2}
                  className="mdx-chart-point"
                  style={{
                    fill: chartColorForTone(normalizedTone, index, emphasized),
                  }}
                />
              );
            })}
          </svg>
          <div className="mdx-chart-trend-labels">
            {safeData.map((item) => (
              <div key={item.label} className="mdx-chart-trend-label">
                <span>{item.label}</span>
                <strong>{formatChartValue(item.value, valueSuffix)}</strong>
              </div>
            ))}
          </div>
        </div>
      ) : (
        <div className="mdx-chart-bars">
          {safeData.map((item, index) => {
            const emphasized = item.label === highlight;
            const width =
              maxValue > 0
                ? `${(Math.abs(item.value) / maxValue) * 100}%`
                : "0%";
            return (
              <div key={`${item.label}-${index}`} className="mdx-chart-bar-row">
                <span className="mdx-chart-bar-label">{item.label}</span>
                <div className="mdx-chart-bar-track">
                  <span
                    className="mdx-chart-bar-fill"
                    data-emphasized={emphasized ? "true" : "false"}
                    style={{
                      width,
                      background: chartColorForTone(
                        normalizedTone,
                        index,
                        emphasized,
                      ),
                    }}
                  />
                </div>
                <strong className="mdx-chart-bar-value">
                  {formatChartValue(item.value, valueSuffix)}
                </strong>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}

type MdxPillRowProps = HTMLAttributes<HTMLDivElement> & {
  gap?: LayoutGapScale | string;
};

function MdxPillRow({
  children,
  className,
  style,
  gap = "sm",
  ...props
}: MdxPillRowProps) {
  return (
    <div
      {...props}
      className={["mdx-pill-row", className].filter(Boolean).join(" ")}
      style={{
        gap: layoutGapCssValue(gap),
        ...style,
      }}
    >
      {children}
    </div>
  );
}

type MdxPillProps = HTMLAttributes<HTMLSpanElement> & {
  tone?: CardTone | string;
};

function MdxPill({
  children,
  className,
  tone = "default",
  ...props
}: MdxPillProps) {
  const normalizedTone = normalizeCardTone(tone);
  return (
    <span
      {...props}
      className={["mdx-pill", `mdx-pill--${normalizedTone}`, className]
        .filter(Boolean)
        .join(" ")}
    >
      {children}
    </span>
  );
}

function MdxCaption({
  children,
  className,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      {...props}
      className={["mdx-caption", className].filter(Boolean).join(" ")}
    >
      {children}
    </div>
  );
}

type MdxQuoteProps = HTMLAttributes<HTMLQuoteElement> & {
  attribution?: ReactNode;
};

function MdxQuote({
  children,
  className,
  attribution,
  ...props
}: MdxQuoteProps) {
  return (
    <blockquote
      {...props}
      className={["mdx-quote", className].filter(Boolean).join(" ")}
    >
      <div className="mdx-quote-body">{children}</div>
      {attribution ? (
        <footer className="mdx-quote-attribution">{attribution}</footer>
      ) : null}
    </blockquote>
  );
}

function MdxRule({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      {...props}
      className={["mdx-rule", className].filter(Boolean).join(" ")}
    />
  );
}

type MdxArrowProps = HTMLAttributes<HTMLDivElement> & {
  direction?: string | null;
  tone?: CardTone | string;
  label?: ReactNode;
};

function MdxArrow({
  className,
  direction,
  tone = "default",
  label,
  ...props
}: MdxArrowProps) {
  const normalizedTone = normalizeCardTone(tone);
  const normalizedDirection =
    direction === "left" || direction === "up" || direction === "down"
      ? direction
      : "right";
  return (
    <div
      {...props}
      className={["mdx-arrow", className].filter(Boolean).join(" ")}
      data-direction={normalizedDirection}
      data-tone={normalizedTone}
    >
      {label ? <span className="mdx-arrow-label">{label}</span> : null}
      <span className="mdx-arrow-line" />
      <span className="mdx-arrow-head" />
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

async function call<T>(
  command: string,
  payload?: Record<string, unknown>,
): Promise<T> {
  if (isTauriRuntime()) {
    return invoke<T>(command, payload);
  }

  const path = typeof payload?.path === "string" ? payload.path : "";

  let response: Response;
  switch (command) {
    case "get_app_state":
      response = await fetch(`${AGENT_HOOK_BASE_URL}/app-state`);
      break;
    case "open_project":
    case "load_project":
      response = await fetch(`${AGENT_HOOK_BASE_URL}/open-project`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path }),
      });
      break;
    case "analyze_project":
      response = await fetch(
        `${AGENT_HOOK_BASE_URL}/analyze-project?path=${encodeURIComponent(path)}`,
      );
      break;
    case "compile_project_scene":
      response = await fetch(
        `${AGENT_HOOK_BASE_URL}/compile-project-scene?path=${encodeURIComponent(path)}`,
      );
      break;
    case "get_component_catalog":
      response = await fetch(`${AGENT_HOOK_BASE_URL}/component-catalog`);
      break;
    case "get_component_template": {
      const name = typeof payload?.name === "string" ? payload.name : "";
      response = await fetch(
        `${AGENT_HOOK_BASE_URL}/component-template?name=${encodeURIComponent(name)}`,
      );
      break;
    }
    case "save_component_template":
      response = await fetch(`${AGENT_HOOK_BASE_URL}/save-component-template`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload ?? {}),
      });
      break;
    case "compile_project_scene_manifest":
      response = await fetch(
        `${AGENT_HOOK_BASE_URL}/compile-project-scene-manifest?path=${encodeURIComponent(path)}`,
      );
      break;
    case "compile_project_scene_slide": {
      const index = typeof payload?.index === "number" ? payload.index : -1;
      response = await fetch(
        `${AGENT_HOOK_BASE_URL}/compile-project-scene-slide?path=${encodeURIComponent(path)}&index=${encodeURIComponent(String(index))}`,
      );
      break;
    }
    case "read_project_css":
      response = await fetch(
        `${AGENT_HOOK_BASE_URL}/project-css?path=${encodeURIComponent(path)}`,
      );
      break;
    case "save_project_css":
      response = await fetch(`${AGENT_HOOK_BASE_URL}/project-css`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path,
          css: typeof payload?.css === "string" ? payload.css : "",
        }),
      });
      break;
    default:
      throw new Error(
        `Command \`${command}\` is unavailable outside Tauri runtime.`,
      );
  }

  const data = await response.json().catch(() => null);
  if (!response.ok) {
    const errorMessage =
      data &&
      typeof data === "object" &&
      "error" in data &&
      typeof data.error === "string"
        ? data.error
        : `Hook request failed for \`${command}\`.`;
    throw new Error(errorMessage);
  }

  return data as T;
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

const SYNTAX_THEME_STYLES: Record<
  SyntaxThemeName,
  Record<string, CSSProperties>
> = {
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
  slideBorder: "#00000000",
  slideRadius: "10px",
  slidePadding: "24px",
  slideLayoutGap: "14px",
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
  "system-ui, sans-serif",
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
const MAX_SCENE_SLIDE_WORKERS = 6;
const PROJECT_ROOT_ABSOLUTE_ASSET_PREFIXES = [
  "/assets/",
  "/images/",
  "/media/",
  "/data/",
];
const projectAssetDataUrlCache = new Map<string, string>();
const IMAGE_ASSET_EXTENSION_RE = /\.(avif|bmp|gif|jpe?g|png|svg|webp)$/i;
const VIDEO_ASSET_EXTENSION_RE = /\.(m4v|mov|mp4|ogv|ogg|webm)$/i;
const AGENT_HOOK_BASE_URL = "http://127.0.0.1:38473";
const SCENE_SESSION_EVENT_NAME = "fastslides://scene-session-event";

type InitialPreviewRouteState = {
  deckPath: string;
  slideIndex: number | null;
  presenterMode: boolean;
};

function readInitialPreviewRouteState(): InitialPreviewRouteState {
  if (typeof window === "undefined") {
    return { deckPath: "", slideIndex: null, presenterMode: false };
  }

  const params = new URLSearchParams(window.location.search);
  const deckPath = params.get("deckPath")?.trim() || "";
  const rawSlide = params.get("slide")?.trim() || "";
  const parsedSlide = Number.parseInt(rawSlide, 10);
  const slideIndex =
    Number.isFinite(parsedSlide) && parsedSlide > 0 ? parsedSlide - 1 : null;
  const rawPresenter = params.get("presenter")?.trim().toLowerCase() || "";
  const presenterMode =
    rawPresenter === "1" || rawPresenter === "true" || rawPresenter === "yes";

  return { deckPath, slideIndex, presenterMode };
}

function nextSceneSessionId(): string {
  return `scene-session-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

function preferredSceneWorkerCount(): number {
  if (typeof navigator === "undefined") {
    return 4;
  }
  const concurrency = navigator.hardwareConcurrency || 8;
  return Math.min(
    MAX_SCENE_SLIDE_WORKERS,
    Math.max(2, Math.floor(concurrency / 2)),
  );
}

function createPreviewProjectScene(
  manifest: ProjectSceneManifest,
): PreviewProjectScene {
  return {
    ...manifest,
    slides: manifest.slides.map((slide) => ({
      ...slide,
      nodes: [],
      source_mdx: "",
      status: "loading",
      error: null,
    })),
  };
}

function mergeCompiledSceneSlide(
  current: PreviewProjectScene,
  compiledSlide: SceneSlide,
): PreviewProjectScene {
  return {
    ...current,
    slides: current.slides.map((slide) =>
      slide.index === compiledSlide.index
        ? {
            ...slide,
            ...compiledSlide,
            status: "ready",
            error: null,
          }
        : slide,
    ),
  };
}

function markSceneSlideError(
  current: PreviewProjectScene,
  index: number,
  error: string,
): PreviewProjectScene {
  return {
    ...current,
    slides: current.slides.map((slide) =>
      slide.index === index
        ? {
            ...slide,
            nodes: [],
            source_mdx: "",
            status: "error",
            error,
          }
        : slide,
    ),
  };
}

function applySceneSessionEvent(
  current: PreviewProjectScene | null,
  event: ProjectSceneSessionEvent,
): PreviewProjectScene | null {
  switch (event.kind) {
    case "manifest":
      return createPreviewProjectScene(event.scene);
    case "slide-ready":
      return current ? mergeCompiledSceneSlide(current, event.slide) : current;
    case "slide-error":
      return current
        ? markSceneSlideError(current, event.index, event.error)
        : current;
    case "complete":
      return current;
    default:
      return current;
  }
}

function prioritizeSceneSlideIndices(
  total: number,
  startIndex: number,
): number[] {
  if (total <= 0) {
    return [];
  }

  const clampedStart = Math.min(Math.max(startIndex, 0), total - 1);
  const ordered: number[] = [];

  for (let offset = 0; ordered.length < total; offset += 1) {
    const forward = clampedStart + offset;
    if (forward < total) {
      ordered.push(forward);
    }
    const backward = clampedStart - offset;
    if (offset > 0 && backward >= 0) {
      ordered.push(backward);
    }
  }

  return ordered;
}

function takeNextSceneSlideIndex(
  pending: number[],
  preferredIndex: number,
): number | null {
  if (pending.length === 0) {
    return null;
  }

  let bestPosition = pending.indexOf(preferredIndex);
  if (bestPosition < 0) {
    let bestDistance = Number.POSITIVE_INFINITY;
    bestPosition = 0;
    for (let position = 0; position < pending.length; position += 1) {
      const distance = Math.abs(pending[position] - preferredIndex);
      if (distance < bestDistance) {
        bestDistance = distance;
        bestPosition = position;
      }
    }
  }

  const [nextIndex] = pending.splice(bestPosition, 1);
  return typeof nextIndex === "number" ? nextIndex : null;
}

function summarizePreviewScene(scene: PreviewProjectScene | null) {
  if (!scene) {
    return null;
  }

  let readyCount = 0;
  let errorCount = 0;
  for (const slide of scene.slides) {
    if (slide.status === "ready") {
      readyCount += 1;
    } else if (slide.status === "error") {
      errorCount += 1;
    }
  }

  const totalCount = scene.slide_count;
  const loadingCount = Math.max(totalCount - readyCount - errorCount, 0);
  return {
    totalCount,
    readyCount,
    errorCount,
    loadingCount,
    complete: loadingCount === 0,
  };
}

function clampPreviewZoom(zoom: number): number {
  return Math.min(PREVIEW_ZOOM_MAX, Math.max(PREVIEW_ZOOM_MIN, zoom));
}

function clampSidebarWidth(width: number): number {
  return Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, width));
}

function normalizeRawHtmlFragment(source: string): string {
  return source
    .replace(/\bclassName=/g, "class=")
    .replace(/\bhtmlFor=/g, "for=");
}

function canRenderRawHtmlFragment(source: string): boolean {
  if (!source.trim().startsWith("<")) {
    return false;
  }
  if (/[{}]/.test(source)) {
    return false;
  }
  return /<[a-z][\w-]*\b/.test(source);
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

function splitAssetPathAndSuffix(raw: string): {
  pathOnly: string;
  suffix: string;
} {
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
      (prefix) =>
        relative === prefix.slice(0, -1) || relative.startsWith(prefix),
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

    const theme =
      THEMES[mermaidThemeName as keyof typeof THEMES] ?? THEMES["zinc-dark"];
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
          const message =
            cause instanceof Error
              ? cause.message
              : "Failed to render Mermaid diagram.";
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

function CodeBlockView({
  code,
  language,
  mermaidThemeName,
  syntaxThemeName,
  uiTheme,
}: {
  code: string;
  language?: string | null;
  mermaidThemeName: MermaidThemeName;
  syntaxThemeName: SyntaxThemeName;
  uiTheme: "dark" | "light";
}) {
  const normalizedLanguage = language?.trim().toLowerCase() ?? "";
  if (normalizedLanguage === "mermaid") {
    return <MermaidDiagram code={code} mermaidThemeName={mermaidThemeName} />;
  }

  const syntaxTheme =
    SYNTAX_THEME_STYLES[syntaxThemeName] ||
    (uiTheme === "light" ? oneLight : oneDark);

  return (
    <SyntaxHighlighter
      className="slide-syntax-block"
      language={normalizedLanguage || "text"}
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
      showLineNumbers={code.split("\n").length > 6}
      lineNumberStyle={{
        color: "var(--color-text-tertiary)",
        opacity: 0.85,
        paddingRight: "0.85rem",
      }}
    >
      {code}
    </SyntaxHighlighter>
  );
}

async function resolveProjectAssetSource(
  projectPath: string,
  rawSrc: string,
): Promise<string> {
  const normalizedRelative = normalizeProjectRelativeAsset(rawSrc);
  if (!normalizedRelative || !projectPath) {
    return rawSrc;
  }

  if (!isTauriRuntime()) {
    const params = new URLSearchParams({
      projectPath,
      src: normalizedRelative,
    });
    return `${AGENT_HOOK_BASE_URL}/project-asset?${params.toString()}`;
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

  return (
    <img
      {...props}
      src={resolvedSrc || source}
      loading={props.loading ?? "lazy"}
    />
  );
}

function ProjectAssetVideo({
  projectPath,
  ...props
}: VideoHTMLAttributes<HTMLVideoElement> & { projectPath: string }) {
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
        if (!cancelled) {
          setResolvedSrc(nextSource);
        }
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

  return (
    <video
      {...props}
      src={resolvedSrc || source}
      controls={props.controls ?? true}
    />
  );
}

type SceneRenderContext = {
  projectPath: string;
  mermaidThemeName: MermaidThemeName;
  syntaxThemeName: SyntaxThemeName;
  uiTheme: "dark" | "light";
};

function renderSceneNodes(
  nodes: SceneNode[],
  context: SceneRenderContext,
  pathPrefix: string,
): ReactNode[] {
  return nodes.map((node, index) =>
    renderSceneNode(node, context, `${pathPrefix}-${index}`),
  );
}

function sceneNodesTextLength(nodes: SceneNode[]): number {
  return nodes.reduce((total, node) => {
    switch (node.kind) {
      case "canvas":
      case "area":
      case "layout-group":
      case "surface":
        return total + sceneNodesTextLength(node.children);
      case "metric":
        return (
          total +
          (node.label?.length ?? 0) +
          (node.value?.length ?? 0) +
          (node.hint?.length ?? 0)
        );
      case "chart":
        return (
          total +
          (node.title?.length ?? 0) +
          (node.value_suffix?.length ?? 0) +
          node.data.reduce((sum, item) => sum + item.label.length, 0)
        );
      case "text":
        return total + node.text.length;
      case "list":
        return total + node.items.reduce((sum, item) => sum + item.length, 0);
      case "media":
        return total + (node.alt?.length ?? 0);
      case "code-block":
        return total + node.code.length;
      case "pill":
        return total + node.text.length;
      case "raw":
        return total + node.text.length;
      case "rule":
        return total;
      case "arrow":
        return total + (node.label?.length ?? 0);
      default:
        return total;
    }
  }, 0);
}

function renderSceneTextNode(node: SceneTextNode, key: string): ReactNode {
  const className = node.class_name || undefined;
  if (node.role === "kicker") {
    return (
      <MdxKicker key={key} className={className}>
        {node.text}
      </MdxKicker>
    );
  }
  if (node.role === "takeaway") {
    const headingTag = node.level === 2 ? "h2" : node.level === 3 ? "h3" : "h1";
    return (
      <MdxTakeaway
        key={key}
        as={headingTag}
        className={className}
        textLength={node.text.length}
      >
        {node.text}
      </MdxTakeaway>
    );
  }
  if (node.role === "caption") {
    return (
      <MdxCaption key={key} className={className}>
        {node.text}
      </MdxCaption>
    );
  }
  if (node.role === "heading") {
    if (node.level === 2) {
      return (
        <h2 key={key} className={className}>
          {node.text}
        </h2>
      );
    }
    if (node.level === 3) {
      return (
        <h3 key={key} className={className}>
          {node.text}
        </h3>
      );
    }
    return (
      <h1 key={key} className={className}>
        {node.text}
      </h1>
    );
  }
  return (
    <p key={key} className={className}>
      {node.text}
    </p>
  );
}

function renderSceneNode(
  node: SceneNode,
  context: SceneRenderContext,
  key: string,
): ReactNode {
  switch (node.kind) {
    case "canvas":
      return (
        <MdxCanvas
          key={key}
          cols={node.cols}
          rows={node.rows}
          gap={node.gap ?? undefined}
          className={node.class_name || undefined}
        >
          {renderSceneNodes(node.children, context, key)}
        </MdxCanvas>
      );
    case "area":
      return (
        <MdxArea
          key={key}
          x={node.x}
          y={node.y}
          w={node.w}
          h={node.h}
          layer={node.layer ?? undefined}
          gap={node.gap ?? undefined}
          align={node.align ?? undefined}
          justify={node.justify ?? undefined}
          className={node.class_name || undefined}
        >
          {renderSceneNodes(node.children, context, key)}
        </MdxArea>
      );
    case "layout-group":
      if (node.component === "Stack") {
        return (
          <MdxStack
            key={key}
            gap={node.gap ?? undefined}
            align={node.align ?? undefined}
            justify={node.justify ?? undefined}
            className={node.class_name || undefined}
          >
            {renderSceneNodes(node.children, context, key)}
          </MdxStack>
        );
      }
      if (node.component === "Row") {
        return (
          <MdxRow
            key={key}
            gap={node.gap ?? undefined}
            align={node.align ?? undefined}
            justify={node.justify ?? undefined}
            nowrap={node.nowrap ?? false}
            className={node.class_name || undefined}
          >
            {renderSceneNodes(node.children, context, key)}
          </MdxRow>
        );
      }
      if (node.component === "Grid") {
        return (
          <MdxGrid
            key={key}
            cols={node.cols ?? undefined}
            gap={node.gap ?? undefined}
            align={node.align ?? undefined}
            className={node.class_name || undefined}
          >
            {renderSceneNodes(node.children, context, key)}
          </MdxGrid>
        );
      }
      if (node.component === "PillRow") {
        return (
          <MdxPillRow
            key={key}
            gap={node.gap ?? undefined}
            className={node.class_name || undefined}
          >
            {renderSceneNodes(node.children, context, key)}
          </MdxPillRow>
        );
      }
      return (
        <div
          key={key}
          className={["scene-unknown-node", node.class_name]
            .filter(Boolean)
            .join(" ")}
        >
          {renderSceneNodes(node.children, context, key)}
        </div>
      );
    case "surface":
      if (node.component === "Card") {
        return (
          <MdxCard
            key={key}
            title={node.title ?? undefined}
            subtitle={node.subtitle ?? undefined}
            tone={node.tone ?? undefined}
            className={node.class_name || undefined}
          >
            {renderSceneNodes(node.children, context, key)}
          </MdxCard>
        );
      }
      if (node.component === "Panel") {
        return (
          <MdxPanel
            key={key}
            kicker={node.kicker ?? undefined}
            title={node.title ?? undefined}
            subtitle={node.subtitle ?? undefined}
            foot={node.foot ?? undefined}
            tone={node.tone ?? undefined}
            bodyLength={sceneNodesTextLength(node.children)}
            className={node.class_name || undefined}
          >
            {renderSceneNodes(node.children, context, key)}
          </MdxPanel>
        );
      }
      if (node.component === "Callout") {
        return (
          <MdxCallout
            key={key}
            title={node.title ?? undefined}
            tone={node.tone ?? undefined}
            bodyLength={sceneNodesTextLength(node.children)}
            className={node.class_name || undefined}
          >
            {renderSceneNodes(node.children, context, key)}
          </MdxCallout>
        );
      }
      if (node.component === "Quote") {
        return (
          <MdxQuote
            key={key}
            attribution={node.attribution ?? undefined}
            className={node.class_name || undefined}
          >
            {renderSceneNodes(node.children, context, key)}
          </MdxQuote>
        );
      }
      return (
        <div
          key={key}
          className={["scene-unknown-node", node.class_name]
            .filter(Boolean)
            .join(" ")}
        >
          {renderSceneNodes(node.children, context, key)}
        </div>
      );
    case "metric":
      return (
        <MdxMetric
          key={key}
          label={node.label ?? undefined}
          value={node.value ?? undefined}
          hint={node.hint ?? undefined}
          valueLength={node.value?.length ?? 0}
          hintLength={node.hint?.length ?? 0}
          className={node.class_name || undefined}
        />
      );
    case "chart":
      return (
        <MdxChart
          key={key}
          chartType={node.chart_type}
          title={node.title ?? undefined}
          tone={node.tone ?? undefined}
          valueSuffix={node.value_suffix}
          highlight={node.highlight}
          data={node.data}
          className={node.class_name || undefined}
        />
      );
    case "text":
      return renderSceneTextNode(node, key);
    case "list":
      if (node.ordered) {
        return (
          <ol key={key}>
            {node.items.map((item, index) => (
              <li key={`${key}-${index}`}>{item}</li>
            ))}
          </ol>
        );
      }
      return (
        <ul key={key}>
          {node.items.map((item, index) => (
            <li key={`${key}-${index}`}>{item}</li>
          ))}
        </ul>
      );
    case "media":
      if (node.media_kind === "video") {
        return (
          <ProjectAssetVideo
            key={key}
            projectPath={context.projectPath}
            src={node.src}
            aria-label={node.alt || "Slide video"}
          />
        );
      }
      return (
        <ProjectAssetImage
          key={key}
          projectPath={context.projectPath}
          src={node.src}
          alt={node.alt || "Slide image"}
        />
      );
    case "code-block":
      return (
        <CodeBlockView
          key={key}
          code={node.code}
          language={node.language}
          mermaidThemeName={context.mermaidThemeName}
          syntaxThemeName={context.syntaxThemeName}
          uiTheme={context.uiTheme}
        />
      );
    case "pill":
      return (
        <MdxPill
          key={key}
          tone={node.tone ?? undefined}
          className={node.class_name || undefined}
        >
          {node.text}
        </MdxPill>
      );
    case "rule":
      return <MdxRule key={key} className={node.class_name || undefined} />;
    case "arrow":
      return (
        <MdxArrow
          key={key}
          direction={node.direction}
          tone={node.tone ?? undefined}
          label={node.label ?? undefined}
          className={node.class_name || undefined}
        />
      );
    case "raw": {
      const normalizedHtml = normalizeRawHtmlFragment(node.text);
      if (node.format === "html" && canRenderRawHtmlFragment(normalizedHtml)) {
        return (
          <div
            key={key}
            className="scene-raw-html"
            dangerouslySetInnerHTML={{ __html: normalizedHtml }}
          />
        );
      }
      return (
        <pre key={key} className="scene-raw-node">
          <code>{node.text}</code>
        </pre>
      );
    }
    default:
      return null;
  }
}

function renderSlidePlaceholder(
  slide: PreviewSceneSlide,
  detailed: boolean,
): ReactNode {
  if (slide.status === "error" || detailed) {
    return (
      <div
        className={`embedded-slide-placeholder embedded-slide-placeholder-${slide.status}`}
      >
        <div className="embedded-slide-placeholder-meta">
          Slide {slide.index + 1}
        </div>
        <h2>{slide.title || `Slide ${slide.index + 1}`}</h2>
        <p>
          {slide.status === "error"
            ? slide.error || "This slide failed to compile."
            : "Rendering slide scene…"}
        </p>
      </div>
    );
  }

  return (
    <div
      className="embedded-slide-placeholder embedded-slide-placeholder-loading"
      data-quiet="true"
      aria-label={`Rendering ${slide.title || `slide ${slide.index + 1}`}`}
    >
      <span className="embedded-slide-placeholder-skeleton embedded-slide-placeholder-skeleton-kicker" />
      <span className="embedded-slide-placeholder-skeleton embedded-slide-placeholder-skeleton-title" />
      <span className="embedded-slide-placeholder-skeleton embedded-slide-placeholder-skeleton-body" />
      <span className="embedded-slide-placeholder-skeleton embedded-slide-placeholder-skeleton-body is-short" />
    </div>
  );
}

function EmbeddedDeckPreview({
  scene,
  error,
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
  scene: PreviewProjectScene | null;
  error: string;
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
  const previewRootRef = useRef<HTMLDivElement | null>(null);
  const slideElementsRef = useRef<HTMLElement[]>([]);
  const previousActiveSlideRef = useRef(-1);
  const sceneRenderContext = useMemo<SceneRenderContext>(
    () => ({
      projectPath,
      mermaidThemeName,
      syntaxThemeName,
      uiTheme,
    }),
    [mermaidThemeName, projectPath, syntaxThemeName, uiTheme],
  );

  useEffect(() => {
    if (!scene) {
      onSlideCountChange(0);
      onSlideOutlineChange([]);
      return;
    }

    onSlideCountChange(scene.slides.length);
    onSlideOutlineChange(
      scene.slides.map((slide, index) => ({
        index,
        title: slide.title || `Slide ${index + 1}`,
        status: slide.status,
      })),
    );
  }, [onSlideCountChange, onSlideOutlineChange, scene]);

  useEffect(() => {
    const root = previewRootRef.current;
    if (!root || !scene) {
      slideElementsRef.current = [];
      previousActiveSlideRef.current = -1;
      return;
    }

    const slides = Array.from(root.querySelectorAll<HTMLElement>(".slide"));
    slideElementsRef.current = slides;
    previousActiveSlideRef.current = -1;
    root.classList.remove("embedded-preview-single");

    if (slides.length === 0) {
      return;
    }

    for (let index = 0; index < slides.length; index += 1) {
      slides[index].dataset.active = "false";
      slides[index].setAttribute("aria-hidden", "false");
    }
  }, [scene?.path, scene?.slide_count]);

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
  }, [activeSlideIndex, presenterMode, scene?.path, scene?.slide_count]);

  useEffect(() => {
    const root = previewRootRef.current;
    if (!root) {
      return;
    }

    const onClick = (event: MouseEvent): void => {
      const target = event.target as HTMLElement | null;

      const image = target?.closest("img") as HTMLImageElement | null;
      if (image) {
        const rawSource =
          image.currentSrc || image.src || image.getAttribute("src") || "";
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
        const sourceNode =
          video.querySelector<HTMLSourceElement>("source[src]");
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
            alt:
              video.getAttribute("aria-label") ||
              video.getAttribute("title") ||
              "Slide video",
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

  if (!scene) {
    return (
      <div className="embedded-preview-loading">Loading deck preview…</div>
    );
  }

  return (
    <div ref={previewRootRef} className="embedded-preview-deck">
      <div
        className={Array.from(
          new Set(
            ["deck", scene.deck_class_name || ""]
              .join(" ")
              .split(/\s+/)
              .filter(Boolean),
          ),
        ).join(" ")}
      >
        {scene.slides.map((slide) => (
          <section
            key={slide.index}
            className={`slide slide-${slide.status}`}
            data-slide-index={slide.index}
            data-slide-status={slide.status}
            aria-busy={slide.status === "loading"}
          >
            {slide.status === "ready"
              ? renderSceneNodes(
                  slide.nodes,
                  sceneRenderContext,
                  `slide-${slide.index}`,
                )
              : renderSlidePlaceholder(
                  slide,
                  presenterMode && slide.index === activeSlideIndex,
                )}
          </section>
        ))}
      </div>
    </div>
  );
}

export default function Home() {
  const initialPreviewRoute = useMemo(readInitialPreviewRouteState, []);
  const [appState, setAppState] = useState<AppState | null>(null);
  const [selectedPath, setSelectedPath] = useState("");
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [sidebarWidth, setSidebarWidth] = useState(280);
  const [sidebarDragging, setSidebarDragging] = useState(false);
  const [selectedProjectDetail, setSelectedProjectDetail] =
    useState<ProjectDetail | null>(null);
  const [presenterMode, setPresenterMode] = useState(false);
  const [activeSlideIndex, setActiveSlideIndex] = useState(0);
  const [embeddedSlideCount, setEmbeddedSlideCount] = useState(0);
  const [slideOutline, setSlideOutline] = useState<SlideOutlineEntry[]>([]);
  const [projectScene, setProjectScene] = useState<PreviewProjectScene | null>(
    null,
  );
  const [projectSceneError, setProjectSceneError] = useState("");
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
  const [mermaidThemeName, setMermaidThemeName] = useState<MermaidThemeName>(
    () => {
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
    },
  );
  const [syntaxThemeName, setSyntaxThemeName] = useState<SyntaxThemeName>(
    () => {
      if (typeof window !== "undefined") {
        const stored = localStorage.getItem(SYNTAX_THEME_STATE_KEY);
        if (stored && isSyntaxThemeName(stored)) {
          return stored;
        }
        const storedUiTheme =
          localStorage.getItem(THEME_STATE_KEY) === "light" ? "light" : "dark";
        const mode = syntaxThemeModeForUiTheme(storedUiTheme);
        return SYNTAX_THEME_OPTIONS_BY_MODE[mode][0];
      }
      return SYNTAX_THEME_OPTIONS_BY_MODE.dark[0];
    },
  );
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsTab, setSettingsTab] = useState<SettingsTab>("theme");
  const [expandedAsset, setExpandedAsset] = useState<ExpandableAsset | null>(
    null,
  );
  const [projectCss, setProjectCss] = useState("");
  const [cssEditorValue, setCssEditorValue] = useState("");
  const [slideTokens, setSlideTokens] = useState<SlideTokens>({
    ...DEFAULT_TOKENS,
  });
  const [componentCatalog, setComponentCatalog] =
    useState<ComponentCatalog | null>(null);
  const [componentCatalogLoading, setComponentCatalogLoading] = useState(false);
  const [componentCatalogError, setComponentCatalogError] = useState("");
  const [selectedComponentName, setSelectedComponentName] = useState("");
  const [selectedComponentTemplate, setSelectedComponentTemplate] =
    useState<DesignTemplate | null>(null);
  const [componentTemplateLoading, setComponentTemplateLoading] =
    useState(false);
  const [componentTemplateError, setComponentTemplateError] = useState("");
  const sidebarResizeCleanupRef = useRef<(() => void) | null>(null);
  const previewDockHideTimerRef = useRef<number | null>(null);
  const previewDockHoveringRef = useRef(false);
  const previewSurfaceRef = useRef<HTMLDivElement | null>(null);
  const initialPreviewRouteAppliedRef = useRef(false);
  const sceneCompileRequestIdRef = useRef(0);
  const sceneSessionIdRef = useRef("");
  const sceneCompilePumpRef = useRef<(() => void) | null>(null);
  const sceneCompilePriorityIndexRef = useRef(0);

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

  const visibleSlideCount = useMemo(() => {
    const fallbackCount =
      projectScene?.slide_count ||
      selectedProjectDetail?.slide_count ||
      selectedProject?.slide_count ||
      0;
    return embeddedSlideCount > 0 ? embeddedSlideCount : fallbackCount;
  }, [
    embeddedSlideCount,
    projectScene?.slide_count,
    selectedProject?.slide_count,
    selectedProjectDetail?.slide_count,
  ]);

  const maxSlideIndex = Math.max(visibleSlideCount - 1, 0);
  const sceneLoadSummary = useMemo(
    () => summarizePreviewScene(projectScene),
    [projectScene],
  );
  const slideTocEntries = useMemo<SlideOutlineEntry[]>(() => {
    if (slideOutline.length > 0) {
      return slideOutline;
    }
    if (projectScene?.slides.length) {
      return projectScene.slides.map((slide, index) => ({
        index,
        title: slide.title || `Slide ${index + 1}`,
        status: slide.status,
      }));
    }
    return Array.from({ length: visibleSlideCount }, (_, index) => ({
      index,
      title: `Slide ${index + 1}`,
    }));
  }, [projectScene?.slides, slideOutline, visibleSlideCount]);

  const previewStatusLabel = useMemo(() => {
    if (!selectedProject || projectSceneError) {
      return "";
    }
    if (!projectScene) {
      return "Preparing slide manifest…";
    }
    if (
      !sceneLoadSummary ||
      sceneLoadSummary.totalCount === 0 ||
      sceneLoadSummary.complete
    ) {
      return "";
    }
    return `Rendering slides ${sceneLoadSummary.readyCount}/${sceneLoadSummary.totalCount}`;
  }, [projectScene, projectSceneError, sceneLoadSummary, selectedProject]);

  useEffect(() => {
    setActiveSlideIndex(0);
    setEmbeddedSlideCount(0);
    setSlideOutline([]);
    setPresenterMode(false);
  }, [selectedProject?.path]);

  useEffect(() => {
    sceneCompilePriorityIndexRef.current = activeSlideIndex;
    sceneCompilePumpRef.current?.();
  }, [activeSlideIndex, selectedProject?.path]);

  useEffect(() => {
    if (initialPreviewRouteAppliedRef.current) {
      return;
    }
    if (!selectedProject || !projectScene) {
      return;
    }

    const requestedSlideIndex = initialPreviewRoute.slideIndex;
    const requestedPresenterMode = initialPreviewRoute.presenterMode;
    if (requestedSlideIndex === null && !requestedPresenterMode) {
      initialPreviewRouteAppliedRef.current = true;
      return;
    }

    const maxIndex = Math.max(projectScene.slide_count - 1, 0);
    setActiveSlideIndex(
      Math.min(Math.max(requestedSlideIndex ?? 0, 0), maxIndex),
    );
    setPresenterMode(requestedPresenterMode);
    initialPreviewRouteAppliedRef.current = true;
  }, [
    initialPreviewRoute.presenterMode,
    initialPreviewRoute.slideIndex,
    projectScene,
    selectedProject,
  ]);

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
      setProjectScene(null);
      setProjectSceneError("");
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
          const message =
            cause instanceof Error
              ? cause.message
              : "Failed to load project source.";
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
      setProjectScene(null);
      setProjectSceneError("");
      sceneSessionIdRef.current = "";
      sceneCompilePumpRef.current = null;
      return;
    }

    let cancelled = false;
    const requestId = sceneCompileRequestIdRef.current + 1;
    sceneCompileRequestIdRef.current = requestId;
    sceneCompilePriorityIndexRef.current = activeSlideIndex;
    const selectedProjectPath = selectedProject.path;
    setProjectScene(null);
    setProjectSceneError("");

    const isCurrentRequest = (): boolean =>
      !cancelled && sceneCompileRequestIdRef.current === requestId;

    const updateScene = (
      updater: (
        current: PreviewProjectScene | null,
      ) => PreviewProjectScene | null,
    ): void => {
      if (!isCurrentRequest()) {
        return;
      }
      startTransition(() => {
        setProjectScene((current) => {
          if (!isCurrentRequest()) {
            return current;
          }
          return updater(current);
        });
      });
    };

    if (isTauriRuntime()) {
      sceneCompilePumpRef.current = null;
      let unlistenSceneSession: (() => void) | null = null;

      async function startNativeSceneSession(): Promise<void> {
        try {
          const { listen } = await import("@tauri-apps/api/event");
          const cleanup = await listen<ProjectSceneSessionEvent>(
            SCENE_SESSION_EVENT_NAME,
            (event) => {
              if (!isCurrentRequest()) {
                return;
              }
              const payload = event.payload;
              if (
                !payload ||
                payload.session_id !== sceneSessionIdRef.current
              ) {
                return;
              }
              updateScene((current) =>
                applySceneSessionEvent(current, payload),
              );
            },
          );

          if (!isCurrentRequest()) {
            cleanup();
            return;
          }

          unlistenSceneSession = cleanup;
          const nextSessionId = nextSceneSessionId();
          sceneSessionIdRef.current = nextSessionId;
          const session = await call<ProjectSceneSessionHandle>(
            "start_project_scene_session",
            {
              path: selectedProjectPath,
              priorityIndex: activeSlideIndex,
              sessionId: nextSessionId,
            },
          );

          if (!isCurrentRequest()) {
            cleanup();
            return;
          }

          sceneSessionIdRef.current = session.session_id;
        } catch (cause) {
          if (!isCurrentRequest()) {
            return;
          }
          const message =
            cause instanceof Error
              ? cause.message
              : "Failed to start project scene session.";
          console.log(message);
          setProjectScene(null);
          setProjectSceneError(message);
          sceneSessionIdRef.current = "";
        }
      }

      void startNativeSceneSession();

      return () => {
        cancelled = true;
        if (sceneCompileRequestIdRef.current === requestId) {
          sceneCompilePumpRef.current = null;
          sceneSessionIdRef.current = "";
        }
        unlistenSceneSession?.();
      };
    }

    const pendingSlides: number[] = [];
    const runningSlides = new Set<number>();
    let manifestReady = false;
    const workerCount = preferredSceneWorkerCount();

    const pumpSlideQueue = (): void => {
      if (!manifestReady || !isCurrentRequest()) {
        return;
      }

      while (runningSlides.size < workerCount) {
        const nextIndex = takeNextSceneSlideIndex(
          pendingSlides,
          sceneCompilePriorityIndexRef.current,
        );
        if (nextIndex === null) {
          break;
        }

        runningSlides.add(nextIndex);
        void call<SceneSlide>("compile_project_scene_slide", {
          path: selectedProjectPath,
          index: nextIndex,
        })
          .then((compiledSlide) => {
            updateScene((current) =>
              current
                ? mergeCompiledSceneSlide(current, compiledSlide)
                : current,
            );
          })
          .catch((cause) => {
            const message =
              cause instanceof Error
                ? cause.message
                : "Failed to compile slide scene.";
            console.log(message);
            updateScene((current) =>
              current
                ? markSceneSlideError(current, nextIndex, message)
                : current,
            );
          })
          .finally(() => {
            runningSlides.delete(nextIndex);
            pumpSlideQueue();
          });
      }
    };

    sceneCompilePumpRef.current = pumpSlideQueue;

    void call<ProjectSceneManifest>("compile_project_scene_manifest", {
      path: selectedProjectPath,
    })
      .then((manifest) => {
        if (!isCurrentRequest()) {
          return;
        }

        setProjectScene(createPreviewProjectScene(manifest));
        pendingSlides.push(
          ...prioritizeSceneSlideIndices(
            manifest.slide_count,
            sceneCompilePriorityIndexRef.current,
          ),
        );
        manifestReady = true;
        pumpSlideQueue();
      })
      .catch((cause) => {
        if (!isCurrentRequest()) {
          return;
        }
        const message =
          cause instanceof Error
            ? cause.message
            : "Failed to compile project scene.";
        console.log(message);
        setProjectScene(null);
        setProjectSceneError(message);
        sceneCompilePumpRef.current = null;
      });

    return () => {
      cancelled = true;
      if (sceneCompileRequestIdRef.current === requestId) {
        sceneCompilePumpRef.current = null;
        sceneSessionIdRef.current = "";
      }
    };
  }, [selectedProject?.path, selectedProjectDetail?.updated_at]);

  useEffect(() => {
    if (!(settingsOpen && settingsTab === "library")) {
      return;
    }

    let cancelled = false;
    setComponentCatalogLoading(true);
    setComponentCatalogError("");

    call<ComponentCatalog>("get_component_catalog")
      .then((catalog) => {
        if (cancelled) {
          return;
        }
        setComponentCatalog(catalog);
        setSelectedComponentName((current) => {
          if (catalog.items.some((entry) => entry.name === current)) {
            return current;
          }
          return catalog.items[0]?.name || "";
        });
      })
      .catch((cause) => {
        if (cancelled) {
          return;
        }
        const message =
          cause instanceof Error
            ? cause.message
            : "Failed to load component library.";
        console.log(message);
        setComponentCatalog(null);
        setComponentCatalogError(message);
        setSelectedComponentName("");
      })
      .finally(() => {
        if (!cancelled) {
          setComponentCatalogLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [settingsOpen, settingsTab]);

  useEffect(() => {
    if (!(settingsOpen && settingsTab === "library" && selectedComponentName)) {
      setComponentTemplateLoading(false);
      setComponentTemplateError("");
      if (!selectedComponentName) {
        setSelectedComponentTemplate(null);
      }
      return;
    }

    let cancelled = false;
    setComponentTemplateLoading(true);
    setComponentTemplateError("");
    setSelectedComponentTemplate(null);

    call<DesignTemplate>("get_component_template", {
      name: selectedComponentName,
    })
      .then((template) => {
        if (!cancelled) {
          setSelectedComponentTemplate(template);
        }
      })
      .catch((cause) => {
        if (cancelled) {
          return;
        }
        const message =
          cause instanceof Error
            ? cause.message
            : "Failed to load component template.";
        console.log(message);
        setComponentTemplateError(message);
      })
      .finally(() => {
        if (!cancelled) {
          setComponentTemplateLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [selectedComponentName, settingsOpen, settingsTab]);

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
    return () => {
      cancelled = true;
    };
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
    return () => {
      style?.remove();
    };
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
    const slides = Array.from(
      container.querySelectorAll<HTMLElement>(".embedded-preview-deck .slide"),
    );
    if (slides.length === 0) {
      return -1;
    }

    const viewportCenterY =
      container.getBoundingClientRect().top + container.clientHeight / 2;
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

  function scrollListToSlide(
    index: number,
    behavior: ScrollBehavior = "smooth",
  ): void {
    const container = previewSurfaceRef.current;
    if (!container) {
      return;
    }
    const target = container.querySelector<HTMLElement>(
      `.embedded-preview-deck .slide[data-slide-index="${index}"]`,
    );
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
        setActiveSlideIndex((previous) =>
          previous === nextIndex ? previous : nextIndex,
        );
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
    const hasPreferred =
      preferredPath &&
      nextState.projects.some((item) => item.path === preferredPath);
    const nextSelection = hasPreferred ? preferredPath : fallbackPath;
    setSelectedPath(nextSelection);
    return nextSelection;
  }

  useEffect(() => {
    let cancelled = false;

    async function boot() {
      try {
        const initialDeckPath = initialPreviewRoute.deckPath;

        if (initialDeckPath) {
          const opened = await call<ProjectDetail>("open_project", {
            path: initialDeckPath,
          });
          await refreshState(opened.path);
        } else {
          await refreshState();
        }
        if (!cancelled) {
          console.log("Ready");
        }
      } catch (error) {
        if (!cancelled) {
          const message =
            error instanceof Error
              ? error.message
              : "Failed to load app state.";
          console.log(message);
        }
      }
    }

    void boot();

    return () => {
      cancelled = true;
    };
  }, [initialPreviewRoute.deckPath]);

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
        const cleanupOpenSettings = await listen(
          OPEN_SETTINGS_MENU_EVENT,
          () => {
            openSettingsPanel();
          },
        );
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
          error instanceof Error
            ? error.message
            : "Failed to register application menu listener.";
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
        Boolean(target?.isContentEditable) ||
        targetTag === "INPUT" ||
        targetTag === "TEXTAREA" ||
        targetTag === "SELECT";
      if (isTypingTarget) {
        return;
      }

      if (
        usingCommand &&
        (event.key === "=" || event.key === "+" || event.key === "Add")
      ) {
        event.preventDefault();
        setPreviewZoom((previous) =>
          clampPreviewZoom(previous + PREVIEW_ZOOM_STEP),
        );
        return;
      }

      if (
        usingCommand &&
        (event.key === "-" || event.key === "_" || event.key === "Subtract")
      ) {
        event.preventDefault();
        setPreviewZoom((previous) =>
          clampPreviewZoom(previous - PREVIEW_ZOOM_STEP),
        );
        return;
      }

      if (usingCommand && (event.key === "b" || event.key === "B")) {
        event.preventDefault();
        if (settingsOpen) {
          return;
        }
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

      if (
        event.key === "ArrowRight" ||
        event.key === "PageDown" ||
        event.key === " "
      ) {
        event.preventDefault();
        setActiveSlideIndex((previous) =>
          Math.min(previous + 1, maxSlideIndex),
        );
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
  }, [
    activeSlideIndex,
    expandedAsset,
    maxSlideIndex,
    presenterMode,
    selectedProject,
    settingsOpen,
  ]);

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

  function handlePreviewStagePointerMove(
    event: ReactPointerEvent<HTMLDivElement>,
  ): void {
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
      const folder = await pickFolder(
        "Select a project folder containing page.mdx",
      );
      if (!folder) {
        console.log("Open project cancelled.");
        return;
      }

      const opened = await call<ProjectDetail>("open_project", {
        path: folder,
      });
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
      const hasPreferred =
        preferredPath &&
        nextState.projects.some((project) => project.path === preferredPath);
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

  function openSettingsPanel(nextTab: SettingsTab = "theme"): void {
    setExpandedAsset(null);
    setSettingsTab(nextTab);
    setSettingsOpen(true);
    setSidebarOpen(true);
  }

  function updateToken<K extends keyof SlideTokens>(
    key: K,
    value: string,
  ): void {
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

  function handleSidebarResizeStart(
    event: ReactPointerEvent<HTMLDivElement>,
  ): void {
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
        disabled={settingsOpen}
        onToggle={() => {
          if (!settingsOpen) {
            setSidebarOpen((open) => !open);
          }
        }}
      />

      <AppSidebar
        busy={busy}
        sidebarOpen={sidebarOpen}
        settingsOpen={settingsOpen}
        settingsTab={settingsTab}
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
        onOpenSettings={() => openSettingsPanel("theme")}
        onSelectSettingsTab={setSettingsTab}
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
              scene={projectScene}
              error={projectSceneError}
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
        previewStatusLabel={previewStatusLabel}
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
        settingsTab={settingsTab}
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
        onUpdateToken={(key, value) =>
          updateToken(key as keyof SlideTokens, value)
        }
        onSaveCss={() => {
          void handleSaveCss();
        }}
        fontOptions={FONT_OPTIONS}
        monoFontOptions={MONO_FONT_OPTIONS}
        fontLabel={fontLabel}
        hexForInput={hexForInput}
        isHexColor={isHexColor}
        componentCatalog={componentCatalog?.items || []}
        componentCatalogLoading={componentCatalogLoading}
        componentCatalogError={componentCatalogError}
        selectedComponentName={selectedComponentName}
        onSelectComponent={setSelectedComponentName}
        selectedComponentTemplate={selectedComponentTemplate}
        componentTemplateLoading={componentTemplateLoading}
        componentTemplateError={componentTemplateError}
      />
    </main>
  );
}
