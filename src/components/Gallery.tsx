import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
  type RefObject,
} from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { formatBytes } from "../lib/format";
import { extLabel, fileNameFromRel, formatModified } from "../lib/files";
import { folderOfRel } from "../lib/folderTree";
import { sortMark, toggleSort, type FileSort, type GalleryView, type SortColumn } from "../lib/sort";
import { thumbUrl } from "../lib/thumbUrl";
import type { FileRow } from "../types";

const CARD_WIDTH = 200;
const CARD_HEIGHT = 196;
const LIST_ROW_HEIGHT = 40;
const LIST_GRID =
  "grid-cols-[2.5rem_minmax(7rem,1.5fr)_4.25rem_4.75rem_minmax(5rem,1fr)_5.5rem_8.5rem]";

type FileBind = {
  onMouseDown: (event: MouseEvent<HTMLButtonElement>) => void;
  onClick: (event: MouseEvent<HTMLButtonElement>) => void;
  onDoubleClick: () => void;
  onContextMenu: (event: MouseEvent<HTMLButtonElement>) => void;
  title: string;
};

type Props = {
  files: FileRow[];
  thumbsDir: string | null;
  selectedIds: number[];
  focusedId: number | null;
  thumbOverrides?: Record<number, string>;
  view: GalleryView;
  sort: FileSort;
  onSelect: (file: FileRow, event: MouseEvent) => void;
  onOpen: (file: FileRow) => void;
  onContextMenu: (file: FileRow, x: number, y: number) => void;
  onSort: (sort: FileSort) => void;
  emptyMessage?: string;
};

export function Gallery({
  files,
  thumbsDir,
  selectedIds,
  focusedId,
  thumbOverrides,
  view,
  sort,
  onSelect,
  onOpen,
  onContextMenu,
  onSort,
  emptyMessage,
}: Props) {
  const parentRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const selectedSet = useMemo(() => new Set(selectedIds), [selectedIds]);

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
  }, [view]);

  if (files.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center px-6 text-center text-sm text-md-muted">
        {emptyMessage ?? "No models in this folder."}
      </div>
    );
  }

  function bindFile(file: FileRow) {
    return {
      onMouseDown: (event: MouseEvent<HTMLButtonElement>) => {
        if (event.shiftKey || event.ctrlKey || event.metaKey) {
          event.preventDefault();
        }
      },
      onClick: (event: MouseEvent<HTMLButtonElement>) => onSelect(file, event),
      onDoubleClick: () => onOpen(file),
      onContextMenu: (event: MouseEvent<HTMLButtonElement>) => {
        event.preventDefault();
        onContextMenu(file, event.clientX, event.clientY);
      },
      title: `${file.absPath}\nCtrl+click to add · Shift+click for a range`,
    };
  }

  function selectedClass(file: FileRow): string {
    const selected = selectedSet.has(file.id);
    const focused = file.id === focusedId;
    if (!selected) {
      return "border-md-border bg-md-surface/80 hover:border-md-cyan";
    }
    return focused
      ? "border-md-magenta bg-md-surface shadow-[0_0_12px_rgba(255,43,214,0.35)]"
      : "border-md-cyan bg-md-surface/90";
  }

  if (view === "list") {
    return (
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <div
          className={`grid shrink-0 items-center gap-2 border-b border-md-border bg-md-surface/80 px-3 py-1 text-[11px] uppercase tracking-wide text-md-muted ${LIST_GRID}`}
        >
          <span />
          <ListHead label="Name" column="name" sort={sort} onSort={onSort} />
          <ListHead label="Type" column="type" sort={sort} onSort={onSort} />
          <ListHead label="Size" column="size" sort={sort} onSort={onSort} />
          <ListHead label="Folder" column="folder" sort={sort} onSort={onSort} />
          <span className="truncate">Status</span>
          <ListHead label="Modified" column="modified" sort={sort} onSort={onSort} />
        </div>
        <ListBody
          files={files}
          thumbsDir={thumbsDir}
          thumbOverrides={thumbOverrides}
          parentRef={parentRef}
          bindFile={bindFile}
          selectedClass={selectedClass}
        />
      </div>
    );
  }

  return (
    <GridBody
      files={files}
      thumbsDir={thumbsDir}
      thumbOverrides={thumbOverrides}
      parentRef={parentRef}
      width={width}
      bindFile={bindFile}
      selectedClass={selectedClass}
    />
  );
}

function ListHead({
  label,
  column,
  sort,
  onSort,
}: {
  label: string;
  column: SortColumn;
  sort: FileSort;
  onSort: (sort: FileSort) => void;
}) {
  const mark = sortMark(sort, column);
  return (
    <button
      type="button"
      onClick={() => onSort(toggleSort(sort, column))}
      className={`truncate text-left hover:text-md-cyan ${mark ? "text-md-cyan" : ""}`}
    >
      {label}
      {mark === "asc" ? " ▲" : mark === "desc" ? " ▼" : ""}
    </button>
  );
}

