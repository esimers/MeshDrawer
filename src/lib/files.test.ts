import { describe, expect, it } from "vitest";
import { extLabel, fileNameFromRel, fileStem, isCadPreviewExt } from "./files";

describe("fileNameFromRel", () => {
  it("takes the last path segment", () => {
    expect(fileNameFromRel("models/nested/dragon.stl")).toBe("dragon.stl");
    expect(fileNameFromRel("wing.3mf")).toBe("wing.3mf");
  });
});

describe("extLabel", () => {
  it("uppercases the extension", () => {
    expect(extLabel("stl")).toBe("STL");
    expect(extLabel("stp")).toBe("STEP");
  });
});

describe("isCadPreviewExt", () => {
  it("marks STEP and FBX for tessellated preview", () => {
    expect(isCadPreviewExt("step")).toBe(true);
    expect(isCadPreviewExt("fbx")).toBe(true);
    expect(isCadPreviewExt("3dm")).toBe(false);
    expect(isCadPreviewExt("stl")).toBe(false);
  });
});

describe("fileStem", () => {
  it("strips the last extension", () => {
    expect(fileStem("models/dragon.stl")).toBe("dragon");
    expect(fileStem("wing.3mf")).toBe("wing");
  });
});
