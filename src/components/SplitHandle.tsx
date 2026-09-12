import type { MouseEvent } from "react";
import { clamp } from "../lib/clamp";

type Props = {
  onDrag: (widthFromRight: number, containerWidth: number) => void;
};

export function SplitHandle({ onDrag }: Props) {
  function startDrag(event: MouseEvent<HTMLDivElement>) {
    event.preventDefault();
    const split = event.currentTarget.parentElement;
    if (!split) {
      return;
    }
    const onMove = (move: globalThis.MouseEvent) => {
      const rect = split.getBoundingClientRect();
      onDrag(clamp(rect.right - move.clientX, 240, rect.width - 280), rect.width);
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
      className="group relative z-10 w-2 shrink-0 cursor-col-resize bg-zinc-900 hover:bg-teal-700"
    >
      <div className="absolute inset-y-0 -left-1 -right-1" />
    </div>
  );
}
