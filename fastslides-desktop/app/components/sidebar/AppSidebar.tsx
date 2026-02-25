"use client";

import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { BoxMinimalistic } from "@solar-icons/react";
import { ProjectList } from "./ProjectList";
import type { SidebarProject } from "./types";

type AppSidebarProps = {
  busy: boolean;
  sidebarOpen: boolean;
  settingsOpen: boolean;
  projectsCount: number;
  projects: SidebarProject[];
  pinnedPaths: string[];
  selectedPath: string;
  onBackToApp: () => void;
  onOpenProject: () => void;
  onSelectProject: (path: string) => void;
  onRemoveProject: (path: string) => void;
  onTogglePin: (path: string) => void;
  onOpenSettings: () => void;
};

export function AppSidebar({
  busy,
  sidebarOpen,
  settingsOpen,
  projectsCount,
  projects,
  pinnedPaths,
  selectedPath,
  onBackToApp,
  onOpenProject,
  onSelectProject,
  onRemoveProject,
  onTogglePin,
  onOpenSettings,
}: AppSidebarProps) {
  const interactionsDisabled = busy || !sidebarOpen;

  return (
    <aside className="sidebar" aria-hidden={!sidebarOpen}>
      {settingsOpen ? (
        <div className="sidebar-settings-only" data-tauri-drag-region>
          <button
            type="button"
            className="sidebar-back-link"
            onClick={onBackToApp}
            disabled={interactionsDisabled}
          >
            <ArrowLeftIcon aria-hidden="true" />
            <span>back to app</span>
          </button>
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
          </section>

          <footer className="sidebar-footer">
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
