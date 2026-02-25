"use client";

import { DrawingPinIcon, DrawingPinFilledIcon, ArchiveIcon } from "@radix-ui/react-icons";
import type { SidebarProject } from "./types";

type ProjectListItemProps = {
  project: SidebarProject;
  isActive: boolean;
  isPinned: boolean;
  disabled: boolean;
  onSelect: (path: string) => void;
  onRemove: (path: string) => void;
  onTogglePin: (path: string) => void;
};

export function ProjectListItem({
  project,
  isActive,
  isPinned,
  disabled,
  onSelect,
  onRemove,
  onTogglePin,
}: ProjectListItemProps) {
  return (
    <li className={`project-row ${isActive ? "active" : ""} ${isPinned ? "pinned" : ""}`}>
      <button
        type="button"
        className="project-select-inline"
        onClick={() => onSelect(project.path)}
        disabled={disabled}
        title={project.path}
      >
        <span 
          className="project-icon-slot project-pin-toggle"
          onClick={(event) => {
            event.stopPropagation();
            if (!disabled) {
              onTogglePin(project.path);
            }
          }}
          title={isPinned ? "Unpin project" : "Pin project"}
          style={{ cursor: disabled ? "not-allowed" : "pointer" }}
        >
          {isPinned ? (
            <DrawingPinFilledIcon aria-hidden="true" />
          ) : (
            <DrawingPinIcon aria-hidden="true" />
          )}
        </span>
        <span className="project-title">{project.name}</span>
      </button>

      <button
        type="button"
        className="project-action project-remove"
        onClick={(event) => {
          event.stopPropagation();
          onRemove(project.path);
        }}
        disabled={disabled}
        aria-label={`Remove ${project.name} from tracked projects`}
      >
        <ArchiveIcon aria-hidden="true" />
      </button>
    </li>
  );
}
