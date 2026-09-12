import { describe, expect, it } from "vitest";
import { clamp, menuPosition } from "./clamp";

describe("clamp", () => {
  it("bounds a number", () => {
    expect(clamp(5, 0, 10)).toBe(5);
    expect(clamp(-2, 0, 10)).toBe(0);
    expect(clamp(99, 0, 10)).toBe(10);
  });
});

describe("menuPosition", () => {
  it("keeps the menu on screen", () => {
    expect(menuPosition(10, 10, 180, 240, 800, 600)).toEqual({ left: 10, top: 10 });
    expect(menuPosition(790, 590, 180, 240, 800, 600)).toEqual({
      left: 800 - 180 - 8,
      top: 600 - 240 - 8,
    });
  });
});
