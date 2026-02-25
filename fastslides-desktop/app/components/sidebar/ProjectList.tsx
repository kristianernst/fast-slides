"use client";

import { ProjectListItem } from "./ProjectListItem";
import type { SidebarProject } from "./types";

type ProjectListProps = {
  projects: SidebarProject[];
  pinnedPaths: string[];
  selectedPath: string;
  disabled: boolean;
  onSelectProject: (path: string) => void;
  onRemoveProject: (path: string) => void;
  onTogglePin: (path: string) => void;
};

export function ProjectList({
  projects,
  pinnedPaths,
  selectedPath,
  disabled,
  onSelectProject,
  onRemoveProject,
  onTogglePin,
}: ProjectListProps) {
  // Sort projects: pinned ones first, then alphabetical (or just preserve order of unpinned)
  const sortedProjects = [...projects].sort((a, b) => {
    const aPinned = pinnedPaths.includes(a.path);
    const bPinned = pinnedPaths.includes(b.path);
    if (aPinned && !bPinned) return -1;
    if (!aPinned && bPinned) return 1;
    return a.name.localeCompare(b.name);
  });

  return (
    <ul className="project-list" aria-label="Tracked projects">
      {sortedProjects.map((project) => (
        <ProjectListItem
          key={project.path}
          project={project}
          isActive={project.path === selectedPath}
          isPinned={pinnedPaths.includes(project.path)}
          disabled={disabled}
          onSelect={onSelectProject}
          onRemove={onRemoveProject}
          onTogglePin={onTogglePin}
        />
      ))}
      {projects.length === 0 && (
        <li className="empty-row">No projects yet. Click "Open Project" to add one.</li>
      )}
    </ul>
  );
}
