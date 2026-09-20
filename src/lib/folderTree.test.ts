import { describe, expect, it } from "vitest";
import type { FileRow, LibraryRoot } from "../types";
import {
  ALL_FOLDER_KEY,
  ancestorKeys,
  buildFolderTree,
  fileInFolder,
  folderKey,
  folderOfRel,
  filesInFolder,
  rootLabel,
} from "./folderTree";

function file(partial: Partial<FileRow> & Pick<FileRow, "id" | "rootId" | "relPath">): FileRow {
  return {
    absPath: `C:/${partial.relPath}`,
    ext: "stl",
    size: 1,
    mtimeMs: 1,
    thumbStatus: "none",
    thumbPath: null,
    plateCount: 0,
    triangleCount: null,
    lastError: null,
    sha256: null,
    ignoredDupe: false,
    notes: null,
    status: "unset",
    tags: [],
    ...partial,
  };
}

const roots: LibraryRoot[] = [
  { id: 1, path: "C:\\Prints", addedAt: 0 },
  { id: 2, path: "D:\\Archive", addedAt: 0 },
];

describe("folderOfRel", () => {
  it("strips the file name and normalizes slashes", () => {
    expect(folderOfRel("dragon.stl")).toBe("");
    expect(folderOfRel("models/dragon.stl")).toBe("models");
    expect(folderOfRel("models\\sub\\a.stl")).toBe("models/sub");
  });
});

describe("rootLabel", () => {
  it("uses the last path segment", () => {
    expect(rootLabel("C:\\Prints")).toBe("Prints");
    expect(rootLabel("D:/Archive/")).toBe("Archive");
  });
});

describe("buildFolderTree", () => {
  const files = [
    file({ id: 1, rootId: 1, relPath: "dragon.stl" }),
    file({ id: 2, rootId: 1, relPath: "models/box.stl" }),
    file({ id: 3, rootId: 1, relPath: "models/dragons/a.stl" }),
    file({ id: 4, rootId: 1, relPath: "models/dragons/b.3mf" }),
    file({ id: 5, rootId: 2, relPath: "old/thing.stl" }),
  ];

  it("nests folders under each library root with descendant counts", () => {
    const tree = buildFolderTree(roots, files);
    expect(tree.key).toBe(ALL_FOLDER_KEY);
    expect(tree.count).toBe(5);
    expect(tree.children.map((c) => c.label)).toEqual(["Archive", "Prints"]);

    const prints = tree.children.find((c) => c.label === "Prints")!;
    expect(prints.count).toBe(4);
    expect(prints.key).toBe(folderKey(1, ""));
    const models = prints.children.find((c) => c.label === "models")!;
    expect(models.count).toBe(3);
    expect(models.children[0]?.label).toBe("dragons");
    expect(models.children[0]?.count).toBe(2);
  });

  it("fileInFolder includes descendants and not other roots", () => {
    const models = { key: folderKey(1, "models"), rootId: 1, folder: "models" };
    expect(fileInFolder(files[1], models)).toBe(true);
    expect(fileInFolder(files[2], models)).toBe(true);
    expect(fileInFolder(files[0], models)).toBe(false);
    expect(fileInFolder(files[4], models)).toBe(false);
    expect(filesInFolder(files, models).map((f) => f.id)).toEqual([2, 3, 4]);
  });

  it("lists ancestor keys so the tree can expand to a nested folder", () => {
    const tree = buildFolderTree(roots, files);
    const keys = ancestorKeys(tree, folderKey(1, "models/dragons"));
    expect(keys).toEqual([ALL_FOLDER_KEY, folderKey(1, ""), folderKey(1, "models")]);
  });
});
