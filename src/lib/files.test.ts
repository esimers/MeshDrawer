import { describe, expect, it } from "vitest";
import { extLabel, fileNameFromRel } from "./files";

describe("fileNameFromRel", () => {
  it("takes the last path segment", () => {
    expect(fileNameFromRel("models/nested/dragon.stl")).toBe("dragon.stl");
    expect(fileNameFromRel("wing.3mf")).toBe("wing.3mf");
  });
});

describe("extLabel", () => {
  it("uppercases the extension", () => {
    expect(extLabel("stl")).toBe("STL");
  });
});
