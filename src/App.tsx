import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { DeleteDialog, PropertiesDialog, RenameDialog } from "./components/Dialogs";
import { DetailPane } from "./components/DetailPane";
import { DuplicateView } from "./components/DuplicateView";
import { FilterBar } from "./components/FilterBar";
import { SettingsPanel } from "./components/SettingsPanel";
import { FileContextMenu, type FileMenuAction } from "./components/FileContextMenu";
import { FolderTree } from "./components/FolderTree";
import { Gallery } from "./components/Gallery";
import { SplitHandle } from "./components/SplitHandle";
import { fileNameFromRel } from "./lib/files";
import { clamp } from "./lib/clamp";
import {
  ALL_FOLDER_KEY,
  ancestorKeys,
  buildFolderTree,
  filesInFolder,
  findFolderNode,
  folderKey,
} from "./lib/folderTree";
import {
  applySelection,
  emptySelection,
  gestureFromModifiers,
  selectAll,
  type SelectionState,
} from "./lib/selection";
import { needsMeshThumb, renderOneMeshThumb } from "./lib/meshThumbs";
import { emptyFilter, filterFiles, type FileFilter } from "./lib/search";
import {
  parseFileSort,
  parseGalleryView,
  sortFiles,
  type FileSort,
  type GalleryView,
} from "./lib/sort";
import type {
  AppInfo,
  AppSettings,
  DuplicateGroup,
  FileRow,
  ScanErrorRow,
  HashProgress,
  LibraryRoot,
  ScanProgress,
  SlicerApp,
  ThumbReady,
} from "./types";
import "./App.css";

const DETAIL_WIDTH_KEY = "meshdrewer.detailWidth";
const DETAIL_WIDTH_DEFAULT = 420;
const TREE_WIDTH_KEY = "meshdrewer.treeWidth";
const TREE_WIDTH_DEFAULT = 240;
const TREE_FOLDER_KEY = "meshdrewer.treeFolder";
const VIEW_MODE_KEY = "meshdrewer.viewMode";
const SORT_KEY = "meshdrewer.sort";

const emptyHash: HashProgress = {
  running: false,
  hashed: 0,
  remaining: 0,
  currentPath: "",
};

const emptyProgress: ScanProgress = {
  running: false,
  paused: false,
  seen: 0,
  indexed: 0,
  errors: 0,
  currentPath: "",
  rootPath: "",
};

function patchThumb(files: FileRow[], ready: ThumbReady): FileRow[] {
  return files.map((file) =>
    file.id === ready.id
      ? {
          ...file,
          thumbStatus: ready.thumbStatus,
          thumbPath: ready.thumbPath ?? file.thumbPath,
          plateCount: ready.plateCount || file.plateCount,
        }
      : file,
  );
}

