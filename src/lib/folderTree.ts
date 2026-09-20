import type { FileRow, LibraryRoot } from "../types";

export const ALL_FOLDER_KEY = "all";

export type FolderRef = {
  key: string;
  rootId: number | null;
  folder: string;
};

export type FolderNode = FolderRef & {
  label: string;
  title?: string;
  count: number;
  children: FolderNode[];
};

export function allFolderRef(): FolderRef {
  return { key: ALL_FOLDER_KEY, rootId: null, folder: "" };
}

export function folderKey(rootId: number | null, folder: string): string {
  if (rootId === null) {
    return ALL_FOLDER_KEY;
  }
  return folder ? `root:${rootId}:${folder}` : `root:${rootId}`;
}

export function normalizeRel(relPath: string): string {
  return relPath.replace(/\\/g, "/").replace(/^\/+|\/+$/g, "");
}

export function folderOfRel(relPath: string): string {
  const rel = normalizeRel(relPath);
  const slash = rel.lastIndexOf("/");
  return slash <= 0 ? "" : rel.slice(0, slash);
}

export function rootLabel(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "");
  const parts = trimmed.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] || path;
}

export function fileInFolder(file: FileRow, ref: FolderRef): boolean {
  if (ref.rootId === null) {
    return true;
  }
  if (file.rootId !== ref.rootId) {
    return false;
  }
  if (!ref.folder) {
    return true;
  }
  const dir = folderOfRel(file.relPath);
  return dir === ref.folder || dir.startsWith(`${ref.folder}/`);
}

export function filesInFolder(files: FileRow[], ref: FolderRef): FileRow[] {
  if (ref.rootId === null) {
    return files;
  }
  return files.filter((file) => fileInFolder(file, ref));
}

type Acc = {
  label: string;
  title?: string;
  rootId: number | null;
  folder: string;
  count: number;
  children: Map<string, Acc>;
};

export function buildFolderTree(roots: LibraryRoot[], files: FileRow[]): FolderNode {
  const all: Acc = {
    label: "All",
    rootId: null,
    folder: "",
    count: files.length,
    children: new Map(),
  };

  for (const root of roots) {
    all.children.set(String(root.id), {
      label: rootLabel(root.path),
      title: root.path,
      rootId: root.id,
      folder: "",
      count: 0,
      children: new Map(),
    });
  }

  for (const file of files) {
    let node = all.children.get(String(file.rootId));
    if (!node) {
      node = {
        label: `Library ${file.rootId}`,
        rootId: file.rootId,
        folder: "",
        count: 0,
        children: new Map(),
      };
      all.children.set(String(file.rootId), node);
    }
    node.count += 1;
    const dir = folderOfRel(file.relPath);
    if (!dir) {
      continue;
    }
    let prefix = "";
    let current = node;
    for (const part of dir.split("/").filter(Boolean)) {
      prefix = prefix ? `${prefix}/${part}` : part;
      let child = current.children.get(part);
      if (!child) {
        child = {
          label: part,
          rootId: file.rootId,
          folder: prefix,
          count: 0,
          children: new Map(),
        };
        current.children.set(part, child);
      }
      child.count += 1;
      current = child;
    }
  }

  return freeze(all);
}

export function findFolderNode(tree: FolderNode, key: string): FolderNode | null {
  if (tree.key === key) {
    return tree;
  }
  for (const child of tree.children) {
    const hit = findFolderNode(child, key);
    if (hit) {
      return hit;
    }
  }
  return null;
}

export function ancestorKeys(tree: FolderNode, key: string): string[] {
  const path: string[] = [];
  function walk(node: FolderNode): boolean {
    if (node.key === key) {
      return true;
    }
    path.push(node.key);
    for (const child of node.children) {
      if (walk(child)) {
        return true;
      }
    }
    path.pop();
    return false;
  }
  walk(tree);
  return path;
}

function freeze(acc: Acc): FolderNode {
  const children = [...acc.children.values()]
    .map(freeze)
    .sort((a, b) => a.label.localeCompare(b.label, undefined, { sensitivity: "base" }));
  return {
    key: folderKey(acc.rootId, acc.folder),
    label: acc.label,
    title: acc.title,
    rootId: acc.rootId,
    folder: acc.folder,
    count: acc.count,
    children,
  };
}
