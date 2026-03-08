"use client";

export type ReviewNamedCount = {
  name: string;
  count: number;
};

export type ReviewOutlineEntry = {
  index: number;
  title: string;
};

export type ReviewSlideAnalysis = {
  index: number;
  title: string;
  archetype: string;
  words: number;
  bullets: number;
  max_paragraph_words: number;
  components: ReviewNamedCount[];
  warnings: string[];
};

export type ProjectAnalysis = {
  path: string;
  slide_count: number;
  has_project_css: boolean;
  outline: ReviewOutlineEntry[];
  components: ReviewNamedCount[];
  archetypes: ReviewNamedCount[];
  warnings: string[];
  slides: ReviewSlideAnalysis[];
};

type DeckReviewPanelProps = {
  analysis: ProjectAnalysis | null;
  activeSlideIndex: number;
};

const STRUCTURED_COMPONENTS = new Set([
  "Stack",
  "Row",
  "Grid",
  "Canvas",
  "Area",
  "Card",
  "Panel",
  "Callout",
  "Metric",
  "Caption",
  "Kicker",
  "Takeaway",
  "PillRow",
  "Pill",
  "Quote",
  "Rule",
]);

function formatCountLabel(item: ReviewNamedCount): string {
  return `${item.name} x${item.count}`;
}

export function DeckReviewPanel({ analysis, activeSlideIndex }: DeckReviewPanelProps) {
  if (!analysis) {
    return null;
  }

  const activeSlide =
    analysis.slides.find((slide) => slide.index === activeSlideIndex) || analysis.slides[0] || null;
  const totalFindings =
    analysis.warnings.length +
    analysis.slides.reduce((sum, slide) => sum + slide.warnings.length, 0);
  const topArchetypes = analysis.archetypes.slice(0, 3);
  const structuredComponents = analysis.components.filter((item) =>
    STRUCTURED_COMPONENTS.has(item.name),
  );

  return (
    <aside className="deck-review-panel" aria-label="Deck review summary">
      <div className="deck-review-header">
        <div>
          <p className="deck-review-kicker">Review</p>
          <h2>Current deck</h2>
        </div>
        <span className={`deck-review-badge ${totalFindings > 0 ? "warn" : "ok"}`}>
          {totalFindings > 0 ? `${totalFindings} findings` : "clean"}
        </span>
      </div>

      <div className="deck-review-stats">
        <div className="deck-review-stat">
          <span>Slides</span>
          <strong>{analysis.slide_count}</strong>
        </div>
        <div className="deck-review-stat">
          <span>Theme</span>
          <strong>{analysis.has_project_css ? "local" : "default"}</strong>
        </div>
        <div className="deck-review-stat">
          <span>Primitives</span>
          <strong>{structuredComponents.reduce((sum, item) => sum + item.count, 0)}</strong>
        </div>
      </div>

      {topArchetypes.length > 0 ? (
        <div className="deck-review-group">
          <div className="deck-review-label">Archetypes</div>
          <div className="deck-review-pill-row">
            {topArchetypes.map((item) => (
              <span key={`arch-${item.name}`} className="deck-review-pill">
                {formatCountLabel(item)}
              </span>
            ))}
          </div>
        </div>
      ) : null}

      {analysis.warnings.length > 0 ? (
        <div className="deck-review-group">
          <div className="deck-review-label">Deck findings</div>
          <ul className="deck-review-list">
            {analysis.warnings.slice(0, 3).map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </div>
      ) : null}

      {activeSlide ? (
        <div className="deck-review-group deck-review-slide">
          <div className="deck-review-slide-head">
            <div className="deck-review-label">Current slide</div>
            <span className="deck-review-slide-index">{activeSlide.index + 1}</span>
          </div>
          <div className="deck-review-slide-title">{activeSlide.title}</div>

          <div className="deck-review-mini-stats">
            <span>{activeSlide.archetype}</span>
            <span>{activeSlide.words} words</span>
            <span>{activeSlide.bullets} bullets</span>
          </div>

          {activeSlide.components.length > 0 ? (
            <div className="deck-review-pill-row">
              {activeSlide.components.slice(0, 4).map((item) => (
                <span key={`cmp-${activeSlide.index}-${item.name}`} className="deck-review-pill muted">
                  {formatCountLabel(item)}
                </span>
              ))}
            </div>
          ) : null}

          {activeSlide.warnings.length > 0 ? (
            <ul className="deck-review-list compact">
              {activeSlide.warnings.slice(0, 3).map((warning) => (
                <li key={`${activeSlide.index}-${warning}`}>{warning}</li>
              ))}
            </ul>
          ) : (
            <div className="deck-review-note">No current-slide findings.</div>
          )}
        </div>
      ) : null}
    </aside>
  );
}
