"use client";

import Image from "next/image";
import type {
  CSSProperties,
  PointerEvent as ReactPointerEvent,
  ReactNode,
  RefObject,
} from "react";
import { PreviewModeDock } from "./PreviewModeDock";
import { SlideTocRail, type SlideTocEntry } from "./SlideTocRail";

type PreviewWorkspaceProps = {
  settingsOpen: boolean;
  hasSelectedProject: boolean;
  presenterMode: boolean;
  previewSurfaceStyle: CSSProperties;
  previewSurfaceRef: RefObject<HTMLDivElement | null>;
  deckPreview: ReactNode;
  slideTocEntries: SlideTocEntry[];
  activeSlideIndex: number;
  onTocSelect: (index: number) => void;
  previewDockVisible: boolean;
  onPreviewDockPointerEnter: () => void;
  onPreviewDockPointerLeave: () => void;
  onTogglePresenterMode: () => void;
  onPreviewStagePointerMove: (event: ReactPointerEvent<HTMLDivElement>) => void;
  onPreviewStagePointerLeave: () => void;
};

export function PreviewWorkspace({
  settingsOpen,
  hasSelectedProject,
  presenterMode,
  previewSurfaceStyle,
  previewSurfaceRef,
  deckPreview,
  slideTocEntries,
  activeSlideIndex,
  onTocSelect,
  previewDockVisible,
  onPreviewDockPointerEnter,
  onPreviewDockPointerLeave,
  onTogglePresenterMode,
  onPreviewStagePointerMove,
  onPreviewStagePointerLeave,
}: PreviewWorkspaceProps) {
  return (
    <section className="workspace" aria-label={settingsOpen ? "Settings area" : "Preview area"}>
      {hasSelectedProject ? (
        <div
          className={`preview-stage ${presenterMode ? "presenter-mode single-slide-mode" : "list-slide-mode"}`}
          onPointerMove={onPreviewStagePointerMove}
          onPointerLeave={onPreviewStagePointerLeave}
        >
          <div className="preview-render-surface" style={previewSurfaceStyle}>
            <div ref={previewSurfaceRef} className="embedded-preview-surface">
              {deckPreview}
            </div>
          </div>

          <SlideTocRail
            entries={slideTocEntries}
            activeIndex={activeSlideIndex}
            onSelect={onTocSelect}
          />

          <PreviewModeDock
            presenterMode={presenterMode}
            visible={previewDockVisible}
            onPointerEnter={onPreviewDockPointerEnter}
            onPointerLeave={onPreviewDockPointerLeave}
            onTogglePresenterMode={onTogglePresenterMode}
          />
        </div>
      ) : (
        <div className="empty-center">
          <div className="empty-state">
            <Image
              src="/logo-clean.png"
              alt="FastSlides logo"
              width={140}
              height={130}
              className="empty-logo"
              priority
            />
            <p>No project selected</p>
          </div>
        </div>
      )}
    </section>
  );
}
