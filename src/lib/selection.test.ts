import { describe, expect, it } from "vitest";
import {
  applySelection,
  emptySelection,
  gestureFromModifiers,
  selectAll,
} from "./selection";

const ids = [1, 2, 3, 4, 5];

describe("gestureFromModifiers", () => {
  it("maps click, ctrl, shift, and ctrl+shift", () => {
    expect(gestureFromModifiers(false, false)).toBe("replace");
    expect(gestureFromModifiers(false, true)).toBe("toggle");
    expect(gestureFromModifiers(true, false)).toBe("range");
    expect(gestureFromModifiers(true, true)).toBe("add-range");
  });
});

describe("applySelection", () => {
  it("replace selects only the clicked file", () => {
    const next = applySelection(
      ids,
      { selectedIds: [1, 2], anchorId: 1, focusedId: 2 },
      4,
      "replace",
    );
    expect(next).toEqual({ selectedIds: [4], anchorId: 4, focusedId: 4 });
  });

  it("toggle adds and removes", () => {
    const added = applySelection(ids, { selectedIds: [1], anchorId: 1, focusedId: 1 }, 3, "toggle");
    expect(added.selectedIds).toEqual([1, 3]);
    expect(added.focusedId).toBe(3);
    const removed = applySelection(ids, added, 1, "toggle");
    expect(removed.selectedIds).toEqual([3]);
    expect(removed.focusedId).toBe(3);
  });

  it("range selects from the anchor", () => {
    const next = applySelection(
      ids,
      { selectedIds: [2], anchorId: 2, focusedId: 2 },
      5,
      "range",
    );
    expect(next.selectedIds).toEqual([2, 3, 4, 5]);
    expect(next.anchorId).toBe(2);
    expect(next.focusedId).toBe(5);
  });

  it("add-range unions with the current selection", () => {
    const next = applySelection(
      ids,
      { selectedIds: [1], anchorId: 1, focusedId: 1 },
      3,
      "add-range",
    );
    expect(next.selectedIds).toEqual([1, 2, 3]);
  });

  it("keep focuses an already selected file", () => {
    const next = applySelection(
      ids,
      { selectedIds: [1, 4], anchorId: 1, focusedId: 1 },
      4,
      "keep",
    );
    expect(next.selectedIds).toEqual([1, 4]);
    expect(next.focusedId).toBe(4);
  });
});

describe("selectAll", () => {
  it("selects every visible id", () => {
    const next = selectAll(ids, emptySelection);
    expect(next.selectedIds).toEqual(ids);
    expect(next.focusedId).toBe(1);
  });
});
