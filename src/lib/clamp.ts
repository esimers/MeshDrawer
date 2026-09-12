export function clamp(n: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, n));
}

export function menuPosition(
  x: number,
  y: number,
  menuWidth: number,
  menuHeight: number,
  viewportWidth: number,
  viewportHeight: number,
  pad = 8,
): { left: number; top: number } {
  return {
    left: clamp(x, pad, Math.max(pad, viewportWidth - menuWidth - pad)),
    top: clamp(y, pad, Math.max(pad, viewportHeight - menuHeight - pad)),
  };
}
