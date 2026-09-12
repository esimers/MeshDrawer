import { describe, expect, it } from "vitest";
import { emptyFilter, filterFiles } from "./search";
import type { FileRow } from "../types";

function row(partial: Partial<FileRow>): FileRow {
  return {
    id: 1,
    rootId: 1,
    relPath: "models/dragon.stl",
    absPath: "C:\\lib\\models\\dragon.stl",
    ext: "stl",
    size: 5 * 1024 * 1024,
    mtimeMs: 0,
    thumbStatus: "none",
    thumbPath: null,
    plateCount: 0,
    triangleCount: null,
    lastError: null,
    sha256: null,
    ignoredDupe: false,
    notes: "to print",
    status: "unprinted",
    tags: ["dragon", "gift"],
    ...partial,
  };
}

describe("filterFiles", () => {
  const files = [
    row({ id: 1 }),
    row({
      id: 2,
      relPath: "box.3mf",
      absPath: "C:\\lib\\box.3mf",
      ext: "3mf",
      status: "printed",
      tags: ["box"],
      size: 80 * 1024 * 1024,
      notes: "",
    }),
  ];

  it("matches name, notes, and tags", () => {
    expect(filterFiles(files, { ...emptyFilter, query: "dragon" })).toHaveLength(1);
    expect(filterFiles(files, { ...emptyFilter, query: "gift" })).toHaveLength(1);
    expect(filterFiles(files, { ...emptyFilter, query: "to print" })).toHaveLength(1);
  });

  it("filters format status size and tag", () => {
    expect(filterFiles(files, { ...emptyFilter, ext: "3mf" })[0].id).toBe(2);
    expect(filterFiles(files, { ...emptyFilter, status: "printed" })[0].id).toBe(2);
    expect(filterFiles(files, { ...emptyFilter, tag: "dragon" })[0].id).toBe(1);
    expect(filterFiles(files, { ...emptyFilter, minMb: "50" })[0].id).toBe(2);
  });
});
