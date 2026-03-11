"use client";

import { useMemo, useState, type ReactNode } from "react";
import { Moon, Sun } from "@solar-icons/react";

type SlideTokenValues = {
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

type SlideTokenKey = keyof SlideTokenValues;

type SettingsTab = "theme" | "library";

type ComponentCatalogEntry = {
  name: string;
  family: string;
  kind: string;
  scope: string;
  summary: string;
  tags: string[];
};

type DesignTemplate = {
  kind: string;
  name: string;
  mdx: string;
  notes: string[];
};

type SettingsOverlayProps = {
  open: boolean;
  busy: boolean;
  settingsTab: SettingsTab;
  theme: "dark" | "light";
  onThemeChange: (theme: "dark" | "light") => void;
  mermaidThemeName: string;
  mermaidThemeOptions: readonly string[];
  mermaidThemeLabels: Record<string, string>;
  onMermaidThemeChange: (value: string) => void;
  syntaxThemeName: string;
  syntaxThemeOptionsByMode: {
    dark: readonly string[];
    light: readonly string[];
  };
  syntaxThemeLabels: Record<string, string>;
  onSyntaxThemeChange: (value: string) => void;
  selectedProject: boolean;
  slideTokens: SlideTokenValues;
  onUpdateToken: (key: SlideTokenKey, value: string) => void;
  onSaveCss: () => void;
  fontOptions: readonly string[];
  monoFontOptions: readonly string[];
  fontLabel: (value: string) => string;
  hexForInput: (value: string) => string;
  isHexColor: (value: string) => boolean;
  componentCatalog: readonly ComponentCatalogEntry[];
  componentCatalogLoading: boolean;
  componentCatalogError: string;
  selectedComponentName: string;
  onSelectComponent: (name: string) => void;
  selectedComponentTemplate: DesignTemplate | null;
  componentTemplateLoading: boolean;
  componentTemplateError: string;
};

const COLOR_FIELDS: Array<{ key: SlideTokenKey; label: string }> = [
  { key: "slideBg", label: "Background" },
  { key: "slideFg", label: "Text" },
  { key: "slideH1Color", label: "H1" },
  { key: "slideH2Color", label: "H2" },
  { key: "slideBodyColor", label: "Body" },
  { key: "slideMetaColor", label: "Meta text" },
  { key: "slideAccent", label: "Accent" },
  { key: "slideBorder", label: "Border" },
  { key: "slideLinkColor", label: "Link" },
  { key: "slideCodeBg", label: "Code background" },
];

const PALETTE_KEYS: SlideTokenKey[] = [
  "slidePalette1",
  "slidePalette2",
  "slidePalette3",
  "slidePalette4",
  "slidePalette5",
];

const LAYOUT_FIELDS: Array<{ key: SlideTokenKey; label: string }> = [
  { key: "slideRadius", label: "Radius" },
  { key: "slidePadding", label: "Padding" },
  { key: "slideLayoutGap", label: "Content gap" },
];

const COMPONENT_FIELDS: Array<{ key: SlideTokenKey; label: string }> = [
  { key: "slideCardBg", label: "Card background" },
  { key: "slideCardBorder", label: "Card border" },
  { key: "slideCardRadius", label: "Card radius" },
  { key: "slideCardPadding", label: "Card padding" },
];

function formatFamilyLabel(family: string): string {
  if (!family) return "Other";
  return family
    .split(/[-_]/)
    .filter(Boolean)
    .map((segment) => segment[0].toUpperCase() + segment.slice(1))
    .join(" ");
}

function extractTagContent(
  source: string | null | undefined,
  tag: string,
): string {
  if (!source) return "";
  const match = source.match(
    new RegExp(`<${tag}\\b[^>]*>([\\s\\S]*?)<\\/${tag}>`, "i"),
  );
  return match?.[1]?.replace(/\s+/g, " ").trim() ?? "";
}

function extractAttribute(
  source: string | null | undefined,
  tag: string,
  attribute: string,
): string {
  if (!source) return "";
  const match = source.match(
    new RegExp(`<${tag}\\b[^>]*\\b${attribute}="([^"]+)"`, "i"),
  );
  return match?.[1]?.trim() ?? "";
}

function extractAllAttributes(
  source: string | null | undefined,
  tag: string,
  attributes: string[],
): Array<Record<string, string>> {
  if (!source) return [];
  const matches = source.match(new RegExp(`<${tag}\\b[^>]*\\/?>`, "gi")) ?? [];
  return matches.map((block) => {
    const result: Record<string, string> = {};
    for (const attribute of attributes) {
      const value = block.match(new RegExp(`\\b${attribute}="([^"]+)"`, "i"));
      result[attribute] = value?.[1]?.trim() ?? "";
    }
    return result;
  });
}

function parseChartData(raw: string): Array<{ label: string; value: string }> {
  return raw
    .split(";")
    .map((entry) => entry.trim())
    .filter(Boolean)
    .map((entry) => {
      const [label, value] = entry.split(":");
      return {
        label: (label ?? "").trim(),
        value: (value ?? "").trim(),
      };
    })
    .filter((entry) => entry.label && entry.value);
}

function numericValue(value: string): number {
  const match = value.match(/-?\d+(?:\.\d+)?/);
  return Number(match?.[0] ?? "0");
}

function normalizeSeries(
  data: Array<{ label: string; value: string }>,
  fallback: Array<{ label: string; value: string }>,
) {
  const series = (data.length ? data : fallback).slice(0, 4).map((entry) => ({
    ...entry,
    numeric: numericValue(entry.value),
  }));
  const maxValue = Math.max(...series.map((entry) => entry.numeric), 1);
  return series.map((entry, index) => ({
    ...entry,
    index,
    ratio: Math.max(0.14, entry.numeric / maxValue),
  }));
}

function trendGeometry(series: Array<{ ratio: number }>) {
  const span = Math.max(series.length - 1, 1);
  const points = series.map((entry, index) => {
    const x = series.length === 1 ? 50 : 8 + index * (84 / span);
    const y = 52 - entry.ratio * 38;
    return {
      x,
      y,
    };
  });
  const linePath = points
    .map(
      (point, index) =>
        `${index === 0 ? "M" : "L"} ${point.x.toFixed(1)} ${point.y.toFixed(1)}`,
    )
    .join(" ");
  const firstPoint = points[0] ?? { x: 8, y: 52 };
  const lastPoint = points.at(-1) ?? { x: 92, y: 18 };
  return {
    points,
    linePath,
    areaPath: `${linePath} L ${lastPoint.x.toFixed(1)} 58 L ${firstPoint.x.toFixed(1)} 58 Z`,
  };
}

function FigureBarPreview({
  title,
  data,
  suffix,
}: {
  title?: string;
  data: Array<{ label: string; value: string }>;
  suffix: string;
}) {
  const safeData = normalizeSeries(data, [
    { label: "Option A", value: "72" },
    { label: "Option B", value: "54" },
    { label: "Option C", value: "39" },
  ]);
  return (
    <section className="library-figure-shell">
      <div className="library-plot-card">
        <div className="library-plot-grid" aria-hidden="true">
          <span />
          <span />
          <span />
        </div>
        <div className="library-plot-bars-modern">
          {safeData.map(({ label, value, ratio }) => (
            <div key={label} className="library-plot-row-modern">
              <span className="library-plot-label-modern">{label}</span>
              <div className="library-plot-track-modern">
                <span
                  className="library-plot-fill-modern"
                  style={{ width: `${Math.round(ratio * 100)}%` }}
                />
              </div>
              <strong className="library-plot-value-modern">{`${value}${suffix}`}</strong>
            </div>
          ))}
        </div>
      </div>
      {title ? <div className="mdx-caption">{title}</div> : null}
    </section>
  );
}

function FigureTrendPreview({
  title,
  data,
  suffix,
}: {
  title?: string;
  data: Array<{ label: string; value: string }>;
  suffix: string;
}) {
  const safeData = normalizeSeries(data, [
    { label: "Q1", value: "18" },
    { label: "Q2", value: "26" },
    { label: "Q3", value: "41" },
    { label: "Q4", value: "63" },
  ]);
  const { points, linePath, areaPath } = trendGeometry(safeData);
  return (
    <section className="library-figure-shell">
      <div className="library-plot-card library-plot-card--trend">
        <div className="library-plot-grid" aria-hidden="true">
          <span />
          <span />
          <span />
        </div>
        <svg
          className="library-trend-svg"
          viewBox="0 0 100 64"
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          <path d={areaPath} className="library-trend-area" />
          <path d={linePath} className="library-trend-line" />
          {points.map((point, index) => (
            <circle
              key={`${point.x}-${point.y}`}
              cx={point.x}
              cy={point.y}
              r={index === points.length - 1 ? 2.6 : 2.1}
              className={`library-trend-point ${index === points.length - 1 ? "is-last" : ""}`.trim()}
            />
          ))}
        </svg>
        <div className="library-trend-labels">
          {safeData.map(({ label, value }) => (
            <div key={label} className="library-trend-label">
              <span>{label}</span>
              <strong>{`${value}${suffix}`}</strong>
            </div>
          ))}
        </div>
      </div>
      {title ? <div className="mdx-caption">{title}</div> : null}
    </section>
  );
}

function FigureExhibitPanel({ title }: { title?: string }) {
  return (
    <section className="library-figure-shell">
      <div className="library-exhibit-card">
        <div className="library-exhibit-summary">
          <strong>2.4x</strong>
          <span>More agent volume in owned workflows than open discovery</span>
        </div>
        <div className="library-exhibit-header">
          <span>Workflow</span>
          <span>Owner</span>
          <span>Signal</span>
        </div>
        {[
          ["Discovery", "PM", "Growing usage"],
          ["Execution", "Ops", "High automation fit"],
          ["Review", "Lead", "Needs human override"],
        ].map(([left, middle, right]) => (
          <div key={left} className="library-exhibit-row">
            <span>{left}</span>
            <span>{middle}</span>
            <span>{right}</span>
          </div>
        ))}
      </div>
      {title ? <div className="mdx-caption">{title}</div> : null}
    </section>
  );
}

function InsightRail({
  title,
  body,
  tone,
}: {
  title?: string;
  body: string;
  tone: string;
}) {
  return (
    <aside
      className={`mdx-callout mdx-callout--${tone || "accent"} library-insight-rail`}
      data-variant="rail"
    >
      {title ? <div className="mdx-callout-title">{title}</div> : null}
      <div className="mdx-callout-body">{body}</div>
    </aside>
  );
}

function LibraryPreview({
  theme,
  entry,
  template,
}: {
  theme: "dark" | "light";
  entry: ComponentCatalogEntry;
  template: DesignTemplate | null;
}) {
  const name = entry.name.toLowerCase();
  const source = template?.mdx ?? "";
  const kicker = extractTagContent(source, "Kicker");
  const takeaway = extractTagContent(source, "Takeaway");
  const caption = extractTagContent(source, "Caption");
  const calloutTitle = extractAttribute(source, "Callout", "title");
  const calloutTone = extractAttribute(source, "Callout", "tone") || "accent";
  const calloutBody = extractTagContent(source, "Callout");
  const quoteAttribution = extractAttribute(source, "Quote", "attribution");
  const quoteBody = extractTagContent(source, "Quote");
  const panelTitle = extractAttribute(source, "Panel", "title");
  const panelTone = extractAttribute(source, "Panel", "tone") || "default";
  const panelBody = extractTagContent(source, "Panel");
  const chartTitle = extractAttribute(source, "Chart", "title");
  const chartData = parseChartData(extractAttribute(source, "Chart", "data"));
  const chartSuffix = extractAttribute(source, "Chart", "suffix");
  const arrowLabel = extractAttribute(source, "Arrow", "label");
  const arrowDirection =
    extractAttribute(source, "Arrow", "direction") || "right";
  const arrowTone = extractAttribute(source, "Arrow", "tone") || "accent";
  const metrics = extractAllAttributes(source, "Metric", [
    "label",
    "value",
    "hint",
  ]);
  const panels = extractAllAttributes(source, "Panel", ["title", "tone"]);

  function renderPreview(children: ReactNode, stageClassName?: string) {
    return (
      <div className="library-preview-frame" data-theme={theme}>
        <div className="embedded-preview-deck embedded-preview-single">
          <div className="deck">
            <section
              className="slide slide-ready"
              data-active="true"
              data-slide-index={0}
              data-slide-status="ready"
              aria-label={`${entry.name} preview`}
            >
              <div
                className={["library-preview-stage", stageClassName]
                  .filter(Boolean)
                  .join(" ")}
              >
                {children}
              </div>
            </section>
          </div>
        </div>
      </div>
    );
  }

  if (name === "kicker") {
    return renderPreview(
      <div className="mdx-kicker">{kicker || "Section label"}</div>,
    );
  }

  if (name === "takeaway") {
    return renderPreview(
      <h2 className="mdx-takeaway" data-scale="compact">
        {takeaway || "Replace this with the single conclusion for the slide."}
      </h2>,
    );
  }

  if (name === "caption") {
    return renderPreview(
      <div className="mdx-caption">{caption || "Source or operator note"}</div>,
    );
  }

  if (name === "callout") {
    return renderPreview(
      <aside
        className={`mdx-callout mdx-callout--${calloutTone}`}
        data-variant="rail"
      >
        {calloutTitle ? (
          <div className="mdx-callout-title">{calloutTitle}</div>
        ) : null}
        <div className="mdx-callout-body">
          {calloutBody || "Explain why the evidence matters."}
        </div>
      </aside>,
      "library-preview-stage--rail",
    );
  }

  if (name === "quote") {
    return renderPreview(
      <blockquote className="mdx-quote">
        <div className="mdx-quote-body">
          {quoteBody ||
            "Replace with one sharp proof point in the speaker's own words."}
        </div>
        {quoteAttribution ? (
          <footer className="mdx-quote-attribution">{quoteAttribution}</footer>
        ) : null}
      </blockquote>,
    );
  }

  if (name === "panel") {
    return renderPreview(
      <section
        className={`mdx-panel mdx-panel--${panelTone}`}
        data-variant="compact"
      >
        <div className="mdx-panel-head">
          <h3 className="mdx-panel-title">{panelTitle || "Evidence"}</h3>
        </div>
        <div className="mdx-panel-body">
          <p>{panelBody || "Replace with one structured block of evidence."}</p>
        </div>
      </section>,
    );
  }

  if (name === "metric") {
    const metric = metrics[0] ?? {};
    return renderPreview(
      <article className="mdx-metric" data-variant="compact">
        <p className="mdx-caption mdx-metric-label">
          {metric.label || "Metric"}
        </p>
        <p className="mdx-metric-value">{metric.value || "42%"}</p>
        <p className="mdx-caption mdx-metric-hint">
          {metric.hint || "Short note"}
        </p>
      </article>,
    );
  }

  if (name === "chart") {
    return renderPreview(
      <FigureBarPreview
        title={chartTitle}
        data={chartData}
        suffix={chartSuffix}
      />,
    );
  }

  if (name === "arrow") {
    return renderPreview(
      <div className="library-preview-arrow-row">
        <div
          className="mdx-arrow"
          data-direction={arrowDirection}
          data-tone={arrowTone}
        >
          <span className="mdx-arrow-label">
            {arrowLabel || "Flow of work"}
          </span>
          <span className="mdx-arrow-line" />
          <span className="mdx-arrow-head" />
        </div>
      </div>,
    );
  }

  if (name === "rule") {
    return renderPreview(<div className="mdx-rule" />);
  }

  if (name === "takeawayrail") {
    return renderPreview(
      <>
        <div className="mdx-kicker">{kicker || "Section"}</div>
        <h2 className="mdx-takeaway" data-scale="compact">
          {takeaway || "Replace this with the main conclusion for the slide."}
        </h2>
        <div className="library-preview-grid library-preview-grid--recipe">
          <div />
          <aside
            className={`mdx-callout mdx-callout--${calloutTone}`}
            data-variant="rail"
          >
            {calloutTitle ? (
              <div className="mdx-callout-title">{calloutTitle}</div>
            ) : null}
            <div className="mdx-callout-body">
              {calloutBody ||
                "Explain why the takeaway matters in one short paragraph."}
            </div>
          </aside>
        </div>
        {caption ? <div className="mdx-caption">{caption}</div> : null}
      </>,
    );
  }

  if (name === "takeaway_plus_rail") {
    return renderPreview(
      <>
        {kicker ? <div className="mdx-kicker">{kicker}</div> : null}
        <h2 className="mdx-takeaway" data-scale="compact">
          {takeaway || "Replace this with the main conclusion for the slide."}
        </h2>
        <div className="library-preview-grid library-preview-grid--recipe">
          <div />
          <aside
            className={`mdx-callout mdx-callout--${calloutTone}`}
            data-variant="rail"
          >
            {calloutTitle ? (
              <div className="mdx-callout-title">{calloutTitle}</div>
            ) : null}
            <div className="mdx-callout-body">
              {calloutBody ||
                "Explain why the takeaway matters in one short paragraph."}
            </div>
          </aside>
        </div>
        {caption ? <div className="mdx-caption">{caption}</div> : null}
      </>,
    );
  }

  if (name === "metricstrip" || name === "kpipair") {
    const previewMetrics = metrics.length
      ? metrics
      : [
          { label: "Metric A", value: "12%", hint: "Short supporting note" },
          { label: "Metric B", value: "3.4x", hint: "Short supporting note" },
          { label: "Metric C", value: "24d", hint: "Short supporting note" },
        ];
    return renderPreview(
      <div
        className={`library-preview-grid ${name === "kpipair" ? "library-preview-grid--recipe" : "library-preview-grid--three"}`}
      >
        <div
          className={
            name === "kpipair"
              ? "library-preview-grid library-preview-grid--two library-preview-stack"
              : "library-preview-grid library-preview-grid--three"
          }
        >
          {previewMetrics.slice(0, name === "kpipair" ? 2 : 3).map((metric) => (
            <article
              key={`${metric.label}-${metric.value}`}
              className="mdx-metric"
              data-variant="compact"
            >
              <p className="mdx-caption mdx-metric-label">{metric.label}</p>
              <p className="mdx-metric-value">{metric.value}</p>
              <p className="mdx-caption mdx-metric-hint">{metric.hint}</p>
            </article>
          ))}
        </div>
        {name === "kpipair" ? (
          <aside
            className={`mdx-callout mdx-callout--${calloutTone || "default"}`}
            data-variant="rail"
          >
            {calloutTitle ? (
              <div className="mdx-callout-title">{calloutTitle}</div>
            ) : null}
            <div className="mdx-callout-body">
              {calloutBody || "Add the interpretation implied by the two KPIs."}
            </div>
          </aside>
        ) : null}
      </div>,
    );
  }

  if (
    name === "exhibitcommentary" ||
    name === "exhibit_left_commentary_right"
  ) {
    return renderPreview(
      <>
        {kicker ? <div className="mdx-kicker">{kicker}</div> : null}
        {takeaway ? (
          <h2 className="mdx-takeaway" data-scale="compact">
            {takeaway}
          </h2>
        ) : null}
        <div className="library-preview-grid library-preview-grid--recipe">
          <FigureExhibitPanel title={chartTitle || panelTitle} />
          <InsightRail
            title={calloutTitle}
            body={
              calloutBody ||
              "Explain the one thing the audience should take away from the exhibit."
            }
            tone={calloutTone || "default"}
          />
        </div>
        {caption ? <div className="mdx-caption">{caption}</div> : null}
      </>,
    );
  }

  if (name === "scorecard_with_note") {
    const previewMetrics = metrics.length
      ? metrics
      : [
          { label: "Metric A", value: "12%", hint: "Short note" },
          { label: "Metric B", value: "3.4x", hint: "Short note" },
          { label: "Metric C", value: "24d", hint: "Short note" },
        ];
    return renderPreview(
      <>
        {kicker ? <div className="mdx-kicker">{kicker}</div> : null}
        {takeaway ? (
          <h2 className="mdx-takeaway" data-scale="compact">
            {takeaway}
          </h2>
        ) : null}
        <div className="library-preview-grid library-preview-grid--recipe">
          <div className="library-preview-grid library-preview-grid--three">
            {previewMetrics.slice(0, 3).map((metric) => (
              <article
                key={`${metric.label}-${metric.value}`}
                className="mdx-metric"
                data-variant="compact"
              >
                <p className="mdx-caption mdx-metric-label">{metric.label}</p>
                <p className="mdx-metric-value">{metric.value}</p>
                <p className="mdx-caption mdx-metric-hint">{metric.hint}</p>
              </article>
            ))}
          </div>
          <aside
            className={`mdx-callout mdx-callout--${calloutTone}`}
            data-variant="rail"
          >
            {calloutTitle ? (
              <div className="mdx-callout-title">{calloutTitle}</div>
            ) : null}
            <div className="mdx-callout-body">
              {calloutBody || "Explain the scorecard in one sentence."}
            </div>
          </aside>
        </div>
        {caption ? <div className="mdx-caption">{caption}</div> : null}
      </>,
    );
  }

  if (name === "threeuppanels" || name === "three_up_compare") {
    const previewPanels = panels.length
      ? panels
      : [
          { title: "Column one", tone: "accent" },
          { title: "Column two", tone: "default" },
          { title: "Column three", tone: "default" },
        ];
    return renderPreview(
      <>
        {kicker ? <div className="mdx-kicker">{kicker}</div> : null}
        {takeaway ? (
          <h2 className="mdx-takeaway" data-scale="compact">
            {takeaway}
          </h2>
        ) : null}
        <div className="library-preview-grid library-preview-grid--three">
          {previewPanels.slice(0, 3).map((panel, index) => (
            <section
              key={`${panel.title}-${index}`}
              className={`mdx-panel ${panel.tone && panel.tone !== "default" ? `mdx-panel--${panel.tone}` : ""}`.trim()}
              data-variant="compact"
            >
              <div className="mdx-panel-head">
                <h3 className="mdx-panel-title">
                  {panel.title || `Column ${index + 1}`}
                </h3>
              </div>
              <div className="mdx-panel-body">
                <p>
                  Replace with the{" "}
                  {index === 0 ? "first" : index === 1 ? "second" : "third"}{" "}
                  parallel point.
                </p>
              </div>
            </section>
          ))}
        </div>
      </>,
    );
  }

  if (name === "beforeafter" || name === "static_vs_dynamic_compare") {
    const previewPanels = panels.length
      ? panels
      : [
          { title: "Before", tone: "default" },
          { title: "After", tone: "accent" },
        ];
    return renderPreview(
      <>
        {kicker ? <div className="mdx-kicker">{kicker}</div> : null}
        {takeaway ? (
          <h2 className="mdx-takeaway" data-scale="compact">
            {takeaway}
          </h2>
        ) : null}
        <div className="library-preview-grid library-preview-grid--two">
          {previewPanels.slice(0, 2).map((panel, index) => (
            <section
              key={`${panel.title}-${index}`}
              className={`mdx-panel ${panel.tone && panel.tone !== "default" ? `mdx-panel--${panel.tone}` : ""}`.trim()}
              data-variant="compact"
            >
              <div className="mdx-panel-head">
                <h3 className="mdx-panel-title">
                  {panel.title || (index === 0 ? "Before" : "After")}
                </h3>
              </div>
              <div className="mdx-panel-body">
                <p>
                  {index === 0
                    ? "Replace with the current state."
                    : "Replace with the target state after the shift."}
                </p>
              </div>
            </section>
          ))}
        </div>
        {caption ? <div className="mdx-caption">{caption}</div> : null}
      </>,
    );
  }

  if (name === "operatingmodelrow" || name === "operating_model") {
    const previewPanels = panels.length
      ? panels
      : [
          { title: "Sense", tone: "accent" },
          { title: "Decide", tone: "default" },
          { title: "Act", tone: "default" },
        ];
    return renderPreview(
      <>
        {kicker ? <div className="mdx-kicker">{kicker}</div> : null}
        {takeaway ? (
          <h2 className="mdx-takeaway" data-scale="compact">
            {takeaway}
          </h2>
        ) : null}
        <div className="library-preview-grid library-preview-grid--three">
          {previewPanels.slice(0, 3).map((panel, index) => (
            <section
              key={`${panel.title}-${index}`}
              className={`mdx-panel ${panel.tone && panel.tone !== "default" ? `mdx-panel--${panel.tone}` : ""}`.trim()}
              data-variant="compact"
            >
              <div className="mdx-panel-head">
                <h3 className="mdx-panel-title">
                  {panel.title || `Stage ${index + 1}`}
                </h3>
              </div>
              <div className="mdx-panel-body">
                <p>
                  {index === 0
                    ? "Inputs and signals"
                    : index === 1
                      ? "Rules and routing"
                      : "Execution and review"}
                </p>
              </div>
            </section>
          ))}
        </div>
        {caption ? <div className="mdx-caption">{caption}</div> : null}
      </>,
    );
  }

  if (name === "quoteevidence" || name === "quote_with_evidence") {
    return renderPreview(
      <>
        {kicker ? <div className="mdx-kicker">{kicker}</div> : null}
        {takeaway ? (
          <h2 className="mdx-takeaway" data-scale="compact">
            {takeaway}
          </h2>
        ) : null}
        <div className="library-preview-grid library-preview-grid--recipe">
          <blockquote className="mdx-quote">
            <div className="mdx-quote-body">
              {quoteBody ||
                "Replace with one proof quote that deserves executive attention."}
            </div>
            {quoteAttribution ? (
              <footer className="mdx-quote-attribution">
                {quoteAttribution}
              </footer>
            ) : null}
          </blockquote>
          <section
            className={`mdx-panel ${panelTone !== "default" ? `mdx-panel--${panelTone}` : ""}`.trim()}
            data-variant="compact"
          >
            <div className="mdx-panel-head">
              <h3 className="mdx-panel-title">{panelTitle || "Evidence"}</h3>
            </div>
            <div className="mdx-panel-body">
              <p>
                {panelBody ||
                  "Replace with the structured evidence that supports the quote."}
              </p>
            </div>
          </section>
        </div>
        {caption ? <div className="mdx-caption">{caption}</div> : null}
      </>,
    );
  }

  if (name === "kpi_pair_with_exhibit") {
    const previewMetrics = metrics.length
      ? metrics
      : [
          { label: "KPI one", value: "42%", hint: "Short note" },
          { label: "KPI two", value: "18d", hint: "Short note" },
        ];
    return renderPreview(
      <>
        {kicker ? <div className="mdx-kicker">{kicker}</div> : null}
        {takeaway ? (
          <h2 className="mdx-takeaway" data-scale="compact">
            {takeaway}
          </h2>
        ) : null}
        <div className="library-preview-grid library-preview-grid--recipe">
          <FigureBarPreview
            title={chartTitle}
            data={
              chartData.length
                ? chartData
                : [
                    { label: "Segment A", value: "47" },
                    { label: "Segment B", value: "38" },
                    { label: "Segment C", value: "29" },
                  ]
            }
            suffix={chartSuffix}
          />
          <div className="library-preview-grid library-preview-grid--two library-preview-stack">
            {previewMetrics.slice(0, 2).map((metric) => (
              <article
                key={`${metric.label}-${metric.value}`}
                className="mdx-metric"
                data-variant="compact"
              >
                <p className="mdx-caption mdx-metric-label">{metric.label}</p>
                <p className="mdx-metric-value">{metric.value}</p>
                <p className="mdx-caption mdx-metric-hint">{metric.hint}</p>
              </article>
            ))}
          </div>
        </div>
      </>,
    );
  }

  if (name === "imagefigure") {
    return renderPreview(
      <section className="mdx-panel" data-variant="compact">
        <div className="mdx-panel-head">
          <h3 className="mdx-panel-title">Key visual</h3>
        </div>
        <div className="mdx-panel-body">
          <div className="library-preview-media-frame" />
        </div>
        <div className="mdx-panel-foot">Short caption or explanatory note.</div>
      </section>,
    );
  }

  if (name === "logostrip") {
    return renderPreview(
      <div className="mdx-pill-row">
        <span className="mdx-pill">OpenAI</span>
        <span className="mdx-pill">Anthropic</span>
        <span className="mdx-pill">Google</span>
        <span className="mdx-pill">Microsoft</span>
      </div>,
    );
  }

  if (name === "trendchartcommentary" || name === "barchartcommentary") {
    return renderPreview(
      <div className="library-preview-grid library-preview-grid--recipe">
        {name === "trendchartcommentary" ? (
          <FigureTrendPreview
            title={chartTitle}
            data={chartData}
            suffix={chartSuffix}
          />
        ) : (
          <FigureBarPreview
            title={chartTitle}
            data={chartData}
            suffix={chartSuffix}
          />
        )}
        <InsightRail
          title={calloutTitle}
          body={
            calloutBody ||
            "Explain the one thing the audience should take away from the exhibit."
          }
          tone={calloutTone || "default"}
        />
      </div>,
    );
  }

  if (name === "arrowbridge") {
    return renderPreview(
      <div className="library-preview-arrow-row">
        <div
          className="mdx-arrow"
          data-direction={arrowDirection || "right"}
          data-tone={arrowTone || "accent"}
        >
          <span className="mdx-arrow-label">
            {arrowLabel || "Connect the modules"}
          </span>
          <span className="mdx-arrow-line" />
          <span className="mdx-arrow-head" />
        </div>
      </div>,
    );
  }

  return renderPreview(
    <section className="mdx-panel" data-variant="compact">
      <div className="mdx-panel-head">
        <div className="mdx-kicker">{formatFamilyLabel(entry.family)}</div>
        <h3 className="mdx-panel-title">{entry.name}</h3>
        <p className="mdx-panel-subtitle">{entry.summary}</p>
      </div>
    </section>,
  );
}

export function SettingsOverlay({
  open,
  busy,
  settingsTab,
  theme,
  onThemeChange,
  mermaidThemeName,
  mermaidThemeOptions,
  mermaidThemeLabels,
  onMermaidThemeChange,
  syntaxThemeName,
  syntaxThemeOptionsByMode,
  syntaxThemeLabels,
  onSyntaxThemeChange,
  selectedProject,
  slideTokens,
  onUpdateToken,
  onSaveCss,
  fontOptions,
  monoFontOptions,
  fontLabel,
  hexForInput,
  isHexColor,
  componentCatalog,
  componentCatalogLoading,
  componentCatalogError,
  selectedComponentName,
  onSelectComponent,
  selectedComponentTemplate,
  componentTemplateLoading,
  componentTemplateError,
}: SettingsOverlayProps) {
  const [libraryQuery, setLibraryQuery] = useState("");
  const [expandedFamilies, setExpandedFamilies] = useState<
    Record<string, boolean>
  >({});
  const normalizedLibraryQuery = libraryQuery.trim().toLowerCase();
  const filteredCatalog = useMemo(() => {
    if (!normalizedLibraryQuery) {
      return componentCatalog;
    }
    return componentCatalog.filter((entry) => {
      const haystack = [
        entry.name,
        entry.family,
        entry.kind,
        entry.scope,
        entry.summary,
        ...entry.tags,
      ]
        .join(" ")
        .toLowerCase();
      return haystack.includes(normalizedLibraryQuery);
    });
  }, [componentCatalog, normalizedLibraryQuery]);
  const groupedCatalog = useMemo(() => {
    const groups = new Map<string, ComponentCatalogEntry[]>();
    for (const entry of filteredCatalog) {
      const key = entry.family || "other";
      const bucket = groups.get(key);
      if (bucket) {
        bucket.push(entry);
        continue;
      }
      groups.set(key, [entry]);
    }
    return Array.from(groups.entries());
  }, [filteredCatalog]);
  const selectedCatalogEntry =
    componentCatalog.find((entry) => entry.name === selectedComponentName) ||
    null;
  const panelTitle = settingsTab === "library" ? "Library" : "Theme";
  const panelHint =
    settingsTab === "library"
      ? "Browse reusable primitives, compositions, recipes, and saved snippets."
      : "Tune rendering defaults, deck tokens, and theme behavior.";

  function toggleLibraryFamily(family: string): void {
    setExpandedFamilies((current) => ({
      ...current,
      [family]: !current[family],
    }));
  }

  if (!open) {
    return null;
  }

  return (
    <div className="settings-overlay">
      <div className="settings-dialog">
        <header className="settings-header">
          <div className="settings-header-copy">
            <span className="settings-header-eyebrow">Settings</span>
            <h2>{panelTitle}</h2>
            <p className="settings-hint">{panelHint}</p>
          </div>
        </header>
        <div className="settings-body">
          {settingsTab === "theme" ? (
            <>
              <div className="settings-section">
                <span className="settings-label">Theme</span>
                <div className="theme-toggle">
                  <button
                    type="button"
                    className={theme === "dark" ? "active" : ""}
                    onClick={() => onThemeChange("dark")}
                  >
                    <Moon size={14} weight="Linear" /> Dark
                  </button>
                  <button
                    type="button"
                    className={theme === "light" ? "active" : ""}
                    onClick={() => onThemeChange("light")}
                  >
                    <Sun size={14} weight="Linear" /> Light
                  </button>
                </div>
              </div>

              <div className="settings-section">
                <span className="settings-label">Mermaid</span>
                <div className="token-grid">
                  <label className="token-row">
                    <span className="token-name">Theme</span>
                    <select
                      className="token-select"
                      value={mermaidThemeName}
                      onChange={(event) =>
                        onMermaidThemeChange(event.target.value)
                      }
                    >
                      {mermaidThemeOptions.map((option) => (
                        <option key={option} value={option}>
                          {mermaidThemeLabels[option] || option}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>
              </div>

              <div className="settings-section">
                <span className="settings-label">Code blocks</span>
                <div className="token-grid">
                  <label className="token-row">
                    <span className="token-name">Editor theme</span>
                    <select
                      className="token-select"
                      value={syntaxThemeName}
                      onChange={(event) =>
                        onSyntaxThemeChange(event.target.value)
                      }
                    >
                      <optgroup label="Dark">
                        {syntaxThemeOptionsByMode.dark.map((option) => (
                          <option key={option} value={option}>
                            {syntaxThemeLabels[option] || option}
                          </option>
                        ))}
                      </optgroup>
                      <optgroup label="Light">
                        {syntaxThemeOptionsByMode.light.map((option) => (
                          <option key={option} value={option}>
                            {syntaxThemeLabels[option] || option}
                          </option>
                        ))}
                      </optgroup>
                    </select>
                  </label>
                </div>
              </div>

              {selectedProject ? (
                <>
                  <div className="settings-section">
                    <span className="settings-label">Colors</span>
                    <div className="token-grid">
                      {COLOR_FIELDS.map(({ key, label }) => (
                        <label key={key} className="token-row">
                          <span className="token-name">{label}</span>
                          <span className="color-field">
                            <input
                              type="color"
                              value={hexForInput(slideTokens[key])}
                              onChange={(event) =>
                                onUpdateToken(key, event.target.value)
                              }
                            />
                            <input
                              type="text"
                              className="color-text"
                              value={slideTokens[key]}
                              onChange={(event) =>
                                onUpdateToken(key, event.target.value)
                              }
                            />
                          </span>
                        </label>
                      ))}
                    </div>
                  </div>

                  <div className="settings-section">
                    <span className="settings-label">Palette</span>
                    <div className="palette-row">
                      {PALETTE_KEYS.map((key) => (
                        <label key={key} className="palette-swatch">
                          <input
                            type="color"
                            value={hexForInput(slideTokens[key])}
                            onChange={(event) =>
                              onUpdateToken(key, event.target.value)
                            }
                          />
                          <span
                            className="palette-preview"
                            style={{
                              background: isHexColor(slideTokens[key])
                                ? slideTokens[key]
                                : "#888",
                            }}
                          />
                        </label>
                      ))}
                    </div>
                  </div>

                  <div className="settings-section">
                    <span className="settings-label">Typography</span>
                    <div className="token-grid">
                      <label className="token-row">
                        <span className="token-name">Font</span>
                        <select
                          className="token-select"
                          value={slideTokens.slideFontFamily}
                          onChange={(event) =>
                            onUpdateToken("slideFontFamily", event.target.value)
                          }
                        >
                          {fontOptions.map((option) => (
                            <option key={option} value={option}>
                              {fontLabel(option)}
                            </option>
                          ))}
                        </select>
                      </label>
                      <label className="token-row">
                        <span className="token-name">Heading font</span>
                        <select
                          className="token-select"
                          value={slideTokens.slideHeadingFont}
                          onChange={(event) =>
                            onUpdateToken(
                              "slideHeadingFont",
                              event.target.value,
                            )
                          }
                        >
                          <option value="var(--slide-font-family)">
                            Same as body
                          </option>
                          {fontOptions.map((option) => (
                            <option key={option} value={option}>
                              {fontLabel(option)}
                            </option>
                          ))}
                        </select>
                      </label>
                      <label className="token-row">
                        <span className="token-name">Code font</span>
                        <select
                          className="token-select"
                          value={slideTokens.slideCodeFont}
                          onChange={(event) =>
                            onUpdateToken("slideCodeFont", event.target.value)
                          }
                        >
                          {monoFontOptions.map((option) => (
                            <option key={option} value={option}>
                              {fontLabel(option)}
                            </option>
                          ))}
                        </select>
                      </label>
                      <label className="token-row">
                        <span className="token-name">Meta font</span>
                        <select
                          className="token-select"
                          value={slideTokens.slideMetaFont}
                          onChange={(event) =>
                            onUpdateToken("slideMetaFont", event.target.value)
                          }
                        >
                          <option value="var(--slide-code-font)">
                            Same as code
                          </option>
                          {monoFontOptions.map((option) => (
                            <option key={option} value={option}>
                              {fontLabel(option)}
                            </option>
                          ))}
                        </select>
                      </label>
                      <label className="token-row">
                        <span className="token-name">Meta size</span>
                        <input
                          type="text"
                          className="token-input"
                          value={slideTokens.slideMetaSize}
                          onChange={(event) =>
                            onUpdateToken("slideMetaSize", event.target.value)
                          }
                        />
                      </label>
                    </div>
                  </div>

                  <div className="settings-section">
                    <span className="settings-label">Layout</span>
                    <div className="token-grid">
                      {LAYOUT_FIELDS.map(({ key, label }) => (
                        <label key={key} className="token-row">
                          <span className="token-name">{label}</span>
                          <input
                            type="text"
                            className="token-input"
                            value={slideTokens[key]}
                            onChange={(event) =>
                              onUpdateToken(key, event.target.value)
                            }
                          />
                        </label>
                      ))}
                    </div>
                  </div>

                  <div className="settings-section">
                    <span className="settings-label">Components</span>
                    <div className="token-grid">
                      {COMPONENT_FIELDS.map(({ key, label }) => {
                        const isColor =
                          key.endsWith("Bg") || key.endsWith("Border");
                        return (
                          <label key={key} className="token-row">
                            <span className="token-name">{label}</span>
                            {isColor ? (
                              <span className="color-field">
                                <input
                                  type="color"
                                  value={hexForInput(slideTokens[key])}
                                  onChange={(event) =>
                                    onUpdateToken(key, event.target.value)
                                  }
                                />
                                <input
                                  type="text"
                                  className="color-text"
                                  value={slideTokens[key]}
                                  onChange={(event) =>
                                    onUpdateToken(key, event.target.value)
                                  }
                                />
                              </span>
                            ) : (
                              <input
                                type="text"
                                className="token-input"
                                value={slideTokens[key]}
                                onChange={(event) =>
                                  onUpdateToken(key, event.target.value)
                                }
                              />
                            )}
                          </label>
                        );
                      })}
                    </div>
                  </div>

                  <button
                    type="button"
                    className="btn btn-primary settings-save-btn"
                    onClick={onSaveCss}
                    disabled={busy}
                  >
                    Save to slides.css
                  </button>
                </>
              ) : (
                <div className="settings-section">
                  <span className="settings-label">Project theme</span>
                  <p className="settings-hint">
                    Slide tokens like fonts, heading styles, colors, palette,
                    and component surfaces are project-specific and live in{" "}
                    <code>slides.css</code>. Open or create a project to edit
                    them here.
                  </p>
                </div>
              )}
            </>
          ) : (
            <div className="settings-section">
              <div className="library-toolbar">
                <div>
                  <span className="settings-label">Component library</span>
                  <p className="settings-hint">
                    Query the design system without loading every component into
                    prompt context.
                  </p>
                </div>
                <input
                  type="search"
                  className="library-search-input"
                  placeholder="Search components"
                  value={libraryQuery}
                  onChange={(event) => setLibraryQuery(event.target.value)}
                />
              </div>

              <div className="library-browser">
                <section
                  className="library-list-panel"
                  aria-label="Component catalog"
                >
                  {componentCatalogLoading ? (
                    <p className="settings-hint">Loading component library…</p>
                  ) : componentCatalogError ? (
                    <p className="library-empty-state">
                      {componentCatalogError}
                    </p>
                  ) : groupedCatalog.length === 0 ? (
                    <p className="library-empty-state">
                      {normalizedLibraryQuery
                        ? "No library entries match that search."
                        : "No library entries available yet."}
                    </p>
                  ) : (
                    groupedCatalog.map(([family, entries]) => (
                      <div key={family} className="library-group">
                        <button
                          type="button"
                          className={`library-group-toggle ${normalizedLibraryQuery || expandedFamilies[family] ? "is-expanded" : ""}`}
                          onClick={() => toggleLibraryFamily(family)}
                          aria-expanded={
                            normalizedLibraryQuery
                              ? true
                              : Boolean(expandedFamilies[family])
                          }
                        >
                          <span className="library-group-head">
                            <span className="library-group-name">
                              {formatFamilyLabel(family)}
                            </span>
                            <span className="library-group-count">
                              {entries.length}
                            </span>
                          </span>
                          <span className="library-group-toggle-text">
                            {normalizedLibraryQuery || expandedFamilies[family]
                              ? "close"
                              : "open"}
                          </span>
                        </button>
                        {normalizedLibraryQuery || expandedFamilies[family] ? (
                          <div className="library-group-items">
                            {entries.map((entry) => (
                              <button
                                key={entry.name}
                                type="button"
                                className={`library-item ${entry.name === selectedComponentName ? "is-active" : ""}`}
                                onClick={() => onSelectComponent(entry.name)}
                              >
                                <span className="library-item-row">
                                  <span className="library-item-name">
                                    {entry.name}
                                  </span>
                                  <span
                                    className="library-item-kind"
                                    data-kind={entry.kind}
                                  >
                                    {entry.kind}
                                  </span>
                                </span>
                                <span className="library-item-summary">
                                  {entry.summary}
                                </span>
                              </button>
                            ))}
                          </div>
                        ) : null}
                      </div>
                    ))
                  )}
                </section>

                <section
                  className="library-detail-panel"
                  aria-label="Selected component details"
                >
                  {selectedCatalogEntry ? (
                    <>
                      <div className="library-detail-head">
                        <div>
                          <h3>{selectedCatalogEntry.name}</h3>
                          <p className="library-detail-summary">
                            {selectedCatalogEntry.summary}
                          </p>
                        </div>
                        <div className="library-chip-row">
                          <span className="library-chip">
                            {selectedCatalogEntry.family}
                          </span>
                          <span
                            className="library-chip"
                            data-chip-role="kind"
                            data-chip-value={selectedCatalogEntry.kind}
                          >
                            {selectedCatalogEntry.kind}
                          </span>
                          <span className="library-chip">
                            {selectedCatalogEntry.scope}
                          </span>
                        </div>
                      </div>

                      <div className="settings-section">
                        <span className="settings-label">Preview</span>
                        <LibraryPreview
                          theme={theme}
                          entry={selectedCatalogEntry}
                          template={selectedComponentTemplate}
                        />
                      </div>

                      {selectedCatalogEntry.tags.length > 0 ? (
                        <div className="library-chip-row">
                          {selectedCatalogEntry.tags.map((tag) => (
                            <span key={tag} className="library-chip">
                              {tag}
                            </span>
                          ))}
                        </div>
                      ) : null}

                      <div className="settings-section">
                        <span className="settings-label">Template</span>
                        {componentTemplateLoading ? (
                          <p className="settings-hint">
                            Loading canonical template…
                          </p>
                        ) : componentTemplateError ? (
                          <p className="library-empty-state">
                            {componentTemplateError}
                          </p>
                        ) : selectedComponentTemplate ? (
                          <pre className="library-code">
                            {selectedComponentTemplate.mdx}
                          </pre>
                        ) : (
                          <p className="settings-hint">
                            Select a library entry to inspect it.
                          </p>
                        )}
                      </div>

                      {selectedComponentTemplate?.notes.length ? (
                        <div className="settings-section">
                          <span className="settings-label">Usage notes</span>
                          <ul className="library-note-list">
                            {selectedComponentTemplate.notes.map((note) => (
                              <li key={note}>{note}</li>
                            ))}
                          </ul>
                        </div>
                      ) : null}
                    </>
                  ) : (
                    <p className="library-empty-state">
                      Select a component to inspect its canonical MDX and usage
                      notes.
                    </p>
                  )}
                </section>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
