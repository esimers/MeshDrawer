import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { fileNameFromRel } from "../lib/files";
import { menuPosition } from "../lib/clamp";
import type { FileRow, SlicerApp } from "../types";

export type FileMenuAction =
  | { type: "preview" }
  | { type: "open"; slicerId: string }
  | { type: "reveal" }
  | { type: "copy-path" }
  | { type: "copy-name" }
  | { type: "rename" }
  | { type: "delete" }
  | { type: "properties" };

type Props = {
  file: FileRow;
  x: number;
  y: number;
  slicers: SlicerApp[];
  onAction: (action: FileMenuAction) => void;
  onClose: () => void;
};

export function FileContextMenu({ file, x, y, slicers, onAction, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: x, top: y });
  const available = slicers.filter((s) => s.available);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) {
      return;
    }
    const rect = el.getBoundingClientRect();
    setPos(
      menuPosition(x, y, rect.width, rect.height, window.innerWidth, window.innerHeight),
    );
  }, [x, y]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };
    const onDown = (event: MouseEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) {
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onDown);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      role="menu"
      className="fixed z-50 min-w-52 rounded-md border border-zinc-700 bg-zinc-900 py-1 text-sm shadow-xl"
      style={{ left: pos.left, top: pos.top }}
    >
      <p className="truncate px-3 py-1.5 text-[11px] text-zinc-500">
        {fileNameFromRel(file.relPath)}
      </p>
      <MenuItem
        label="Open"
        hint="Preview"
        onClick={() => onAction({ type: "preview" })}
      />
      {available.map((slicer) => (
        <MenuItem
          key={slicer.id}
          label={`Open with ${slicer.name}`}
          onClick={() => onAction({ type: "open", slicerId: slicer.id })}
        />
      ))}
      <div className="my-1 h-px bg-zinc-800" />
      <MenuItem label="Show in Explorer" onClick={() => onAction({ type: "reveal" })} />
      <MenuItem label="Copy path" onClick={() => onAction({ type: "copy-path" })} />
      <MenuItem label="Copy name" onClick={() => onAction({ type: "copy-name" })} />
      <div className="my-1 h-px bg-zinc-800" />
      <MenuItem label="Rename" hint="F2" onClick={() => onAction({ type: "rename" })} />
      <MenuItem label="Delete" hint="Del" onClick={() => onAction({ type: "delete" })} />
      <MenuItem label="Properties" onClick={() => onAction({ type: "properties" })} />
    </div>
  );
}

function MenuItem({
  label,
  hint,
  onClick,
}: {
  label: string;
  hint?: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      onClick={onClick}
      className="flex w-full items-center justify-between gap-6 px-3 py-1.5 text-left text-zinc-100 hover:bg-teal-700"
    >
      <span>{label}</span>
      {hint ? <span className="text-[11px] text-zinc-400">{hint}</span> : null}
    </button>
  );
}
