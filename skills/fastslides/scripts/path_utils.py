#!/usr/bin/env python3
"""Small shared path helpers for FastSlides deck scripts."""

from __future__ import annotations

import os
from pathlib import Path


def default_projects_dir() -> Path:
    configured = os.environ.get("DECK_PROJECTS_DIR") or os.environ.get("FASTSLIDES_PROJECTS_DIR")
    if configured:
        return Path(configured).expanduser().resolve()

    return (Path.cwd().resolve() / "projects").resolve()
