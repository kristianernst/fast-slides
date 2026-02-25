"use client";

import { Maximize, Minimize } from "@solar-icons/react";

type PreviewModeDockProps = {
  presenterMode: boolean;
  visible: boolean;
  onPointerEnter: () => void;
  onPointerLeave: () => void;
  onTogglePresenterMode: () => void;
};

export function PreviewModeDock({
  presenterMode,
  visible,
  onPointerEnter,
  onPointerLeave,
  onTogglePresenterMode,
}: PreviewModeDockProps) {
  return (
    <div
      className={`preview-mode-dock ${visible ? "is-visible" : ""}`}
      onPointerEnter={onPointerEnter}
      onPointerLeave={onPointerLeave}
    >
      <button
        type="button"
        className={`preview-dock-toggle ${presenterMode ? "active" : ""}`}
        onClick={onTogglePresenterMode}
        aria-pressed={presenterMode}
        aria-label={
          presenterMode ? "Switch to slide list mode" : "Switch to focused single-slide mode"
        }
        title={presenterMode ? "Minimize to slide list" : "Maximize to single slide"}
      >
        {presenterMode ? <Minimize size={16} weight="Linear" /> : <Maximize size={16} weight="Linear" />}
      </button>
    </div>
  );
}
