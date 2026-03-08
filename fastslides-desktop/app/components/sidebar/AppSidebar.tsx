"use client";

import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { BoxMinimalistic } from "@solar-icons/react";
import { DeckReviewPanel, type ProjectAnalysis } from "../workspace/DeckReviewPanel";
import { ProjectList } from "./ProjectList";
import type { SidebarProject } from "./types";

type AppSidebarProps = {
  busy: boolean;
  sidebarOpen: boolean;
  settingsOpen: boolean;
  settingsTab: "theme" | "library";
  projectsCount: number;
  projects: SidebarProject[];
  pinnedPaths: string[];
  selectedPath: string;
  deckAnalysis: ProjectAnalysis | null;
  activeSlideIndex: number;
  reviewVisible: boolean;
  onBackToApp: () => void;
  onOpenProject: () => void;
  onSelectProject: (path: string) => void;
  onRemoveProject: (path: string) => void;
  onTogglePin: (path: string) => void;
  onToggleReview: () => void;
  onOpenSettings: () => void;
  onSelectSettingsTab: (tab: "theme" | "library") => void;
};

export function AppSidebar({
  busy,
  sidebarOpen,
  settingsOpen,
  settingsTab,
  projectsCount,
  projects,
  pinnedPaths,
  selectedPath,
  deckAnalysis,
  activeSlideIndex,
  reviewVisible,
  onBackToApp,
  onOpenProject,
  onSelectProject,
  onRemoveProject,
  onTogglePin,
  onToggleReview,
  onOpenSettings,
  onSelectSettingsTab,
}: AppSidebarProps) {
  const interactionsDisabled = busy || !sidebarOpen;

  return (
    <aside className="sidebar" aria-hidden={!sidebarOpen}>
      <div className="sidebar-drag-region" data-tauri-drag-region aria-hidden="true" />
      {settingsOpen ? (
        <div className="sidebar-settings-shell">
          <button
            type="button"
            className="sidebar-back-link"
            onClick={onBackToApp}
            disabled={interactionsDisabled}
          >
            <ArrowLeftIcon aria-hidden="true" />
            <span>back to app</span>
          </button>
          <div className="sidebar-settings-only">
            <div className="section-title-row">
              <h2>
                <BoxMinimalistic
                  className="section-title-icon"
                  size={14}
                  weight="Linear"
                  aria-hidden="true"
                />
                <span>Settings</span>
              </h2>
              <span className="count-pill">2</span>
            </div>
            <nav className="settings-nav" aria-label="Settings sections">
              <button
                type="button"
                className={`settings-nav-item ${settingsTab === "theme" ? "is-active" : ""}`}
                onClick={() => onSelectSettingsTab("theme")}
                disabled={interactionsDisabled}
              >
                <span className="settings-nav-label">Theme</span>
                <span className="settings-nav-meta">Tokens, code, Mermaid</span>
              </button>
              <button
                type="button"
                className={`settings-nav-item ${settingsTab === "library" ? "is-active" : ""}`}
                onClick={() => onSelectSettingsTab("library")}
                disabled={interactionsDisabled}
              >
                <span className="settings-nav-label">Library</span>
                <span className="settings-nav-meta">Patterns, primitives, saved snippets</span>
              </button>
            </nav>
          </div>
        </div>
      ) : (
        <>
          <header className="sidebar-head" data-tauri-drag-region>
            <button
              type="button"
              className="btn btn-primary btn-block"
              onClick={onOpenProject}
              disabled={interactionsDisabled}
            >
              Open Project
            </button>
          </header>

          <section className="project-section">
            <div className="section-title-row">
              <h2>
                <BoxMinimalistic
                  className="section-title-icon"
                  size={14}
                  weight="Linear"
                  aria-hidden="true"
                />
                <span>Projects</span>
              </h2>
              <span className="count-pill">{projectsCount}</span>
            </div>

            <ProjectList
              projects={projects}
              pinnedPaths={pinnedPaths}
              selectedPath={selectedPath}
              disabled={interactionsDisabled}
              onSelectProject={onSelectProject}
              onRemoveProject={onRemoveProject}
              onTogglePin={onTogglePin}
            />

            {reviewVisible && deckAnalysis ? (
              <DeckReviewPanel analysis={deckAnalysis} activeSlideIndex={activeSlideIndex} />
            ) : null}
          </section>

          <footer className="sidebar-footer">
            <button
              type="button"
              className={`sidebar-footer-link ${reviewVisible ? "is-active" : ""}`}
              onClick={onToggleReview}
              aria-pressed={reviewVisible}
              disabled={interactionsDisabled || !deckAnalysis}
            >
              {reviewVisible ? "Hide Review" : "Show Review"}
            </button>
            <button
              type="button"
              className="sidebar-footer-link"
              onClick={onOpenSettings}
              disabled={interactionsDisabled}
            >
              Settings
            </button>
          </footer>
        </>
      )}
    </aside>
  );
}
