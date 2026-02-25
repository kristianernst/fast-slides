"use client";

export type SlideTocEntry = {
  index: number;
  title: string;
};

type SlideTocRailProps = {
  entries: SlideTocEntry[];
  activeIndex: number;
  onSelect: (index: number) => void;
};

export function SlideTocRail({ entries, activeIndex, onSelect }: SlideTocRailProps) {
  if (entries.length <= 1) {
    return null;
  }

  return (
    <nav className="slides-toc-rail" aria-label="Slide table of contents">
      <ol className="slides-toc-list">
        {entries.map((entry) => {
          const isActive = entry.index === activeIndex;
          return (
            <li key={`toc-${entry.index}`} className={`slides-toc-tick ${isActive ? "active" : ""}`}>
              <button
                type="button"
                className="slides-toc-hit"
                onClick={() => onSelect(entry.index)}
                aria-label={`Go to ${entry.title}`}
                title={entry.title}
              >
                <span className="slides-toc-line" />
              </button>
              <span className="slides-toc-label">{entry.title}</span>
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
