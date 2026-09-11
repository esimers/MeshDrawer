import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";

type LibraryRoot = {
  id: number;
  path: string;
  addedAt: number;
};

type AppInfo = {
  dbPath: string;
  thumbsDir: string;
  dataDir: string;
};

function App() {
  const [roots, setRoots] = useState<LibraryRoot[]>([]);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    const [nextRoots, nextInfo] = await Promise.all([
      invoke<LibraryRoot[]>("list_roots"),
      invoke<AppInfo>("get_app_info"),
    ]);
    setRoots(nextRoots);
    setInfo(nextInfo);
  }, []);

  useEffect(() => {
    refresh().catch((err: unknown) => {
      setError(err instanceof Error ? err.message : String(err));
    });
  }, [refresh]);

  async function addFolder() {
    setError(null);
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Add a library folder",
    });
    if (!selected || Array.isArray(selected)) {
      return;
    }
    setBusy(true);
    try {
      const next = await invoke<LibraryRoot[]>("add_library_folder", {
        path: selected,
      });
      setRoots(next);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="flex min-h-screen flex-col bg-zinc-950 text-zinc-100">
      <header className="flex items-center justify-between border-b border-zinc-800 px-6 py-4">
        <div>
          <h1 className="text-lg font-semibold tracking-tight">MeshDrawer</h1>
          <p className="text-sm text-zinc-400">Local 3D print library</p>
        </div>
        <button
          type="button"
          onClick={() => void addFolder()}
          disabled={busy}
          className="rounded-md bg-teal-600 px-3 py-2 text-sm font-medium text-white hover:bg-teal-500 disabled:opacity-50"
        >
          Add a library folder
        </button>
      </header>

      <section className="flex flex-1 flex-col items-center justify-center px-6 py-16">
        {error ? (
          <p className="mb-6 max-w-lg text-center text-sm text-red-400">{error}</p>
        ) : null}

        {roots.length === 0 ? (
          <div className="max-w-md text-center">
            <p className="text-xl font-medium">Add a library folder</p>
            <p className="mt-3 text-sm leading-6 text-zinc-400">
              Point MeshDrawer at folders where your STL, 3MF, and OBJ files already
              live. Nothing is uploaded or moved.
            </p>
            <button
              type="button"
              onClick={() => void addFolder()}
              disabled={busy}
              className="mt-8 rounded-md bg-teal-600 px-4 py-2.5 text-sm font-medium text-white hover:bg-teal-500 disabled:opacity-50"
            >
              {busy ? "Adding…" : "Add a library folder"}
            </button>
          </div>
        ) : (
          <div className="w-full max-w-2xl">
            <h2 className="text-sm font-medium uppercase tracking-wide text-zinc-500">
              Library folders
            </h2>
            <ul className="mt-3 divide-y divide-zinc-800 overflow-hidden rounded-lg border border-zinc-800">
              {roots.map((root) => (
                <li key={root.id} className="px-4 py-3 font-mono text-sm text-zinc-200">
                  {root.path}
                </li>
              ))}
            </ul>
            <p className="mt-4 text-sm text-zinc-500">
              Indexing lands in the next phase. Folders are stored; files are not scanned yet.
            </p>
          </div>
        )}
      </section>

      {info ? (
        <footer className="border-t border-zinc-800 px-6 py-3 text-xs text-zinc-500">
          Data: {info.dataDir}
        </footer>
      ) : null}
    </main>
  );
}

export default App;
