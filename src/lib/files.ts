export function fileNameFromRel(relPath: string): string {
  const parts = relPath.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? relPath;
}

export function extLabel(ext: string): string {
  return ext.toUpperCase();
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
