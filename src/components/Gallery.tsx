import { useEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { formatBytes } from "../lib/format";
import { extLabel, fileNameFromRel } from "../lib/files";
import { thumbUrl } from "../lib/thumbUrl";
import type { FileRow } from "../types";

const CARD_WIDTH = 200;
const CARD_HEIGHT = 196;

type Props = {
  files: FileRow[];
  thumbsDir: string | null;
  selectedId: number | null;
  thumbOverrides?: Record<number, string>;
  onSelect: (file: FileRow) => void;
  onOpen: (file: FileRow) => void;
  onContextMenu: (file: FileRow, x: number, y: number) => void;
};

export function Gallery({
  files,
  thumbsDir,
  selectedId,
  thumbOverrides,
  onSelect,
  onOpen,
  onContextMenu,
}: Props) {
  const parentRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);

  useEffect(() => {
    const el = parentRef.current;
    if (!el) {
      return;
    }
    const update = () => setWidth(el.clientWidth);
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const columns = Math.max(1, Math.floor((width || CARD_WIDTH) / CARD_WIDTH));
  const rowCount = Math.ceil(files.length / columns);

  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => CARD_HEIGHT,
    overscan: 6,
  });

  if (files.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center px-6 text-center text-sm text-zinc-400">
        No STL, 3MF, or OBJ files found in the library folders.
      </div>
    );
  }

  return (
    <div ref={parentRef} className="min-h-0 flex-1 overflow-auto px-4 py-4">
      <div
        className="relative w-full"
        style={{ height: `${virtualizer.getTotalSize()}px` }}
      >
        {virtualizer.getVirtualItems().map((row) => {
          const start = row.index * columns;
          const slice = files.slice(start, start + columns);
          return (
            <div
              key={row.key}
              className="absolute left-0 top-0 grid w-full gap-3"
              style={{
                transform: `translateY(${row.start}px)`,
                gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
                height: `${row.size}px`,
              }}
            >
              {slice.map((file) => {
                const thumbName = thumbOverrides?.[file.id] ?? file.thumbPath;
                const src =
                  thumbName && thumbsDir ? thumbUrl(thumbsDir, thumbName) : null;
                const selected = file.id === selectedId;
                return (
                  <button
                    key={file.id}
                    type="button"
                    onClick={() => onSelect(file)}
                    onDoubleClick={() => onOpen(file)}
                    onContextMenu={(event) => {
                      event.preventDefault();
                      onSelect(file);
                      onContextMenu(file, event.clientX, event.clientY);
                    }}
                    title={`${file.absPath}\nRight-click for options`}
                    className={`flex h-[184px] flex-col overflow-hidden rounded-lg border text-left ${
                      selected
                        ? "border-teal-400 bg-zinc-900"
                        : "border-zinc-800 bg-zinc-900/80 hover:border-zinc-600"
                    }`}
                  >
                    <div className="flex h-28 items-center justify-center bg-zinc-950">
                      {src ? (
                        <img src={src} alt="" className="h-full w-full object-contain" />
                      ) : (
                        <span className="rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-semibold tracking-wide text-teal-300">
                          {extLabel(file.ext)}
                        </span>
                      )}
                    </div>
                    <div className="min-w-0 px-3 py-2">
                      <p className="truncate text-sm font-medium text-zinc-100">
                        {fileNameFromRel(file.relPath)}
                      </p>
                      <p className="mt-0.5 text-xs text-zinc-500">
                        {extLabel(file.ext)} · {formatBytes(file.size)}
                        {file.plateCount > 1 ? ` · ${file.plateCount} plates` : ""}
                        {file.status && file.status !== "unset" ? ` · ${file.status}` : ""}
                      </p>
                    </div>
                  </button>
                );
              })}
            </div>
          );
        })}
      </div>
    </div>
  );
}
