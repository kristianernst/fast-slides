"use client";

import { SidebarMinimalistic } from "@solar-icons/react";

type SidebarToggleButtonProps = {
  sidebarOpen: boolean;
  disabled?: boolean;
  onToggle: () => void;
};

export function SidebarToggleButton({
  sidebarOpen,
  disabled = false,
  onToggle,
}: SidebarToggleButtonProps) {
  return (
    <button
      type="button"
      className="btn btn-ghost btn-icon-only sidebar-toggle-btn"
      onClick={onToggle}
      disabled={disabled}
      aria-label={sidebarOpen ? "Collapse sidebar" : "Expand sidebar"}
    >
      <SidebarMinimalistic size={14} weight="Linear" />
    </button>
  );
}