function App() {
  const [roots, setRoots] = useState<LibraryRoot[]>([]);
  const [files, setFiles] = useState<FileRow[]>([]);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [progress, setProgress] = useState<ScanProgress>(emptyProgress);
  const [slicers, setSlicers] = useState<SlicerApp[]>([]);
  const [selection, setSelection] = useState<SelectionState>(emptySelection);
  const [thumbOverrides, setThumbOverrides] = useState<Record<number, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [thumbTick, setThumbTick] = useState(0);
  const [detailWidth, setDetailWidth] = useState(() => {
    const stored = Number(localStorage.getItem(DETAIL_WIDTH_KEY));
    return Number.isFinite(stored) && stored >= 240 ? stored : DETAIL_WIDTH_DEFAULT;
  });
  const [treeWidth, setTreeWidth] = useState(() => {
    const stored = Number(localStorage.getItem(TREE_WIDTH_KEY));
    return Number.isFinite(stored) && stored >= 160 ? stored : TREE_WIDTH_DEFAULT;
  });
  const [folderKeyState, setFolderKeyState] = useState(
    () => localStorage.getItem(TREE_FOLDER_KEY) ?? ALL_FOLDER_KEY,
  );
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set([ALL_FOLDER_KEY]));
  const collapsedByUser = useRef(new Set<string>());
  const [galleryView, setGalleryView] = useState<GalleryView>(() =>
    parseGalleryView(localStorage.getItem(VIEW_MODE_KEY)),
  );
  const [fileSort, setFileSort] = useState<FileSort>(() =>
    parseFileSort(localStorage.getItem(SORT_KEY)),
  );
  const [menu, setMenu] = useState<{ file: FileRow; x: number; y: number } | null>(null);
  const [renameFile, setRenameFile] = useState<FileRow | null>(null);
  const [deleteFile, setDeleteFile] = useState<FileRow | null>(null);
  const [propsFile, setPropsFile] = useState<FileRow | null>(null);
  const [view, setView] = useState<"gallery" | "dupes" | "settings">("gallery");
  const [filter, setFilter] = useState<FileFilter>(emptyFilter);
  const [allTags, setAllTags] = useState<string[]>([]);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [scanErrors, setScanErrors] = useState<ScanErrorRow[]>([]);
  const [showErrors, setShowErrors] = useState(false);
  const [dupeGroups, setDupeGroups] = useState<DuplicateGroup[]>([]);
  const [hashProgress, setHashProgress] = useState<HashProgress>(emptyHash);
  const meshBusy = useRef(false);

  const refresh = useCallback(async () => {
    const [nextRoots, nextFiles, nextInfo, nextProgress, nextSlicers, nextHash, nextDupes, nextTags, nextSettings, nextScanErrors] =
      await Promise.all([
        invoke<LibraryRoot[]>("list_roots"),
        invoke<FileRow[]>("list_files"),
        invoke<AppInfo>("get_app_info"),
        invoke<ScanProgress>("get_scan_status"),
        invoke<SlicerApp[]>("list_slicers"),
        invoke<HashProgress>("get_hash_status"),
        invoke<DuplicateGroup[]>("list_duplicate_groups"),
        invoke<string[]>("list_tags"),
        invoke<AppSettings>("get_app_settings"),
        invoke<ScanErrorRow[]>("list_scan_errors"),
      ]);
    setRoots(nextRoots);
    setFiles(nextFiles);
    setInfo(nextInfo);
    setProgress(nextProgress);
    setSlicers(nextSlicers);
    setHashProgress(nextHash);
    setDupeGroups(nextDupes);
    setAllTags(nextTags);
    setSettings(nextSettings);
    setScanErrors(nextScanErrors);
  }, []);

  useEffect(() => {
    refresh().catch((err: unknown) => {
      setError(err instanceof Error ? err.message : String(err));
    });
  }, [refresh]);

  useEffect(() => {
    let cancelled = false;
    const unsubs: Array<() => void> = [];

    async function attach() {
      const add = (unlisten: () => void) => {
        if (cancelled) {
          unlisten();
        } else {
          unsubs.push(unlisten);
        }
      };
      add(
        await listen<ScanProgress>("scan-progress", (event) => {
          setProgress(event.payload);
        }),
      );
      add(
        await listen("scan-complete", () => {
          void refresh();
        }),
      );
      add(
        await listen("library-changed", () => {
          void refresh();
        }),
      );
      add(
        await listen<ThumbReady>("thumb-ready", (event) => {
          setFiles((prev) => patchThumb(prev, event.payload));
        }),
      );
      add(
        await listen<HashProgress>("hash-progress", (event) => {
          setHashProgress(event.payload);
        }),
      );
      add(
        await listen("hash-complete", () => {
          void invoke<DuplicateGroup[]>("list_duplicate_groups").then(setDupeGroups);
          void invoke<HashProgress>("get_hash_status").then(setHashProgress);
        }),
      );
    }

    attach().catch((err: unknown) => {
      setError(err instanceof Error ? err.message : String(err));
    });

    return () => {
      cancelled = true;
      unsubs.forEach((fn) => fn());
    };
  }, [refresh]);

  useEffect(() => {
    if (!progress.running) {
      return;
    }
    const id = window.setInterval(() => {
      void invoke<FileRow[]>("list_files").then(setFiles).catch(() => undefined);
    }, 1500);
    return () => window.clearInterval(id);
  }, [progress.running]);

  useEffect(() => {
    if (progress.running || meshBusy.current) {
      return;
    }
    const next = files.find(needsMeshThumb);
    if (!next) {
      return;
    }
    meshBusy.current = true;
    renderOneMeshThumb(next)
      .catch((err: unknown) => {
        console.warn("mesh thumb failed", err);
      })
      .finally(() => {
        meshBusy.current = false;
        setThumbTick((n) => n + 1);
      });
  }, [files, progress.running, thumbTick]);

  const resizeDetail = useCallback((widthFromRight: number) => {
    setDetailWidth(widthFromRight);
    localStorage.setItem(DETAIL_WIDTH_KEY, String(Math.round(widthFromRight)));
  }, []);

  const resizeTree = useCallback((width: number) => {
    const next = clamp(width, 160, 420);
    setTreeWidth(next);
    localStorage.setItem(TREE_WIDTH_KEY, String(Math.round(next)));
  }, []);

  const preferredSlicer = slicers.find((s) => s.available)?.id ?? "system";

  const runFileAction = useCallback(
    async (file: FileRow, action: FileMenuAction) => {
      setError(null);
      try {
        if (action.type === "preview") {
          setSelection((prev) =>
            prev.selectedIds.includes(file.id)
              ? { ...prev, focusedId: file.id }
              : { selectedIds: [file.id], anchorId: file.id, focusedId: file.id },
          );
          return;
        }
        if (action.type === "open") {
          await invoke("open_in_slicer", { id: file.id, slicerId: action.slicerId });
          return;
        }
        if (action.type === "reveal") {
          await invoke("reveal_in_explorer", { id: file.id });
          return;
        }
        if (action.type === "copy-path") {
          await navigator.clipboard.writeText(file.absPath);
          return;
        }
        if (action.type === "copy-name") {
          await navigator.clipboard.writeText(fileNameFromRel(file.relPath));
          return;
        }
        if (action.type === "rename") {
          setRenameFile(file);
          return;
        }
        if (action.type === "delete") {
          setDeleteFile(file);
          return;
        }
        if (action.type === "properties") {
          setPropsFile(file);
        }
      } catch (err: unknown) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [],
  );

  const folderTree = useMemo(() => buildFolderTree(roots, files), [roots, files]);
  const selectedFolder = useMemo(() => {
    return findFolderNode(folderTree, folderKeyState) ?? folderTree;
  }, [folderTree, folderKeyState]);
  const folderFiles = useMemo(
    () => filesInFolder(files, selectedFolder),
    [files, selectedFolder],
  );
  const visibleFiles = useMemo(
    () => sortFiles(filterFiles(folderFiles, filter), fileSort),
    [folderFiles, filter, fileSort],
  );
  const visibleIds = useMemo(() => visibleFiles.map((row) => row.id), [visibleFiles]);

  useEffect(() => {
    if (folderKeyState === ALL_FOLDER_KEY) {
      return;
    }
    if (files.length === 0 && roots.length === 0) {
      return;
    }
    if (!findFolderNode(folderTree, folderKeyState)) {
      setFolderKeyState(ALL_FOLDER_KEY);
      localStorage.setItem(TREE_FOLDER_KEY, ALL_FOLDER_KEY);
    }
  }, [folderTree, folderKeyState, files.length, roots.length]);

  useEffect(() => {
    setExpanded((prev) => {
      let changed = false;
      const next = new Set(prev);
      if (!next.has(ALL_FOLDER_KEY)) {
        next.add(ALL_FOLDER_KEY);
        changed = true;
      }
      for (const root of roots) {
        const key = folderKey(root.id, "");
        if (!next.has(key) && !collapsedByUser.current.has(key)) {
          next.add(key);
          changed = true;
        }
      }
      return changed ? next : prev;
    });
  }, [roots]);

  const selectFolder = useCallback(
    (node: { key: string }) => {
      if (node.key !== folderKeyState) {
        setSelection(emptySelection);
      }
      setFolderKeyState(node.key);
      localStorage.setItem(TREE_FOLDER_KEY, node.key);
      setExpanded((prev) => {
        const next = new Set(prev);
        for (const key of ancestorKeys(folderTree, node.key)) {
          next.add(key);
        }
        next.add(node.key);
        return next;
      });
    },
    [folderTree, folderKeyState],
  );

  const toggleFolder = useCallback((key: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
        collapsedByUser.current.add(key);
      } else {
        next.add(key);
        collapsedByUser.current.delete(key);
      }
      return next;
    });
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (renameFile || deleteFile || propsFile || menu) {
        return;
      }
      const target = event.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) {
        return;
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "a") {
        event.preventDefault();
        setSelection((prev) => selectAll(visibleIds, prev));
        return;
      }
      if (event.key === "Escape") {
        setSelection(emptySelection);
        return;
      }
      const file = files.find((row) => row.id === selection.focusedId);
      if (!file) {
        return;
      }
      if (event.key === "F2") {
        event.preventDefault();
        setRenameFile(file);
      }
      if (event.key === "Delete") {
        event.preventDefault();
        setDeleteFile(file);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [selection.focusedId, files, visibleIds, renameFile, deleteFile, propsFile, menu]);

  const handlePlateThumb = useCallback((fileId: number, thumbPath: string | null) => {
    setThumbOverrides((prev) => {
      if (!thumbPath) {
        const next = { ...prev };
        delete next[fileId];
        return next;
      }
      return { ...prev, [fileId]: thumbPath };
    });
  }, []);

  const selectedFiles = useMemo(() => {
    const map = new Map(files.map((file) => [file.id, file]));
    return selection.selectedIds
      .map((id) => map.get(id))
      .filter((file): file is FileRow => Boolean(file));
  }, [files, selection.selectedIds]);
  const selected =
    files.find((file) => file.id === selection.focusedId) ?? selectedFiles[0] ?? null;

  async function addFolder() {
    setError(null);
    const selectedFolder = await open({
      directory: true,
      multiple: false,
      title: "Add a library folder",
    });
    if (!selectedFolder || Array.isArray(selectedFolder)) {
      return;
    }
    setBusy(true);
    try {
      const next = await invoke<LibraryRoot[]>("add_library_folder", {
        path: selectedFolder,
      });
      setRoots(next);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function runRescan() {
    setError(null);
    try {
      await invoke("rescan");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function togglePause() {
    if (progress.paused) {
      await invoke("resume_scan");
    } else {
      await invoke("pause_scan");
    }
  }

  const patchFiles = useCallback((rows: FileRow[]) => {
    setFiles((prev) => {
      const map = new Map(rows.map((row) => [row.id, row]));
      return prev.map((file) => map.get(file.id) ?? file);
    });
    void invoke<string[]>("list_tags").then(setAllTags);
  }, []);

  const patchFile = useCallback(
    (row: FileRow) => {
      patchFiles([row]);
    },
    [patchFiles],
  );

  useEffect(() => {
    const live = new Set(files.map((file) => file.id));
    setSelection((prev) => {
      const selectedIds = prev.selectedIds.filter((id) => live.has(id));
      const focusedId =
        prev.focusedId && live.has(prev.focusedId)
          ? prev.focusedId
          : (selectedIds[selectedIds.length - 1] ?? null);
      const anchorId =
        prev.anchorId && live.has(prev.anchorId) ? prev.anchorId : (selectedIds[0] ?? null);
      if (
        selectedIds.length === prev.selectedIds.length &&
        focusedId === prev.focusedId &&
        anchorId === prev.anchorId
      ) {
        return prev;
      }
      return { selectedIds, focusedId, anchorId };
    });
  }, [files]);

  const emptyLibrary = roots.length === 0;

  return (
    <main className="flex h-screen flex-col bg-md-bg text-md-text">
      <header className="flex items-center justify-between gap-4 border-b border-md-border px-4 py-3">
        <div className="min-w-0">
          <h1 className="text-lg font-semibold tracking-tight">MeshDrawer</h1>
          <p className="truncate text-sm text-md-muted">
            {files.length} model{files.length === 1 ? "" : "s"}
            {selection.selectedIds.length > 1
              ? ` · ${selection.selectedIds.length} selected`
              : ""}
            {roots.length > 0 ? ` · ${roots.length} folder${roots.length === 1 ? "" : "s"}` : ""}
            {dupeGroups.length > 0
              ? ` · ${dupeGroups.length} duplicate group${dupeGroups.length === 1 ? "" : "s"}`
              : ""}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {progress.running ? (
            <button
              type="button"
              onClick={() => void togglePause()}
              className="rounded-md border border-md-border px-3 py-2 text-sm hover:bg-md-border"
            >
              {progress.paused ? "Resume" : "Pause"}
            </button>
          ) : null}
          <button
            type="button"
            onClick={() => setView(view === "settings" ? "gallery" : "settings")}
            className={`rounded-md border px-3 py-2 text-sm hover:bg-md-border ${
              view === "settings" ? "border-md-cyan text-md-cyan" : "border-md-border"
            }`}
          >
            Settings
          </button>
          <button
            type="button"
            onClick={() => setView(view === "dupes" ? "gallery" : "dupes")}
            className={`rounded-md border px-3 py-2 text-sm hover:bg-md-border ${
              view === "dupes" ? "border-md-cyan text-md-cyan" : "border-md-border"
            }`}
          >
            Duplicates{dupeGroups.length > 0 ? ` (${dupeGroups.length})` : ""}
          </button>
          <button
            type="button"
            onClick={() => void runRescan()}
            disabled={progress.running || roots.length === 0}
            className="rounded-md border border-md-border px-3 py-2 text-sm hover:bg-md-border disabled:opacity-40"
          >
            Rescan
          </button>
          <button
            type="button"
            onClick={() => void addFolder()}
            disabled={busy}
            className="rounded-md bg-md-cyan px-3 py-2 text-sm font-medium text-md-bg hover:bg-md-magenta hover:text-md-text disabled:opacity-50"
          >
            Add a library folder
          </button>
        </div>
      </header>

      {progress.running ? (
        <div className="border-b border-md-border bg-md-surface px-4 py-2 text-xs text-md-muted">
          <div className="flex items-center justify-between gap-4">
            <span>
              {progress.paused ? "Paused" : "Scanning"} · {progress.indexed} indexed · {progress.seen}{" "}
              files seen
              {progress.errors > 0 ? ` · ${progress.errors} errors` : ""}
            </span>
            <span className="max-w-[50%] truncate font-mono text-[11px] text-md-muted">
              {progress.currentPath}
            </span>
          </div>
          <div className="mt-2 h-1 overflow-hidden rounded bg-md-border">
            <div className="h-full w-1/3 animate-pulse bg-md-cyan" />
          </div>
        </div>
      ) : null}

      {hashProgress.running ? (
        <div className="border-b border-md-border bg-md-surface px-4 py-1.5 text-xs text-md-muted">
          Hashing exact copies · {hashProgress.hashed} done · {hashProgress.remaining} queued
          <span className="ml-3 max-w-[40%] truncate font-mono text-[11px] text-md-muted">
            {hashProgress.currentPath}
          </span>
        </div>
      ) : null}

      {scanErrors.length > 0 ? (
        <div className="border-b border-amber-900/60 bg-amber-950/20 px-4 py-2 text-xs text-amber-200">
          <button type="button" onClick={() => setShowErrors((v) => !v)} className="font-medium">
            {scanErrors.length} unreadable or locked file{scanErrors.length === 1 ? "" : "s"}{" "}
            {showErrors ? "▾" : "▸"}
          </button>
          {showErrors ? (
            <ul className="mt-2 max-h-32 space-y-1 overflow-auto font-mono text-[11px] text-amber-100/80">
              {scanErrors.slice(0, 50).map((row) => (
                <li key={`${row.path}-${row.seenAt}`} className="truncate" title={row.path}>
                  {row.message} — {row.path}
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      ) : null}

      {error ? (
        <p className="border-b border-red-900 bg-red-950/40 px-4 py-2 text-sm text-red-300">{error}</p>
      ) : null}

      {emptyLibrary ? (
        <section className="flex flex-1 flex-col items-center justify-center px-6">
          <p className="text-xl font-medium">Add a library folder</p>
          <p className="mt-3 max-w-md text-center text-sm leading-6 text-md-muted">
            Point MeshDrawer at folders where your STL, 3MF, OBJ, STEP, FBX, and 3DM files already live.
            Nothing is uploaded or moved.
          </p>
          <button
            type="button"
            onClick={() => void addFolder()}
            disabled={busy}
            className="mt-8 rounded-md bg-md-cyan px-4 py-2.5 text-sm font-medium text-md-bg hover:bg-md-magenta hover:text-md-text disabled:opacity-50"
          >
            {busy ? "Adding…" : "Add a library folder"}
          </button>
        </section>
      ) : view === "settings" && settings ? (
        <SettingsPanel
          settings={settings}
          roots={roots}
          onChange={setSettings}
          onSave={() => {
            void invoke<AppSettings>("save_app_settings", { settings })
              .then((saved) => {
                setSettings(saved);
                void invoke<SlicerApp[]>("list_slicers").then(setSlicers);
              })
              .catch((err: unknown) => {
                setError(err instanceof Error ? err.message : String(err));
              });
          }}
          onAddRoot={() => void addFolder()}
          onRemoveRoot={(id) => {
            void invoke<LibraryRoot[]>("remove_library_folder", { id })
              .then((next) => {
                setRoots(next);
                void refresh();
              })
              .catch((err: unknown) => {
                setError(err instanceof Error ? err.message : String(err));
              });
          }}
          onError={setError}
        />
      ) : view === "dupes" ? (
        <DuplicateView
          groups={dupeGroups}
          thumbsDir={info?.thumbsDir ?? null}
          hashing={hashProgress.running}
          onReveal={(id) => {
            void invoke("reveal_in_explorer", { id }).catch((err: unknown) => {
              setError(err instanceof Error ? err.message : String(err));
            });
          }}
          onSelect={(id) => {
            setSelection({ selectedIds: [id], anchorId: id, focusedId: id });
            setView("gallery");
          }}
          onQuarantine={(id) => {
            void invoke("quarantine_file", { id })
              .then(() => {
                setFiles((prev) => prev.filter((f) => f.id !== id));
                setDupeGroups((prev) =>
                  prev
                    .map((g) => ({ ...g, files: g.files.filter((f) => f.id !== id) }))
                    .filter((g) => g.files.length >= 2),
                );
              })
              .catch((err: unknown) => {
                setError(err instanceof Error ? err.message : String(err));
              });
          }}
          onIgnore={(sha256) => {
            void invoke("ignore_duplicate_group", { sha256 })
              .then(() => {
                setDupeGroups((prev) => prev.filter((g) => g.sha256 !== sha256));
              })
              .catch((err: unknown) => {
                setError(err instanceof Error ? err.message : String(err));
              });
          }}
        />
      ) : (
        <>
        <FilterBar
          filter={filter}
          tags={allTags}
          resultCount={visibleFiles.length}
          totalCount={folderFiles.length}
          sort={fileSort}
          view={galleryView}
          onChange={setFilter}
          onSort={(next) => {
            setFileSort(next);
            localStorage.setItem(SORT_KEY, next);
          }}
          onView={(next) => {
            setGalleryView(next);
            localStorage.setItem(VIEW_MODE_KEY, next);
          }}
        />
        <div className="flex min-h-0 min-w-0 flex-1">
          <div
            className="flex h-full shrink-0 flex-col"
            style={{ width: clamp(treeWidth, 160, 420) }}
          >
            <FolderTree
              tree={folderTree}
              selectedKey={selectedFolder.key}
              expanded={expanded}
              onSelect={selectFolder}
              onToggle={toggleFolder}
            />
          </div>
          <SplitHandle side="start" onDrag={(size) => resizeTree(size)} />
          <Gallery
            files={visibleFiles}
            thumbsDir={info?.thumbsDir ?? null}
            selectedIds={selection.selectedIds}
            focusedId={selection.focusedId}
            thumbOverrides={thumbOverrides}
            view={galleryView}
            sort={fileSort}
            onSort={(next) => {
              setFileSort(next);
              localStorage.setItem(SORT_KEY, next);
            }}
            emptyMessage={
              folderFiles.length === 0
                ? selectedFolder.key === ALL_FOLDER_KEY
                  ? "No models in the library folders."
                  : `No models in ${selectedFolder.label}.`
                : "No models match the current filters."
            }
            onSelect={(file, event) => {
              const gesture = gestureFromModifiers(
                event.shiftKey,
                event.ctrlKey || event.metaKey,
              );
              setSelection((prev) => applySelection(visibleIds, prev, file.id, gesture));
            }}
            onOpen={(file) => {
              setSelection((prev) => applySelection(visibleIds, prev, file.id, "keep"));
              void runFileAction(file, { type: "open", slicerId: preferredSlicer });
            }}
            onContextMenu={(file, x, y) => {
              setSelection((prev) => applySelection(visibleIds, prev, file.id, "keep"));
              setMenu({ file, x, y });
            }}
          />
          {selected ? (
            <>
              <SplitHandle
                onDrag={(widthFromRight) => resizeDetail(widthFromRight)}
              />
              <div
                className="flex h-full shrink-0 flex-col border-l border-md-border"
                style={{ width: clamp(detailWidth, 240, 900) }}
              >
                <DetailPane
                  file={selected}
                  selectedFiles={selectedFiles}
                  thumbsDir={info?.thumbsDir ?? null}
                  slicers={slicers}
                  onClose={() => setSelection(emptySelection)}
                  onError={setError}
                  onPlateThumb={handlePlateThumb}
                  onFileChange={patchFile}
                  onFilesChange={patchFiles}
                />
              </div>
            </>
          ) : null}
        </div>
        </>
      )}

      {menu ? (
        <FileContextMenu
          file={menu.file}
          x={menu.x}
          y={menu.y}
          slicers={slicers}
          onClose={() => setMenu(null)}
          onAction={(action) => {
            const file = menu.file;
            setMenu(null);
            void runFileAction(file, action);
          }}
        />
      ) : null}

      {renameFile ? (
        <RenameDialog
          file={renameFile}
          onCancel={() => setRenameFile(null)}
          onSubmit={(name) => {
            const id = renameFile.id;
            setRenameFile(null);
            void invoke<FileRow>("rename_library_file", { id, newName: name })
              .then((row) => {
                setFiles((prev) => prev.map((f) => (f.id === row.id ? row : f)));
              })
              .catch((err: unknown) => {
                setError(err instanceof Error ? err.message : String(err));
              });
          }}
        />
      ) : null}

      {deleteFile ? (
        <DeleteDialog
          file={deleteFile}
          onCancel={() => setDeleteFile(null)}
          onConfirm={() => {
            const id = deleteFile.id;
            setDeleteFile(null);
            void invoke("delete_library_file", { id })
              .then(() => {
                setFiles((prev) => prev.filter((f) => f.id !== id));
                setSelection((current) => {
                  const selectedIds = current.selectedIds.filter((rowId) => rowId !== id);
                  return {
                    selectedIds,
                    focusedId:
                      current.focusedId === id ? (selectedIds[0] ?? null) : current.focusedId,
                    anchorId:
                      current.anchorId === id ? (selectedIds[0] ?? null) : current.anchorId,
                  };
                });
                setPropsFile((current) => (current?.id === id ? null : current));
              })
              .catch((err: unknown) => {
                setError(err instanceof Error ? err.message : String(err));
              });
          }}
        />
      ) : null}

      {propsFile ? (
        <PropertiesDialog
          file={files.find((f) => f.id === propsFile.id) ?? propsFile}
          onClose={() => setPropsFile(null)}
          onExplorer={() => {
            void runFileAction(propsFile, { type: "reveal" });
          }}
          onWindowsProps={() => {
            void invoke("windows_properties", { id: propsFile.id }).catch((err: unknown) => {
              setError(err instanceof Error ? err.message : String(err));
            });
          }}
        />
      ) : null}

      {info ? (
        <footer className="border-t border-md-border px-4 py-2 text-xs text-md-muted">
          {info.dataDir}
        </footer>
      ) : null}
    </main>
  );
}

export default App;
