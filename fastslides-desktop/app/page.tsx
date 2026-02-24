"use client";

import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ComponentType,
  type CSSProperties,
  type ImgHTMLAttributes,
  type PointerEvent as ReactPointerEvent,
} from "react";
import Image from "next/image";
import { invoke } from "@tauri-apps/api/core";
import * as jsxRuntime from "react/jsx-runtime";
import {
  BoxMinimalistic,
  CloseCircle,
  Maximize,
  Minimize,
  Moon,
  Pin,
  Settings,
  SidebarMinimalistic,
  Sun,
} from "@solar-icons/react";

type AppConfig = {
  projects_roots: string[];
  recent_projects: string[];
};

type ProjectSummary = {
  name: string;
  path: string;
  root: string;
  slide_count: number;
  updated_at: number;
};

type ProjectDetail = {
  name: string;
  path: string;
  root: string;
  page_mdx: string;
  slide_count: number;
  updated_at: number;
};

type AppState = {
  config: AppConfig;
  projects: ProjectSummary[];
};

type SlideOutlineEntry = {
  index: number;
  title: string;
};

type SlideTocTick = {
  key: string;
  ratio: number;
  slideIndex: number;
  title: string;
};

function isTauriRuntime(): boolean {
  if (typeof window === "undefined") {
    return false;
  }
  const runtimeWindow = window as Window & {
    __TAURI_INTERNALS__?: unknown;
    __TAURI__?: unknown;
  };
  return Boolean(runtimeWindow.__TAURI_INTERNALS__ || runtimeWindow.__TAURI__);
}

async function call<T>(command: string, payload?: Record<string, unknown>): Promise<T> {
  if (!isTauriRuntime()) {
    throw new Error("Tauri runtime not detected. Launch this screen with `npm run tauri:dev`.");
  }
  return invoke<T>(command, payload);
}

async function pickFolder(title: string): Promise<string> {
  if (!isTauriRuntime()) {
    throw new Error("Folder picker is available only in Tauri runtime.");
  }
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    directory: true,
    multiple: false,
    title,
  });
  if (typeof selected === "string") {
    return selected;
  }
  return "";
}

const SELECTED_STATE_KEY = "fastslides_selected_path";
const SIDEBAR_WIDTH_STATE_KEY = "fastslides_sidebar_width";
const PINNED_STATE_KEY = "fastslides_pinned_paths";
const THEME_STATE_KEY = "fastslides_theme";
const SIDEBAR_MIN_WIDTH = 220;
const SIDEBAR_MAX_WIDTH = 420;
const EXPORT_SKILL_MENU_EVENT = "fastslides://export-skill";
const PREVIEW_ZOOM_MIN = 0.8;
const PREVIEW_ZOOM_MAX = 2.5;
const PREVIEW_ZOOM_STEP = 0.05;
const PROJECT_ROOT_ABSOLUTE_ASSET_PREFIXES = ["/assets/", "/images/", "/media/", "/data/"];
const projectAssetDataUrlCache = new Map<string, string>();

function clampPreviewZoom(zoom: number): number {
  return Math.min(PREVIEW_ZOOM_MAX, Math.max(PREVIEW_ZOOM_MIN, zoom));
}

function clampSidebarWidth(width: number): number {
  return Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, width));
}

function stripFrontmatter(source: string): string {
  return source.replace(/^---\s*\n[\s\S]*?\n---\s*(?:\n|$)/, "");
}

function normalizeDeckSource(source: string): string {
  return stripFrontmatter(source).trim();
}

function clampUnit(value: number): number {
  return Math.min(1, Math.max(0, value));
}

function isExternalAssetPath(value: string): boolean {
  const lower = value.toLowerCase();
  return (
    value.startsWith("//") ||
    lower.startsWith("http://") ||
    lower.startsWith("https://") ||
    lower.startsWith("data:") ||
    lower.startsWith("blob:") ||
    lower.startsWith("mailto:") ||
    lower.startsWith("tel:")
  );
}

