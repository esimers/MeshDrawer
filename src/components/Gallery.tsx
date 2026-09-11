import { useEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { formatBytes } from "../lib/format";
import { extLabel, fileNameFromRel } from "../lib/files";
import type { FileRow } from "../types";

const CARD_WIDTH = 200;
const CARD_HEIGHT = 128;

type Props = {
  files: FileRow[];
};

export function Gallery({ files }: Props) {
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
    overscan: 8,
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
              {slice.map((file) => (
                <article
                  key={file.id}
                  className="flex h-[116px] flex-col justify-between rounded-lg border border-zinc-800 bg-zinc-900/80 p-3"
                  title={file.absPath}
                >
                  <span className="w-fit rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-semibold tracking-wide text-teal-300">
                    {extLabel(file.ext)}
                  </span>
                  <div>
                    <p className="truncate text-sm font-medium text-zinc-100">
                      {fileNameFromRel(file.relPath)}
                    </p>
                    <p className="mt-0.5 text-xs text-zinc-500">
                      {formatBytes(file.size)}
                    </p>
                  </div>
                </article>
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}
