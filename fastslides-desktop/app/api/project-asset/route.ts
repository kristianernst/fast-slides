import { promises as fs } from "node:fs";
import path from "node:path";

import { NextRequest, NextResponse } from "next/server";

const PROJECT_ROOT_ABSOLUTE_ASSET_PREFIXES = [
  "/assets/",
  "/images/",
  "/media/",
  "/data/",
];

const MIME_TYPES: Record<string, string> = {
  ".avif": "image/avif",
  ".bmp": "image/bmp",
  ".css": "text/css; charset=utf-8",
  ".csv": "text/csv; charset=utf-8",
  ".gif": "image/gif",
  ".jpeg": "image/jpeg",
  ".jpg": "image/jpeg",
  ".json": "application/json; charset=utf-8",
  ".m4v": "video/x-m4v",
  ".md": "text/markdown; charset=utf-8",
  ".mov": "video/quicktime",
  ".mp4": "video/mp4",
  ".ogg": "video/ogg",
  ".ogv": "video/ogg",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".txt": "text/plain; charset=utf-8",
  ".webm": "video/webm",
  ".webp": "image/webp",
};

function normalizeProjectRelativeAsset(raw: string): string | null {
  const trimmed = raw.trim();
  if (
    !trimmed ||
    trimmed.startsWith("#") ||
    /^[a-z][a-z0-9+.-]*:/i.test(trimmed)
  ) {
    return null;
  }

  const pathOnly = trimmed.match(/^([^?#]*)(.*)$/)?.[1] ?? trimmed;
  if (!pathOnly) {
    return null;
  }

  let relative = pathOnly.replace(/\\/g, "/");
  if (relative.startsWith("/")) {
    const supportedRootRelative = PROJECT_ROOT_ABSOLUTE_ASSET_PREFIXES.some(
      (prefix) =>
        relative === prefix.slice(0, -1) || relative.startsWith(prefix),
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

  return normalizedParts.length ? normalizedParts.join("/") : null;
}

function mimeTypeForFile(filePath: string): string {
  return (
    MIME_TYPES[path.extname(filePath).toLowerCase()] ??
    "application/octet-stream"
  );
}

export async function GET(request: NextRequest): Promise<NextResponse> {
  const projectPath = request.nextUrl.searchParams.get("projectPath")?.trim() || "";
  const rawSrc = request.nextUrl.searchParams.get("src")?.trim() || "";
  const normalizedRelative = normalizeProjectRelativeAsset(rawSrc);
  if (!projectPath || !normalizedRelative) {
    return NextResponse.json(
      { error: "Missing or invalid project asset parameters." },
      { status: 400 },
    );
  }

  const canonicalProjectPath = await fs.realpath(projectPath).catch(() => "");
  if (!canonicalProjectPath) {
    return NextResponse.json(
      { error: "Project path could not be resolved." },
      { status: 404 },
    );
  }

  const resolvedAssetPath = path.resolve(canonicalProjectPath, normalizedRelative);
  const projectRootWithSep = canonicalProjectPath.endsWith(path.sep)
    ? canonicalProjectPath
    : `${canonicalProjectPath}${path.sep}`;
  if (
    resolvedAssetPath !== canonicalProjectPath &&
    !resolvedAssetPath.startsWith(projectRootWithSep)
  ) {
    return NextResponse.json(
      { error: "Asset path escapes project folder." },
      { status: 400 },
    );
  }

  const stat = await fs.stat(resolvedAssetPath).catch(() => null);
  if (!stat?.isFile()) {
    return NextResponse.json({ error: "Asset not found." }, { status: 404 });
  }

  const bytes = await fs.readFile(resolvedAssetPath);
  return new NextResponse(bytes, {
    status: 200,
    headers: {
      "cache-control": "no-store",
      "content-length": String(bytes.byteLength),
      "content-type": mimeTypeForFile(resolvedAssetPath),
    },
  });
}
