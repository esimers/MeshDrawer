import { describe, expect, it } from "vitest";
import { tagCoverage } from "./tags";
import type { FileRow } from "../types";

function file(id: number, tags: string[]): FileRow {
  return {
    id,
    rootId: 1,
    relPath: `${id}.stl`,
    absPath: `C:/${id}.stl`,
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
    tags,
  };
}

describe("tagCoverage", () => {
  it("counts tags across the selection", () => {
    const rows = tagCoverage([
      file(1, ["dragon", "gift"]),
      file(2, ["dragon"]),
      file(3, []),
    ]);
    expect(rows).toEqual([
      { name: "dragon", count: 2 },
      { name: "gift", count: 1 },
    ]);
  });
});