function ListBody({
  files,
  thumbsDir,
  thumbOverrides,
  parentRef,
  bindFile,
  selectedClass,
}: {
  files: FileRow[];
  thumbsDir: string | null;
  thumbOverrides?: Record<number, string>;
  parentRef: RefObject<HTMLDivElement | null>;
  bindFile: (file: FileRow) => FileBind;
  selectedClass: (file: FileRow) => string;
}) {
  const virtualizer = useVirtualizer({
    count: files.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => LIST_ROW_HEIGHT,
    overscan: 12,
  });

  return (
    <div ref={parentRef} className="min-h-0 flex-1 overflow-auto">
      <div className="relative w-full" style={{ height: `${virtualizer.getTotalSize()}px` }}>
        {virtualizer.getVirtualItems().map((row) => {
          const file = files[row.index];
          const thumbName = thumbOverrides?.[file.id] ?? file.thumbPath;
          const src = thumbName && thumbsDir ? thumbUrl(thumbsDir, thumbName) : null;
          const folder = folderOfRel(file.relPath);
          return (
            <button
              key={file.id}
              type="button"
              {...bindFile(file)}
              className={`absolute left-0 grid w-full items-center gap-2 border-b border-l-2 px-3 text-left ${LIST_GRID} ${selectedClass(file)}`}
              style={{
                transform: `translateY(${row.start}px)`,
                height: `${row.size}px`,
              }}
            >
              <span className="relative flex h-7 w-7 items-center justify-center overflow-hidden rounded bg-md-bg">
                {file.lastError ? (
                  <span
                    className="absolute right-0 top-0 h-1.5 w-1.5 rounded-full bg-red-500"
                    title={file.lastError}
                  />
                ) : null}
                {src ? (
                  <img src={src} alt="" className="h-full w-full object-contain" />
                ) : (
                  <span className="text-[9px] font-semibold text-md-cyan">{extLabel(file.ext)}</span>
                )}
              </span>
              <span className="truncate text-sm text-md-text">{fileNameFromRel(file.relPath)}</span>
              <span className="truncate text-xs text-md-muted">{extLabel(file.ext)}</span>
              <span className="truncate text-xs text-md-muted">{formatBytes(file.size)}</span>
              <span className="truncate text-xs text-md-muted" title={folder}>
                {folder || "—"}
              </span>
              <span className="truncate text-xs text-md-muted">
                {file.status && file.status !== "unset" ? file.status : "—"}
              </span>
              <span className="truncate text-xs text-md-muted">{formatModified(file.mtimeMs)}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

function GridBody({
  files,
  thumbsDir,
  thumbOverrides,
  parentRef,
  width,
  bindFile,
  selectedClass,
}: {
  files: FileRow[];
  thumbsDir: string | null;
  thumbOverrides?: Record<number, string>;
  parentRef: RefObject<HTMLDivElement | null>;
  width: number;
  bindFile: (file: FileRow) => FileBind;
  selectedClass: (file: FileRow) => string;
}) {
  const columns = Math.max(1, Math.floor((width || CARD_WIDTH) / CARD_WIDTH));
  const rowCount = Math.ceil(files.length / columns);
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => CARD_HEIGHT,
    overscan: 4,
  });

  return (
    <div ref={parentRef} className="min-h-0 flex-1 overflow-auto px-4 py-4">
      <div className="relative w-full" style={{ height: `${virtualizer.getTotalSize()}px` }}>
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
                const src = thumbName && thumbsDir ? thumbUrl(thumbsDir, thumbName) : null;
                return (
                  <button
                    key={file.id}
                    type="button"
                    {...bindFile(file)}
                    className={`flex h-[184px] flex-col overflow-hidden rounded-lg border text-left [content-visibility:auto] ${selectedClass(file)}`}
                  >
                    <div className="relative flex h-28 items-center justify-center bg-md-bg">
                      {file.lastError ? (
                        <span
                          className="absolute right-1 top-1 h-2 w-2 rounded-full bg-red-500"
                          title={file.lastError}
                        />
                      ) : null}
                      {src ? (
                        <img
                          src={src}
                          alt=""
                          loading="lazy"
                          decoding="async"
                          className="h-full w-full object-contain"
                        />
                      ) : (
                        <span className="rounded bg-md-border px-1.5 py-0.5 text-[10px] font-semibold tracking-wide text-md-cyan">
                          {extLabel(file.ext)}
                        </span>
                      )}
                    </div>
                    <div className="min-w-0 px-3 py-2">
                      <p className="truncate text-sm font-medium text-md-text">
                        {fileNameFromRel(file.relPath)}
                      </p>
                      <p className="mt-0.5 text-xs text-md-muted">
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
