"use client";

import { SidebarMinimalistic } from "@solar-icons/react";

type SidebarToggleButtonProps = {
  sidebarOpen: boolean;
  onToggle: () => void;
};

export function SidebarToggleButton({ sidebarOpen, onToggle }: SidebarToggleButtonProps) {
  return (
    <button
      type="button"
      className="btn btn-ghost btn-icon-only sidebar-toggle-btn"
      onClick={onToggle}
      aria-label={sidebarOpen ? "Collapse sidebar" : "Expand sidebar"}
    >
      <SidebarMinimalistic size={14} weight="Linear" />
    </button>
  );
}