function splitAssetPathAndSuffix(raw: string): { pathOnly: string; suffix: string } {
  const match = raw.match(/^([^?#]*)(.*)$/);
  return {
    pathOnly: match?.[1] ?? raw,
    suffix: match?.[2] ?? "",
  };
}

function normalizeProjectRelativeAsset(raw: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed || trimmed.startsWith("#") || isExternalAssetPath(trimmed)) {
    return null;
  }

  const { pathOnly } = splitAssetPathAndSuffix(trimmed);
  if (!pathOnly) {
    return null;
  }

  let relative = pathOnly.replace(/\\/g, "/");
  if (relative.startsWith("/")) {
    const supportedRootRelative = PROJECT_ROOT_ABSOLUTE_ASSET_PREFIXES.some(
      (prefix) => relative === prefix.slice(0, -1) || relative.startsWith(prefix),
    );
    if (!supportedRootRelative) {
      return null;
    }
    relative = relative.slice(1);
  }

  const normalizedParts: string[] = [];
  for (const part of relative.split("/")) {
    if (!part || part === ".") {
      continue;
    }
    if (part === "..") {
      if (normalizedParts.length === 0) {
        return null;
      }
      normalizedParts.pop();
      continue;
    }
    normalizedParts.push(part);
  }

  if (normalizedParts.length === 0) {
    return null;
  }

  return normalizedParts.join("/");
}

function ProjectAssetImage({
  projectPath,
  ...props
}: ImgHTMLAttributes<HTMLImageElement> & { projectPath: string }) {
  const source = typeof props.src === "string" ? props.src : "";
  const [resolvedSrc, setResolvedSrc] = useState(source);

  useEffect(() => {
    if (typeof props.src !== "string") {
      return;
    }

    const normalizedRelative = normalizeProjectRelativeAsset(props.src);
    if (!normalizedRelative || !projectPath || !isTauriRuntime()) {
      setResolvedSrc(props.src);
      return;
    }

    const cacheKey = `${projectPath}::${normalizedRelative}`;
    const cached = projectAssetDataUrlCache.get(cacheKey);
    if (cached) {
      setResolvedSrc(cached);
      return;
    }

    let cancelled = false;
    setResolvedSrc(props.src);
    call<string>("resolve_project_asset_data_url", {
      projectPath,
      rawSrc: props.src,
    })
      .then((nextSource) => {
        if (cancelled) {
          return;
        }
        projectAssetDataUrlCache.set(cacheKey, nextSource);
        setResolvedSrc(nextSource);
      })
      .catch(() => {
        if (!cancelled) {
          setResolvedSrc(source);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [projectPath, props.src]);

  return <img {...props} src={resolvedSrc || source} loading={props.loading ?? "lazy"} />;
}

function EmbeddedDeckPreview({
  source,
  projectPath,
  presenterMode,
  activeSlideIndex,
  onSlideCountChange,
  onActiveSlidePick,
  onSlideOutlineChange,
}: {
  source: string;
  projectPath: string;
  presenterMode: boolean;
  activeSlideIndex: number;
  onSlideCountChange: (count: number) => void;
  onActiveSlidePick: (index: number) => void;
  onSlideOutlineChange: (slides: SlideOutlineEntry[]) => void;
}) {
  const [CompiledDeck, setCompiledDeck] = useState<ComponentType<Record<string, unknown>> | null>(null);
  const [error, setError] = useState("");
  const previewRootRef = useRef<HTMLDivElement | null>(null);
  const slideElementsRef = useRef<HTMLElement[]>([]);
  const previousActiveSlideRef = useRef(-1);
  const mdxComponents = useMemo(
    () => ({
      img: (props: ImgHTMLAttributes<HTMLImageElement>) => {
        return <ProjectAssetImage {...props} projectPath={projectPath} />;
      },
    }),
    [projectPath],
  );

  useEffect(() => {
    let cancelled = false;

    async function compileSource() {
      if (!source.trim()) {
        setError("Project has an empty page.mdx.");
        setCompiledDeck(null);
        return;
      }

      setError("");
      setCompiledDeck(null);

      try {
        const { evaluate } = await import("@mdx-js/mdx");
        const module = await evaluate(source, {
          ...jsxRuntime,
          development: false,
        });

        if (!cancelled) {
          setCompiledDeck(() => module.default as ComponentType<Record<string, unknown>>);
        }
      } catch (cause) {
        if (cancelled) {
          return;
        }
        const message =
          cause instanceof Error ? cause.message : "Failed to compile page.mdx for embedded preview.";
        setError(message);
      }
    }

    void compileSource();

    return () => {
      cancelled = true;
    };
  }, [source]);

  useEffect(() => {
    const root = previewRootRef.current;
    if (!root || !CompiledDeck) {
      slideElementsRef.current = [];
      previousActiveSlideRef.current = -1;
      onSlideCountChange(0);
      onSlideOutlineChange([]);
      return;
    }

    const slides = Array.from(root.querySelectorAll<HTMLElement>(".slide"));
    slideElementsRef.current = slides;
    previousActiveSlideRef.current = -1;
    onSlideCountChange(slides.length);
    root.classList.remove("embedded-preview-single");

    const nextOutline: SlideOutlineEntry[] = [];
    for (let index = 0; index < slides.length; index += 1) {
      const slide = slides[index];
      slide.dataset.slideIndex = String(index);
      const heading = slide.querySelector("h1, h2, h3");
      const headingText = heading?.textContent?.trim();
      nextOutline.push({
        index,
        title: headingText && headingText.length > 0 ? headingText : `Slide ${index + 1}`,
      });
    }
    onSlideOutlineChange(nextOutline);

    if (slides.length === 0) {
      return;
    }

    for (let index = 0; index < slides.length; index += 1) {
      slides[index].dataset.active = "false";
      slides[index].setAttribute("aria-hidden", "false");
    }
  }, [CompiledDeck, onSlideCountChange, onSlideOutlineChange]);

  useEffect(() => {
    const root = previewRootRef.current;
    const slides = slideElementsRef.current;
    if (!root || slides.length === 0) {
      return;
    }

    const safeIndex = Math.min(activeSlideIndex, slides.length - 1);
    const wasSingleMode = root.classList.contains("embedded-preview-single");
    root.classList.toggle("embedded-preview-single", presenterMode);

    if (!presenterMode) {
      if (wasSingleMode) {
        for (let index = 0; index < slides.length; index += 1) {
          slides[index].setAttribute("aria-hidden", "false");
        }
      }
      const previous = previousActiveSlideRef.current;
      if (previous >= 0 && previous < slides.length && previous !== safeIndex) {
        slides[previous].dataset.active = "false";
      }
      slides[safeIndex].dataset.active = "true";
      previousActiveSlideRef.current = safeIndex;
      return;
    }

    const previous = previousActiveSlideRef.current;
    if (previous >= 0 && previous < slides.length && previous !== safeIndex) {
      slides[previous].dataset.active = "false";
      slides[previous].setAttribute("aria-hidden", "true");
    }

    slides[safeIndex].dataset.active = "true";
    slides[safeIndex].setAttribute("aria-hidden", "false");

    if (!wasSingleMode) {
      for (let index = 0; index < slides.length; index += 1) {
        if (index !== safeIndex) {
          slides[index].setAttribute("aria-hidden", "true");
        }
      }
    }

    previousActiveSlideRef.current = safeIndex;
  }, [activeSlideIndex, presenterMode]);

  useEffect(() => {
    const root = previewRootRef.current;
    if (!root) {
      return;
    }

    const onClick = (event: MouseEvent): void => {
      if (presenterMode) {
        return;
      }
      const target = event.target as HTMLElement | null;
      const slide = target?.closest(".slide") as HTMLElement | null;
      if (!slide) {
        return;
      }
      const slides = Array.from(root.querySelectorAll<HTMLElement>(".slide"));
      const clickedIndex = slides.indexOf(slide);
      if (clickedIndex >= 0) {
        onActiveSlidePick(clickedIndex);
      }
    };

    root.addEventListener("click", onClick);
    return () => {
      root.removeEventListener("click", onClick);
    };
  }, [onActiveSlidePick, presenterMode]);

  if (error) {
    return <div className="embedded-preview-error">{error}</div>;
  }

  if (!CompiledDeck) {
    return <div className="embedded-preview-loading">Loading deck preview…</div>;
  }

  return (
    <div ref={previewRootRef} className="embedded-preview-deck">
      <CompiledDeck components={mdxComponents} />
    </div>
  );
}

export default function Home() {
  const [appState, setAppState] = useState<AppState | null>(null);
  const [selectedPath, setSelectedPath] = useState("");
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [sidebarWidth, setSidebarWidth] = useState(280);
  const [sidebarDragging, setSidebarDragging] = useState(false);
  const [pinnedPaths, setPinnedPaths] = useState<string[]>([]);
  const [selectedProjectDetail, setSelectedProjectDetail] = useState<ProjectDetail | null>(null);
  const [presenterMode, setPresenterMode] = useState(false);
  const [activeSlideIndex, setActiveSlideIndex] = useState(0);
  const [embeddedSlideCount, setEmbeddedSlideCount] = useState(0);
  const [slideOutline, setSlideOutline] = useState<SlideOutlineEntry[]>([]);
  const [tocPositionRatio, setTocPositionRatio] = useState(0);
  const [previewDockVisible, setPreviewDockVisible] = useState(true);
  const [previewZoom, setPreviewZoom] = useState(1);
  const [busy, setBusy] = useState(false);
  const [theme, setTheme] = useState<"dark" | "light">(() => {
    if (typeof window !== "undefined") {
      const stored = localStorage.getItem(THEME_STATE_KEY);
      if (stored === "light") return "light";
    }
    return "dark";
  });
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [projectCss, setProjectCss] = useState("");
  const [cssEditorValue, setCssEditorValue] = useState("");
  const sidebarResizeCleanupRef = useRef<(() => void) | null>(null);
  const previewDockHideTimerRef = useRef<number | null>(null);
  const previewDockHoveringRef = useRef(false);
  const previewSurfaceRef = useRef<HTMLDivElement | null>(null);

  // Restore selection on load
  useEffect(() => {
    if (typeof window !== "undefined") {
      const stored = localStorage.getItem(SELECTED_STATE_KEY);
      if (stored) {
        setSelectedPath(stored);
      }
    }
  }, []);

  // Sync selection
  useEffect(() => {
    if (typeof window !== "undefined" && selectedPath) {
      localStorage.setItem(SELECTED_STATE_KEY, selectedPath);
    }
  }, [selectedPath]);

  useEffect(() => {
    if (typeof window !== "undefined") {
      const storedWidth = Number(localStorage.getItem(SIDEBAR_WIDTH_STATE_KEY));
      if (Number.isFinite(storedWidth)) {
        setSidebarWidth(clampSidebarWidth(storedWidth));
      }
    }
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    try {
      const raw = localStorage.getItem(PINNED_STATE_KEY);
      if (!raw) {
        return;
      }
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        const nextPinned = Array.from(
          new Set(parsed.filter((item): item is string => typeof item === "string")),
        );
        setPinnedPaths(nextPinned);
      }
    } catch {
      // Ignore malformed local state and continue with empty pin set.
    }
  }, []);

  useEffect(() => {
    if (typeof window !== "undefined") {
      localStorage.setItem(SIDEBAR_WIDTH_STATE_KEY, String(sidebarWidth));
    }
  }, [sidebarWidth]);

  useEffect(() => {
    if (typeof window !== "undefined") {
      localStorage.setItem(PINNED_STATE_KEY, JSON.stringify(pinnedPaths));
    }
  }, [pinnedPaths]);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    if (typeof window !== "undefined") {
      localStorage.setItem(THEME_STATE_KEY, theme);
    }
  }, [theme]);

  const projects = appState?.projects || [];
  useEffect(() => {
    const validPaths = new Set(projects.map((project) => project.path));
    setPinnedPaths((previous) => {
      const next = previous.filter((path) => validPaths.has(path));
      return next.length === previous.length ? previous : next;
    });
  }, [projects]);

  const orderedProjects = useMemo(() => {
    const pinnedSet = new Set(pinnedPaths);
    const pinOrder = new Map(pinnedPaths.map((path, index) => [path, index]));
    const pinnedProjects = projects
      .filter((project) => pinnedSet.has(project.path))
      .sort((left, right) => {
        const leftOrder = pinOrder.get(left.path) ?? Number.MAX_SAFE_INTEGER;
        const rightOrder = pinOrder.get(right.path) ?? Number.MAX_SAFE_INTEGER;
        return leftOrder - rightOrder;
      });
    const unpinnedProjects = projects.filter((project) => !pinnedSet.has(project.path));
    return [...pinnedProjects, ...unpinnedProjects];
  }, [pinnedPaths, projects]);

  const selectedProject = useMemo(() => {
    if (!selectedPath) {
      return null;
    }
    return projects.find((project) => project.path === selectedPath) || null;
  }, [projects, selectedPath]);

  const selectedProjectEmbeddedSource = useMemo(
    () => normalizeDeckSource(selectedProjectDetail?.page_mdx || ""),
    [selectedProjectDetail?.page_mdx],
  );

  const visibleSlideCount = useMemo(() => {
    const fallbackCount = selectedProjectDetail?.slide_count || selectedProject?.slide_count || 0;
    return embeddedSlideCount > 0 ? embeddedSlideCount : fallbackCount;
  }, [embeddedSlideCount, selectedProject?.slide_count, selectedProjectDetail?.slide_count]);

  const maxSlideIndex = Math.max(visibleSlideCount - 1, 0);
  const slideTocEntries = useMemo<SlideOutlineEntry[]>(() => {
    if (slideOutline.length > 0) {
      return slideOutline;
    }
    return Array.from({ length: visibleSlideCount }, (_, index) => ({
      index,
      title: `Slide ${index + 1}`,
    }));
  }, [slideOutline, visibleSlideCount]);

  const slideTocTicks = useMemo<SlideTocTick[]>(() => {
    const denominator = Math.max(slideTocEntries.length - 1, 1);
    return slideTocEntries.map((entry, order) => ({
      key: `major-${entry.index}`,
      ratio: order / denominator,
      slideIndex: entry.index,
      title: entry.title,
    }));
  }, [slideTocEntries]);

  useEffect(() => {
    setActiveSlideIndex(0);
    setEmbeddedSlideCount(0);
    setSlideOutline([]);
    setTocPositionRatio(0);
  }, [selectedProject?.path]);

  useEffect(() => {
    if (!selectedProject) {
      setPreviewDockVisible(false);
      clearPreviewDockHideTimer();
      return;
    }
    revealPreviewDock(1800);
  }, [selectedProject?.path]);

  useEffect(() => {
    setActiveSlideIndex((previous) => Math.min(previous, maxSlideIndex));
  }, [maxSlideIndex]);

  useEffect(() => {
    if (!selectedProject) {
      setSelectedProjectDetail(null);
      return;
    }

    let cancelled = false;
    call<ProjectDetail>("load_project", { path: selectedProject.path })
      .then((detail) => {
        if (!cancelled) {
          setSelectedProjectDetail(detail);
        }
      })
      .catch((cause) => {
        if (!cancelled) {
          const message = cause instanceof Error ? cause.message : "Failed to load project source.";
          console.log(message);
          setSelectedProjectDetail(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [selectedProject?.path]);

  useEffect(() => {
    if (!selectedProject) {
      setProjectCss("");
      setCssEditorValue("");
      return;
    }
    let cancelled = false;
    call<string>("read_project_css", { path: selectedProject.path })
      .then((css) => {
        if (!cancelled) {
          setProjectCss(css);
          setCssEditorValue(css);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setProjectCss("");
          setCssEditorValue("");
        }
      });
    return () => { cancelled = true; };
  }, [selectedProject?.path]);

  useEffect(() => {
    const id = "fastslides-project-css";
    let style = document.getElementById(id) as HTMLStyleElement | null;
    if (!style) {
      style = document.createElement("style");
      style.id = id;
      document.head.appendChild(style);
    }
    style.textContent = projectCss;
    return () => { style?.remove(); };
  }, [projectCss]);

  useEffect(() => {
    if (previewDockHideTimerRef.current !== null) {
      window.clearTimeout(previewDockHideTimerRef.current);
      previewDockHideTimerRef.current = null;
    }
    return () => {
      if (previewDockHideTimerRef.current !== null) {
        window.clearTimeout(previewDockHideTimerRef.current);
      }
    };
  }, []);

  function getCurrentCenteredSlideIndex(): number {
    const container = previewSurfaceRef.current;
    if (!container) {
      return -1;
    }
    const slides = Array.from(container.querySelectorAll<HTMLElement>(".embedded-preview-deck .slide"));
    if (slides.length === 0) {
      return -1;
    }

    const viewportCenterY = container.getBoundingClientRect().top + container.clientHeight / 2;
    let nextIndex = 0;
    let minDistance = Number.POSITIVE_INFINITY;

    for (let position = 0; position < slides.length; position += 1) {
      const slide = slides[position];
      const rect = slide.getBoundingClientRect();
      const slideCenter = rect.top + rect.height / 2;
      const distance = Math.abs(slideCenter - viewportCenterY);
      if (distance < minDistance) {
        minDistance = distance;
        const explicitIndex = Number(slide.dataset.slideIndex);
        nextIndex = Number.isFinite(explicitIndex) ? explicitIndex : position;
      }
    }

    return nextIndex;
  }

  function scrollListToSlide(index: number): void {
    const container = previewSurfaceRef.current;
    if (!container) {
      return;
    }
    const target = container.querySelector<HTMLElement>(`.embedded-preview-deck .slide[data-slide-index="${index}"]`);
    if (!target) {
      return;
    }
    target.scrollIntoView({
      behavior: "smooth",
      block: "center",
      inline: "nearest",
    });
  }

  function handleTocSelect(index: number): void {
    setActiveSlideIndex(index);
    if (!presenterMode) {
      scrollListToSlide(index);
    }
    revealPreviewDock();
  }

  useEffect(() => {
    if (!selectedProject || presenterMode) {
      return;
    }

    const container = previewSurfaceRef.current;
    if (!container) {
      return;
    }

    let rafId = 0;
    const updateActiveSlide = (): void => {
      rafId = 0;
      const maxScroll = Math.max(container.scrollHeight - container.clientHeight, 0);
      const ratio = maxScroll > 0 ? container.scrollTop / maxScroll : 0;
      setTocPositionRatio(clampUnit(ratio));
      const nextIndex = getCurrentCenteredSlideIndex();
      if (nextIndex >= 0) {
        setActiveSlideIndex((previous) => (previous === nextIndex ? previous : nextIndex));
      }
    };

    const requestUpdate = (): void => {
      if (rafId !== 0) {
        return;
      }
      rafId = window.requestAnimationFrame(updateActiveSlide);
    };

    container.addEventListener("scroll", requestUpdate, { passive: true });
    window.addEventListener("resize", requestUpdate);
    requestUpdate();

    return () => {
      if (rafId !== 0) {
        window.cancelAnimationFrame(rafId);
      }
      container.removeEventListener("scroll", requestUpdate);
      window.removeEventListener("resize", requestUpdate);
    };
  }, [embeddedSlideCount, presenterMode, selectedProject]);

  useEffect(() => {
    if (!presenterMode) {
      return;
    }
    const ratio = maxSlideIndex > 0 ? activeSlideIndex / maxSlideIndex : 0;
    setTocPositionRatio(clampUnit(ratio));
  }, [activeSlideIndex, maxSlideIndex, presenterMode]);

  async function refreshState(preferredPath = ""): Promise<string> {
    const nextState = await call<AppState>("get_app_state");
    setAppState(nextState);

    const fallbackPath = nextState.projects[0]?.path || "";
    const hasPreferred = preferredPath && nextState.projects.some((item) => item.path === preferredPath);
    const nextSelection = hasPreferred ? preferredPath : fallbackPath;
    setSelectedPath(nextSelection);
    return nextSelection;
  }

  useEffect(() => {
    let cancelled = false;

    async function boot() {
      try {
        await refreshState();
        if (!cancelled) {
          console.log("Ready");
        }
      } catch (error) {
        if (!cancelled) {
          const message = error instanceof Error ? error.message : "Failed to load app state.";
          console.log(message);
        }
      }
    }

    void boot();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    return () => {
      sidebarResizeCleanupRef.current?.();
      document.body.classList.remove("sidebar-resize-active");
    };
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let disposed = false;
    let unlisten: (() => void) | null = null;

    async function registerMenuListener() {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const cleanup = await listen(EXPORT_SKILL_MENU_EVENT, () => {
          void handleExportSkillArchive();
        });

        if (disposed) {
          cleanup();
          return;
        }

        unlisten = cleanup;
      } catch (error) {
        const message =
          error instanceof Error ? error.message : "Failed to register application menu listener.";
        console.log(message);
      }
    }

    void registerMenuListener();

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      const target = event.target as HTMLElement | null;
      const targetTag = target?.tagName;
      const isTypingTarget =
        Boolean(target?.isContentEditable) || targetTag === "INPUT" || targetTag === "TEXTAREA" || targetTag === "SELECT";
      if (isTypingTarget) {
        return;
      }

      const usingCommand = event.metaKey || event.ctrlKey;
      if (usingCommand && (event.key === "=" || event.key === "+" || event.key === "Add")) {
        event.preventDefault();
        setPreviewZoom((previous) => clampPreviewZoom(previous + PREVIEW_ZOOM_STEP));
        return;
      }

      if (usingCommand && (event.key === "-" || event.key === "_" || event.key === "Subtract")) {
        event.preventDefault();
        setPreviewZoom((previous) => clampPreviewZoom(previous - PREVIEW_ZOOM_STEP));
        return;
      }

      if (usingCommand && (event.key === "b" || event.key === "B")) {
        event.preventDefault();
        setSidebarOpen((open) => !open);
        return;
      }

      if (event.key === "Escape" && settingsOpen) {
        event.preventDefault();
        setSettingsOpen(false);
        return;
      }

      if (event.key === "Escape" && presenterMode) {
        event.preventDefault();
        setPresenterMode(false);
        window.requestAnimationFrame(() => {
          scrollListToSlide(activeSlideIndex);
        });
        revealPreviewDock();
        return;
      }

      if (!presenterMode || !selectedProject) {
        return;
      }

      if (event.key === "ArrowRight" || event.key === "PageDown" || event.key === " ") {
        event.preventDefault();
        setActiveSlideIndex((previous) => Math.min(previous + 1, maxSlideIndex));
        return;
      }

      if (event.key === "ArrowLeft" || event.key === "PageUp") {
        event.preventDefault();
        setActiveSlideIndex((previous) => Math.max(previous - 1, 0));
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [activeSlideIndex, maxSlideIndex, presenterMode, selectedProject, settingsOpen]);

  function clearPreviewDockHideTimer(): void {
    if (previewDockHideTimerRef.current !== null) {
      window.clearTimeout(previewDockHideTimerRef.current);
      previewDockHideTimerRef.current = null;
    }
  }

  function schedulePreviewDockHide(delayMs: number): void {
    clearPreviewDockHideTimer();
    previewDockHideTimerRef.current = window.setTimeout(() => {
      if (!previewDockHoveringRef.current) {
        setPreviewDockVisible(false);
      }
      previewDockHideTimerRef.current = null;
    }, delayMs);
  }

  function revealPreviewDock(hideDelayMs = 1400): void {
    setPreviewDockVisible(true);
    schedulePreviewDockHide(hideDelayMs);
  }

  function togglePresenterMode(): void {
    if (presenterMode) {
      setPresenterMode(false);
      window.requestAnimationFrame(() => {
        scrollListToSlide(activeSlideIndex);
      });
      revealPreviewDock();
      return;
    }

    const centeredSlide = getCurrentCenteredSlideIndex();
    if (centeredSlide >= 0) {
      setActiveSlideIndex(centeredSlide);
    }
    setPresenterMode(true);
    revealPreviewDock();
  }

  function handlePreviewStagePointerMove(event: ReactPointerEvent<HTMLDivElement>): void {
    const stageBounds = event.currentTarget.getBoundingClientRect();
    const distanceFromBottom = stageBounds.bottom - event.clientY;
    if (distanceFromBottom <= 148) {
      revealPreviewDock();
      return;
    }
    if (!previewDockHoveringRef.current) {
      schedulePreviewDockHide(220);
    }
  }

  function handlePreviewStagePointerLeave(): void {
    if (!previewDockHoveringRef.current) {
      schedulePreviewDockHide(140);
    }
  }

  function handlePreviewDockPointerEnter(): void {
    previewDockHoveringRef.current = true;
    clearPreviewDockHideTimer();
    setPreviewDockVisible(true);
  }

  function handlePreviewDockPointerLeave(): void {
    previewDockHoveringRef.current = false;
    schedulePreviewDockHide(180);
  }

  async function withBusy(task: () => Promise<void>): Promise<void> {
    setBusy(true);
    try {
      await task();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Action failed.";
      console.log(message);
    } finally {
      setBusy(false);
    }
  }

  async function handleOpenProjectFolder(): Promise<void> {
    await withBusy(async () => {
      const folder = await pickFolder("Select a project folder containing page.mdx");
      if (!folder) {
        console.log("Open project cancelled.");
        return;
      }

      const opened = await call<ProjectDetail>("open_project", { path: folder });
      await refreshState(opened.path);
      console.log(`Opened project ${opened.name}.`);
    });
  }

  async function handleRemoveProject(path: string): Promise<void> {
    await withBusy(async () => {
      const nextState = await call<AppState>("remove_project", { path });
      setAppState(nextState);

      const fallbackPath = nextState.projects[0]?.path || "";
      const preferredPath = selectedPath === path ? "" : selectedPath;
      const hasPreferred = preferredPath && nextState.projects.some((project) => project.path === preferredPath);
      const nextSelection = hasPreferred ? preferredPath : fallbackPath;
      setSelectedPath(nextSelection);
      setPinnedPaths((previous) => previous.filter((candidate) => candidate !== path));

      console.log("Removed project from tracked list.");
    });
  }

  function handleTogglePin(path: string): void {
    setPinnedPaths((previous) => {
      if (previous.includes(path)) {
        return previous.filter((candidate) => candidate !== path);
      }
      return [path, ...previous];
    });
  }

  async function handleSaveCss(): Promise<void> {
    if (!selectedProject) return;
    await withBusy(async () => {
      await call<void>("save_project_css", { path: selectedProject.path, css: cssEditorValue });
      setProjectCss(cssEditorValue);
    });
  }

  async function handleExportSkillArchive(): Promise<void> {
    await withBusy(async () => {
      if (!isTauriRuntime()) {
        throw new Error("Skill export is available only in Tauri runtime.");
      }

      const { save } = await import("@tauri-apps/plugin-dialog");
      const destination = await save({
        title: "Download FastSlides Skill",
        defaultPath: "fastslides-skill.zip",
        canCreateDirectories: true,
        filters: [
          {
            name: "ZIP archive",
            extensions: ["zip"],
          },
        ],
      });

      if (!destination || typeof destination !== "string") {
        console.log("Skill export cancelled.");
        return;
      }

      const exportedPath = await call<string>("export_fastslides_skill", {
        destination,
      });
      console.log(`FastSlides skill exported to ${exportedPath}.`);
    });
  }

  function handleSidebarResizeStart(event: ReactPointerEvent<HTMLDivElement>): void {
    if (!sidebarOpen) {
      return;
    }

    event.preventDefault();

    const startX = event.clientX;
    const startWidth = sidebarWidth;
    const pointerId = event.pointerId;
    const resizeHandle = event.currentTarget;

    try {
      resizeHandle.setPointerCapture(pointerId);
    } catch {
      // Ignore capture failures; window listeners still provide fallback behavior.
    }

    setSidebarDragging(true);
    document.body.classList.add("sidebar-resize-active");

    const handlePointerMove = (moveEvent: PointerEvent): void => {
      const deltaX = moveEvent.clientX - startX;
      setSidebarWidth(clampSidebarWidth(startWidth + deltaX));
    };

    const stopResize = (): void => {
      setSidebarDragging(false);
      document.body.classList.remove("sidebar-resize-active");
      try {
        if (resizeHandle.hasPointerCapture(pointerId)) {
          resizeHandle.releasePointerCapture(pointerId);
        }
      } catch {
        // Ignore release failures.
      }
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", stopResize);
      window.removeEventListener("pointercancel", stopResize);
      sidebarResizeCleanupRef.current = null;
    };

    sidebarResizeCleanupRef.current?.();
    sidebarResizeCleanupRef.current = stopResize;

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", stopResize);
    window.addEventListener("pointercancel", stopResize);
  }

  const shellStyle = useMemo(
    () =>
      ({
        "--sidebar-runtime-width": `${sidebarWidth}px`,
      }) as CSSProperties,
    [sidebarWidth],
  );

  const previewSurfaceStyle = useMemo(
    () =>
      ({
        "--preview-zoom": `${previewZoom}`,
      }) as CSSProperties,
    [previewZoom],
  );

  return (
    <main
      className={`app-shell ${sidebarOpen ? "sidebar-open" : "sidebar-closed"} ${sidebarDragging ? "sidebar-resizing" : ""}`}
      style={shellStyle}
    >
      <button
        type="button"
        className="btn btn-ghost btn-icon-only sidebar-toggle-btn"
        onClick={() => setSidebarOpen((open) => !open)}
        aria-label={sidebarOpen ? "Collapse sidebar" : "Expand sidebar"}
      >
        <SidebarMinimalistic size={14} weight="Linear" />
      </button>

      <aside className="sidebar" aria-hidden={!sidebarOpen} data-tauri-drag-region>
        <header className="sidebar-head" data-tauri-drag-region>
          <button
            type="button"
            className="btn btn-primary btn-block"
            onClick={() => void handleOpenProjectFolder()}
            disabled={busy || !sidebarOpen}
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
            <span className="count-pill">{projects.length}</span>
          </div>

          <ul className="project-list" aria-label="Tracked projects">
            {orderedProjects.map((project) => {
              const isPinned = pinnedPaths.includes(project.path);
              return (
              <li
                key={project.path}
                className={`project-row ${project.path === selectedPath ? "active" : ""} ${isPinned ? "pinned" : ""}`}
              >
                <button
                  type="button"
                  className={`project-action project-pin ${isPinned ? "is-pinned" : ""}`}
                  onClick={(event) => {
                    event.stopPropagation();
                    handleTogglePin(project.path);
                  }}
                  disabled={busy || !sidebarOpen}
                  aria-label={isPinned ? `Unpin ${project.name}` : `Pin ${project.name} to top`}
                >
                  <Pin size={12} weight={isPinned ? "Bold" : "Linear"} />
                </button>
                <button
                  type="button"
                  className="project-select-inline"
                  onClick={() => setSelectedPath(project.path)}
                  disabled={busy || !sidebarOpen}
                  title={project.path}
                >
                  <span className="project-title">{project.name}</span>
                </button>
                <button
                  type="button"
                  className="project-action project-remove"
                  onClick={(event) => {
                    event.stopPropagation();
                    void handleRemoveProject(project.path);
                  }}
                  disabled={busy || !sidebarOpen}
                  aria-label={`Remove ${project.name} from tracked projects`}
                >
                  <CloseCircle size={12} weight="Linear" />
                </button>
              </li>
            );
            })}
            {projects.length === 0 && (
              <li className="empty-row">No projects yet. Click "Open Project" to add one.</li>
            )}
          </ul>
        </section>

        <footer className="sidebar-foot">
          <button
            type="button"
            className="settings-trigger"
            onClick={() => setSettingsOpen(true)}
            aria-label="Settings"
          >
            <Settings size={14} weight="Linear" />
            <span>Settings</span>
          </button>
        </footer>
      </aside>

      <div
        className="sidebar-resizer"
        role="separator"
        aria-label="Resize sidebar"
        aria-orientation="vertical"
        aria-valuemin={SIDEBAR_MIN_WIDTH}
        aria-valuemax={SIDEBAR_MAX_WIDTH}
        aria-valuenow={sidebarWidth}
        aria-hidden={!sidebarOpen}
        onPointerDown={handleSidebarResizeStart}
      />

      <section className="workspace" aria-label="Preview area">
        {selectedProject ? (
          <div
            className={`preview-stage ${presenterMode ? "presenter-mode single-slide-mode" : "list-slide-mode"}`}
            onPointerMove={handlePreviewStagePointerMove}
            onPointerLeave={handlePreviewStagePointerLeave}
          >
            <div className="preview-render-surface" style={previewSurfaceStyle}>
              <div ref={previewSurfaceRef} className="embedded-preview-surface">
                <EmbeddedDeckPreview
                  source={selectedProjectEmbeddedSource}
                  projectPath={selectedProject.path}
                  presenterMode={presenterMode}
                  activeSlideIndex={activeSlideIndex}
                  onSlideCountChange={setEmbeddedSlideCount}
                  onActiveSlidePick={setActiveSlideIndex}
                  onSlideOutlineChange={setSlideOutline}
                />
              </div>
            </div>
            {slideTocEntries.length > 1 && (
              <nav className="slides-toc-rail" aria-label="Slide table of contents">
                <ol className="slides-toc-list">
                  {slideTocTicks.map((tick) => {
                    const top = `${tick.ratio * 100}%`;
                    const isActive = tick.slideIndex === activeSlideIndex;
                    return (
                      <li
                        key={tick.key}
                        className={`slides-toc-tick ${isActive ? "active" : ""}`}
                        style={{ top }}
                      >
                        <button
                          type="button"
                          className="slides-toc-hit"
                          onClick={() => handleTocSelect(tick.slideIndex)}
                          aria-label={`Go to ${tick.title}`}
                          title={tick.title}
                        >
                          <span className="slides-toc-line" />
                        </button>
                        <span className="slides-toc-label">{tick.title}</span>
                      </li>
                    );
                  })}
                </ol>
                <div className="slides-toc-current" style={{ top: `${tocPositionRatio * 100}%` }}>
                  <span className="slides-toc-current-line" />
                </div>
              </nav>
            )}
            <div
              className={`preview-mode-dock ${previewDockVisible ? "is-visible" : ""}`}
              onPointerEnter={handlePreviewDockPointerEnter}
              onPointerLeave={handlePreviewDockPointerLeave}
            >
              <button
                type="button"
                className={`preview-dock-toggle ${presenterMode ? "active" : ""}`}
                onClick={togglePresenterMode}
                aria-pressed={presenterMode}
                aria-label={presenterMode ? "Switch to slide list mode" : "Switch to focused single-slide mode"}
                title={presenterMode ? "Minimize to slide list" : "Maximize to single slide"}
              >
                {presenterMode ? (
                  <Minimize size={16} weight="Linear" />
                ) : (
                  <Maximize size={16} weight="Linear" />
                )}
              </button>
            </div>
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

      {settingsOpen && (
        <div className="settings-overlay" onClick={() => setSettingsOpen(false)}>
          <div className="settings-dialog" onClick={(e) => e.stopPropagation()}>
            <header className="settings-header">
              <h2>Settings</h2>
              <button
                type="button"
                className="settings-close"
                onClick={() => setSettingsOpen(false)}
                aria-label="Close settings"
              >
                <CloseCircle size={16} weight="Linear" />
              </button>
            </header>
            <div className="settings-body">
              <div className="settings-section">
                <span className="settings-label">Theme</span>
                <div className="theme-toggle">
                  <button
                    type="button"
                    className={theme === "dark" ? "active" : ""}
                    onClick={() => setTheme("dark")}
                  >
                    <Moon size={14} weight="Linear" /> Dark
                  </button>
                  <button
                    type="button"
                    className={theme === "light" ? "active" : ""}
                    onClick={() => setTheme("light")}
                  >
                    <Sun size={14} weight="Linear" /> Light
                  </button>
                </div>
              </div>
              {selectedProject && (
                <div className="settings-section">
                  <span className="settings-label">Slide Styles</span>
                  <p className="settings-hint">
                    slides.css for {selectedProject.name} — edit --slide-* variables
                  </p>
                  <textarea
                    className="css-editor"
                    value={cssEditorValue}
                    onChange={(e) => setCssEditorValue(e.target.value)}
                    spellCheck={false}
                    placeholder={":root {\n  --slide-bg: #1a1a2e;\n  --slide-h1-color: #e2e8f0;\n}"}
                  />
                  <button
                    type="button"
                    className="btn btn-primary"
                    onClick={() => void handleSaveCss()}
                    disabled={busy}
                  >
                    Save
                  </button>
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </main>
  );
}
