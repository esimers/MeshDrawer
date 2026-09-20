import { fileNameFromRel } from "./files";
import { folderOfRel } from "./folderTree";
import type { FileRow } from "../types";

export type FileSort =
  | "name"
  | "name-desc"
  | "newest"
  | "oldest"
  | "largest"
  | "smallest"
  | "type"
  | "folder";

export type GalleryView = "grid" | "list";

export const FILE_SORTS: { id: FileSort; label: string }[] = [
  { id: "name", label: "Name A–Z" },
  { id: "name-desc", label: "Name Z–A" },
  { id: "newest", label: "Newest" },
  { id: "oldest", label: "Oldest" },
  { id: "largest", label: "Largest" },
  { id: "smallest", label: "Smallest" },
  { id: "type", label: "Type" },
  { id: "folder", label: "Folder" },
];

const SORT_IDS = new Set(FILE_SORTS.map((item) => item.id));

export function parseFileSort(value: string | null): FileSort {
  if (value && SORT_IDS.has(value as FileSort)) {
    return value as FileSort;
  }
  return "name";
}

export function parseGalleryView(value: string | null): GalleryView {
  return value === "list" ? "list" : "grid";
}

function cmpName(a: FileRow, b: FileRow): number {
  return fileNameFromRel(a.relPath).localeCompare(fileNameFromRel(b.relPath), undefined, {
    numeric: true,
    sensitivity: "base",
  });
}

function cmpFolder(a: FileRow, b: FileRow): number {
  const folders = folderOfRel(a.relPath).localeCompare(folderOfRel(b.relPath), undefined, {
    numeric: true,
    sensitivity: "base",
  });
  return folders !== 0 ? folders : cmpName(a, b);
}

export function sortFiles(files: FileRow[], sort: FileSort): FileRow[] {
  const copy = files.slice();
  copy.sort((a, b) => {
    switch (sort) {
      case "name":
        return cmpName(a, b);
      case "name-desc":
        return cmpName(b, a);
      case "newest":
        return b.mtimeMs - a.mtimeMs || cmpName(a, b);
      case "oldest":
        return a.mtimeMs - b.mtimeMs || cmpName(a, b);
      case "largest":
        return b.size - a.size || cmpName(a, b);
      case "smallest":
        return a.size - b.size || cmpName(a, b);
      case "type": {
        const ext = a.ext.localeCompare(b.ext, undefined, { sensitivity: "base" });
        return ext !== 0 ? ext : cmpName(a, b);
      }
      case "folder":
        return cmpFolder(a, b);
    }
  });
  return copy;
}

export type SortColumn = "name" | "type" | "size" | "folder" | "modified";

export function toggleSort(current: FileSort, column: SortColumn): FileSort {
  switch (column) {
    case "name":
      return current === "name" ? "name-desc" : "name";
    case "type":
      return "type";
    case "size":
      return current === "largest" ? "smallest" : "largest";
    case "folder":
      return "folder";
    case "modified":
      return current === "newest" ? "oldest" : "newest";
  }
}

export function sortMark(current: FileSort, column: SortColumn): "asc" | "desc" | null {
  switch (column) {
    case "name":
      if (current === "name") {
        return "asc";
      }
      if (current === "name-desc") {
        return "desc";
      }
      return null;
    case "type":
      return current === "type" ? "asc" : null;
    case "size":
      if (current === "largest") {
        return "desc";
      }
      if (current === "smallest") {
        return "asc";
      }
      return null;
    case "folder":
      return current === "folder" ? "asc" : null;
    case "modified":
      if (current === "newest") {
        return "desc";
      }
      if (current === "oldest") {
        return "asc";
      }
      return null;
  }
}
