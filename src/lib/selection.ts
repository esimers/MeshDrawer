export type SelectGesture = "replace" | "toggle" | "range" | "add-range" | "keep";

export type SelectionState = {
  selectedIds: number[];
  anchorId: number | null;
  focusedId: number | null;
};

export const emptySelection: SelectionState = {
  selectedIds: [],
  anchorId: null,
  focusedId: null,
};

export function gestureFromModifiers(
  shiftKey: boolean,
  ctrlOrMeta: boolean,
): SelectGesture {
  if (shiftKey && ctrlOrMeta) {
    return "add-range";
  }
  if (shiftKey) {
    return "range";
  }
  if (ctrlOrMeta) {
    return "toggle";
  }
  return "replace";
}

export function applySelection(
  visibleIds: number[],
  state: SelectionState,
  clickedId: number,
  gesture: SelectGesture,
): SelectionState {
  switch (gesture) {
    case "replace":
      return {
        selectedIds: [clickedId],
        anchorId: clickedId,
        focusedId: clickedId,
      };
    case "keep":
      if (state.selectedIds.includes(clickedId)) {
        return { ...state, focusedId: clickedId };
      }
      return {
        selectedIds: [clickedId],
        anchorId: clickedId,
        focusedId: clickedId,
      };
    case "toggle": {
      const set = new Set(state.selectedIds);
      if (set.has(clickedId)) {
        set.delete(clickedId);
      } else {
        set.add(clickedId);
      }
      const selectedIds = visibleIds.filter((id) => set.has(id));
      const focusedId = set.has(clickedId)
        ? clickedId
        : selectedIds.includes(state.focusedId ?? -1)
          ? state.focusedId
          : (selectedIds[selectedIds.length - 1] ?? null);
      return { selectedIds, anchorId: clickedId, focusedId };
    }
    case "range":
    case "add-range": {
      const range = rangeIds(visibleIds, state.anchorId ?? clickedId, clickedId);
      if (gesture === "range") {
        return {
          selectedIds: range,
          anchorId: state.anchorId ?? clickedId,
          focusedId: clickedId,
        };
      }
      const set = new Set([...state.selectedIds, ...range]);
      return {
        selectedIds: visibleIds.filter((id) => set.has(id)),
        anchorId: state.anchorId ?? clickedId,
        focusedId: clickedId,
      };
    }
  }
}

export function selectAll(visibleIds: number[], state: SelectionState): SelectionState {
  if (visibleIds.length === 0) {
    return emptySelection;
  }
  return {
    selectedIds: [...visibleIds],
    anchorId: state.anchorId ?? visibleIds[0],
    focusedId: state.focusedId && visibleIds.includes(state.focusedId)
      ? state.focusedId
      : visibleIds[0],
  };
}

function rangeIds(visibleIds: number[], fromId: number, toId: number): number[] {
  const a = visibleIds.indexOf(fromId);
  const b = visibleIds.indexOf(toId);
  if (a < 0 && b < 0) {
    return [toId];
  }
  if (a < 0) {
    return [toId];
  }
  if (b < 0) {
    return [fromId];
  }
  const lo = Math.min(a, b);
  const hi = Math.max(a, b);
  return visibleIds.slice(lo, hi + 1);
}
