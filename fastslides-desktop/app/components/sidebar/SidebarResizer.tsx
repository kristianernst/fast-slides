"use client";

import type { PointerEvent as ReactPointerEvent } from "react";

type SidebarResizerProps = {
  sidebarWidth: number;
  sidebarOpen: boolean;
  minWidth: number;
  maxWidth: number;
  onPointerDown: (event: ReactPointerEvent<HTMLDivElement>) => void;
};

export function SidebarResizer({
  sidebarWidth,
  sidebarOpen,
  minWidth,
  maxWidth,
  onPointerDown,
}: SidebarResizerProps) {
  return (
    <div
      className="sidebar-resizer"
      role="separator"
      aria-label="Resize sidebar"
      aria-orientation="vertical"
      aria-valuemin={minWidth}
      aria-valuemax={maxWidth}
      aria-valuenow={sidebarWidth}
      aria-hidden={!sidebarOpen}
      onPointerDown={onPointerDown}
    />
  );
}
