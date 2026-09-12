import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { DetailPane } from "./components/DetailPane";
import { Gallery } from "./components/Gallery";
import { needsMeshThumb, renderOneMeshThumb } from "./lib/meshThumbs";
import type {
  AppInfo,
  FileRow,
  LibraryRoot,
  ScanProgress,
  SlicerApp,
  ThumbReady,
} from "./types";
import "./App.css";

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
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [thumbOverrides, setThumbOverrides] = useState<Record<number, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [thumbTick, setThumbTick] = useState(0);
  const meshBusy = useRef(false);

  const refresh = useCallback(async () => {
    const [nextRoots, nextFiles, nextInfo, nextProgress, nextSlicers] = await Promise.all([
      invoke<LibraryRoot[]>("list_roots"),
      invoke<FileRow[]>("list_files"),
      invoke<AppInfo>("get_app_info"),
      invoke<ScanProgress>("get_scan_status"),
      invoke<SlicerApp[]>("list_slicers"),
    ]);
    setRoots(nextRoots);
    setFiles(nextFiles);
    setInfo(nextInfo);
    setProgress(nextProgress);
    setSlicers(nextSlicers);
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

  const selected = files.find((file) => file.id === selectedId) ?? null;

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

  const emptyLibrary = roots.length === 0;

  return (
    <main className="flex h-screen flex-col bg-zinc-950 text-zinc-100">
      <header className="flex items-center justify-between gap-4 border-b border-zinc-800 px-4 py-3">
        <div className="min-w-0">
          <h1 className="text-lg font-semibold tracking-tight">MeshDrawer</h1>
          <p className="truncate text-sm text-zinc-400">
            {files.length} model{files.length === 1 ? "" : "s"}
            {roots.length > 0 ? ` · ${roots.length} folder${roots.length === 1 ? "" : "s"}` : ""}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {progress.running ? (
            <button
              type="button"
              onClick={() => void togglePause()}
              className="rounded-md border border-zinc-700 px-3 py-2 text-sm hover:bg-zinc-800"
            >
              {progress.paused ? "Resume" : "Pause"}
            </button>
          ) : null}
          <button
            type="button"
            onClick={() => void runRescan()}
            disabled={progress.running || roots.length === 0}
            className="rounded-md border border-zinc-700 px-3 py-2 text-sm hover:bg-zinc-800 disabled:opacity-40"
          >
            Rescan
          </button>
          <button
            type="button"
            onClick={() => void addFolder()}
            disabled={busy}
            className="rounded-md bg-teal-600 px-3 py-2 text-sm font-medium text-white hover:bg-teal-500 disabled:opacity-50"
          >
            Add a library folder
          </button>
        </div>
      </header>

      {progress.running ? (
        <div className="border-b border-zinc-800 bg-zinc-900 px-4 py-2 text-xs text-zinc-400">
          <div className="flex items-center justify-between gap-4">
            <span>
              {progress.paused ? "Paused" : "Scanning"} · {progress.indexed} indexed · {progress.seen}{" "}
              files seen
              {progress.errors > 0 ? ` · ${progress.errors} errors` : ""}
            </span>
            <span className="max-w-[50%] truncate font-mono text-[11px] text-zinc-500">
              {progress.currentPath}
            </span>
          </div>
          <div className="mt-2 h-1 overflow-hidden rounded bg-zinc-800">
            <div className="h-full w-1/3 animate-pulse bg-teal-500" />
          </div>
        </div>
      ) : null}

      {error ? (
        <p className="border-b border-red-900 bg-red-950/40 px-4 py-2 text-sm text-red-300">{error}</p>
      ) : null}

      {emptyLibrary ? (
        <section className="flex flex-1 flex-col items-center justify-center px-6">
          <p className="text-xl font-medium">Add a library folder</p>
          <p className="mt-3 max-w-md text-center text-sm leading-6 text-zinc-400">
            Point MeshDrawer at folders where your STL, 3MF, and OBJ files already live.
            Nothing is uploaded or moved.
          </p>
          <button
            type="button"
            onClick={() => void addFolder()}
            disabled={busy}
            className="mt-8 rounded-md bg-teal-600 px-4 py-2.5 text-sm font-medium text-white hover:bg-teal-500 disabled:opacity-50"
          >
            {busy ? "Adding…" : "Add a library folder"}
          </button>
        </section>
      ) : (
        <div className="flex min-h-0 flex-1">
          <Gallery
            files={files}
            thumbsDir={info?.thumbsDir ?? null}
            selectedId={selectedId}
            thumbOverrides={thumbOverrides}
            onSelect={(file) => setSelectedId(file.id)}
          />
          {selected ? (
            <DetailPane
              file={selected}
              thumbsDir={info?.thumbsDir ?? null}
              slicers={slicers}
              onClose={() => setSelectedId(null)}
              onError={setError}
              onPlateThumb={handlePlateThumb}
            />
          ) : null}
        </div>
      )}

      {info ? (
        <footer className="border-t border-zinc-800 px-4 py-2 text-xs text-zinc-500">
          {info.dataDir}
        </footer>
      ) : null}
    </main>
  );
}

export default App;
