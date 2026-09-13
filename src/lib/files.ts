export function fileNameFromRel(relPath: string): string {
  const parts = relPath.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? relPath;
}

export function extLabel(ext: string): string {
  if (ext === "stp") {
    return "STEP";
  }
  return ext.toUpperCase();
}

export function isCadPreviewExt(ext: string): boolean {
  return ext === "step" || ext === "stp" || ext === "fbx";
}

export function isMeshThumbExt(ext: string): boolean {
  return ext === "stl" || ext === "obj" || ext === "fbx";
}

export function fileStem(relPath: string): string {
  const name = fileNameFromRel(relPath);
  const dot = name.lastIndexOf(".");
  if (dot <= 0) {
    return name;
  }
  return name.slice(0, dot);
}

export function formatModified(mtimeMs: number): string {
  if (!mtimeMs) {
    return "—";
  }
  return new Date(mtimeMs).toLocaleString();
}
