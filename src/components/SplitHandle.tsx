import type { MouseEvent } from "react";
import { clamp } from "../lib/clamp";

type Props = {
  side?: "start" | "end";
  onDrag: (size: number, containerWidth: number) => void;
};

export function SplitHandle({ side = "end", onDrag }: Props) {
  function startDrag(event: MouseEvent<HTMLDivElement>) {
    event.preventDefault();
    const split = event.currentTarget.parentElement;
    if (!split) {
      return;
    }
    const onMove = (move: globalThis.MouseEvent) => {
      const rect = split.getBoundingClientRect();
      const size =
        side === "start"
          ? clamp(move.clientX - rect.left, 160, Math.max(160, rect.width - 400))
          : clamp(rect.right - move.clientX, 240, rect.width - 280);
      onDrag(size, rect.width);
    };
    const onUp = () => {
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  return (
    <div
      role="separator"
      aria-orientation="vertical"
      title="Drag to resize"
      onMouseDown={startDrag}
      className="group relative z-10 w-2 shrink-0 cursor-col-resize bg-md-surface hover:bg-md-magenta"
    >
      <div className="absolute inset-y-0 -left-1 -right-1" />
    </div>
  );
}
