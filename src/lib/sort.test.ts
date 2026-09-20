import { describe, expect, it } from "vitest";
import type { FileRow } from "../types";
import { parseFileSort, sortFiles, toggleSort } from "./sort";

function row(partial: Partial<FileRow> & Pick<FileRow, "id" | "relPath">): FileRow {
  return {
    rootId: 1,
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

describe("sortFiles", () => {
  const files = [
    row({ id: 1, relPath: "models/file10.stl", size: 10, mtimeMs: 100, ext: "stl" }),
    row({ id: 2, relPath: "models/file2.stl", size: 50, mtimeMs: 300, ext: "stl" }),
    row({ id: 3, relPath: "box.3mf", size: 20, mtimeMs: 200, ext: "3mf" }),
  ];

  it("sorts names with numeric order", () => {
    expect(sortFiles(files, "name").map((f) => f.id)).toEqual([3, 2, 1]);
  });

  it("sorts newest first", () => {
    expect(sortFiles(files, "newest").map((f) => f.id)).toEqual([2, 3, 1]);
  });

  it("sorts largest first", () => {
    expect(sortFiles(files, "largest").map((f) => f.id)).toEqual([2, 3, 1]);
  });

  it("groups by type then name", () => {
    expect(sortFiles(files, "type").map((f) => f.ext)).toEqual(["3mf", "stl", "stl"]);
  });
});

describe("toggleSort", () => {
  it("flips name and size directions", () => {
    expect(toggleSort("name", "name")).toBe("name-desc");
    expect(toggleSort("name-desc", "name")).toBe("name");
    expect(toggleSort("name", "size")).toBe("largest");
    expect(toggleSort("largest", "size")).toBe("smallest");
    expect(toggleSort("newest", "modified")).toBe("oldest");
  });
});

describe("parseFileSort", () => {
  it("falls back to name", () => {
    expect(parseFileSort("newest")).toBe("newest");
    expect(parseFileSort("nope")).toBe("name");
  });
});
