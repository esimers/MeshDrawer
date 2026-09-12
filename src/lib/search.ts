import type { FileRow } from "../types";

export type FileFilter = {
  query: string;
  ext: "all" | "stl" | "3mf" | "obj";
  status: "all" | "unset" | "unprinted" | "printed" | "favorite";
  tag: string;
  minMb: string;
  maxMb: string;
};

export const emptyFilter: FileFilter = {
  query: "",
  ext: "all",
  status: "all",
  tag: "",
  minMb: "",
  maxMb: "",
};

export function filterFiles(files: FileRow[], filter: FileFilter): FileRow[] {
  const q = filter.query.trim().toLowerCase();
  const tag = filter.tag.trim().toLowerCase();
  const min = Number(filter.minMb);
  const max = Number(filter.maxMb);
  const minBytes = Number.isFinite(min) && filter.minMb.trim() !== "" ? min * 1024 * 1024 : null;
  const maxBytes = Number.isFinite(max) && filter.maxMb.trim() !== "" ? max * 1024 * 1024 : null;

  return files.filter((file) => {
    if (filter.ext !== "all" && file.ext !== filter.ext) {
      return false;
    }
    if (filter.status !== "all" && file.status !== filter.status) {
      return false;
    }
    if (minBytes !== null && file.size < minBytes) {
      return false;
    }
    if (maxBytes !== null && file.size > maxBytes) {
      return false;
    }
    if (tag && !file.tags.some((t) => t.toLowerCase() === tag)) {
      return false;
    }
    if (!q) {
      return true;
    }
    const hay = [
      file.relPath,
      file.absPath,
      file.notes ?? "",
      file.status,
      ...file.tags,
    ]
      .join("\n")
      .toLowerCase();
    return hay.includes(q);
  });
}
