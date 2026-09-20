import { open } from "@tauri-apps/plugin-dialog";
import type { AppSettings, LibraryRoot } from "../types";

type Props = {
  settings: AppSettings;
  roots: LibraryRoot[];
  onChange: (next: AppSettings) => void;
  onSave: () => void;
  onRemoveRoot: (id: number) => void;
  onAddRoot: () => void;
  onError: (message: string) => void;
};

export function SettingsPanel({
  settings,
  roots,
  onChange,
  onSave,
  onRemoveRoot,
  onAddRoot,
  onError,
}: Props) {
  async function pick(kind: "quarantine" | "bambu" | "orca") {
    try {
      if (kind === "quarantine") {
        const dir = await open({ directory: true, multiple: false, title: "Quarantine folder" });
        if (typeof dir === "string") {
          onChange({ ...settings, quarantineDir: dir });
        }
        return;
      }
      const file = await open({
        multiple: false,
        title: kind === "bambu" ? "Bambu Studio" : "Orca Slicer",
        filters: [{ name: "Executable", extensions: ["exe"] }],
      });
      if (typeof file === "string") {
        if (kind === "bambu") {
          onChange({ ...settings, bambuPath: file });
        } else {
          onChange({ ...settings, orcaPath: file });
        }
      }
    } catch (err: unknown) {
      onError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div className="min-h-0 flex-1 overflow-auto px-6 py-5">
      <div className="mx-auto max-w-2xl space-y-8">
        <section>
          <h2 className="text-sm font-semibold">Library folders</h2>
          <p className="mt-1 text-xs text-md-muted">
            Tags, notes, print status, and ignored duplicates are saved in a{" "}
            <span className="font-mono">.meshdrawer</span> folder next to your models. After a
            reinstall, add the same library folder and MeshDrawer restores them. Thumbnails rebuild
            on their own.
          </p>
          <ul className="mt-2 divide-y divide-md-border overflow-hidden rounded-lg border border-md-border">
            {roots.map((root) => (
              <li key={root.id} className="flex items-center justify-between gap-3 px-3 py-2">
                <span className="truncate font-mono text-xs">{root.path}</span>
                <button
                  type="button"
                  onClick={() => onRemoveRoot(root.id)}
                  className="text-xs text-red-300 hover:text-red-200"
                >
                  Remove
                </button>
              </li>
            ))}
          </ul>
          <button
            type="button"
            onClick={onAddRoot}
            className="mt-2 rounded-md border border-md-border px-3 py-1.5 text-sm hover:bg-md-border"
          >
            Add folder
          </button>
        </section>

        <section>
          <h2 className="text-sm font-semibold">Slicers</h2>
          <p className="mt-1 text-xs text-md-muted">
            Leave blank to auto-detect Bambu Studio and Orca Slicer.
          </p>
          <label className="mt-3 block text-xs text-md-muted">
            Bambu Studio
            <span className="mt-1 flex gap-2">
              <input
                value={settings.bambuPath}
                onChange={(event) => onChange({ ...settings, bambuPath: event.target.value })}
                className="min-w-0 flex-1 rounded-md border border-md-border bg-md-bg px-2 py-1.5 font-mono text-xs"
              />
              <button type="button" onClick={() => void pick("bambu")} className="rounded-md border border-md-border px-2 text-xs">
                Browse
              </button>
            </span>
          </label>
          <label className="mt-3 block text-xs text-md-muted">
            Orca Slicer
            <span className="mt-1 flex gap-2">
              <input
                value={settings.orcaPath}
                onChange={(event) => onChange({ ...settings, orcaPath: event.target.value })}
                className="min-w-0 flex-1 rounded-md border border-md-border bg-md-bg px-2 py-1.5 font-mono text-xs"
              />
              <button type="button" onClick={() => void pick("orca")} className="rounded-md border border-md-border px-2 text-xs">
                Browse
              </button>
            </span>
          </label>
        </section>

        <section>
          <h2 className="text-sm font-semibold">Quarantine folder</h2>
          <span className="mt-2 flex gap-2">
            <input
              value={settings.quarantineDir}
              onChange={(event) => onChange({ ...settings, quarantineDir: event.target.value })}
              className="min-w-0 flex-1 rounded-md border border-md-border bg-md-bg px-2 py-1.5 font-mono text-xs"
            />
            <button type="button" onClick={() => void pick("quarantine")} className="rounded-md border border-md-border px-2 text-xs">
              Browse
            </button>
          </span>
        </section>

        <section>
          <h2 className="text-sm font-semibold">Exclude patterns</h2>
          <p className="mt-1 text-xs text-md-muted">One per line. `*` wildcards and path fragments are OK.</p>
          <textarea
            value={settings.excludePatterns}
            onChange={(event) => onChange({ ...settings, excludePatterns: event.target.value })}
            rows={5}
            className="mt-2 w-full rounded-md border border-md-border bg-md-bg px-2 py-1.5 font-mono text-xs"
            placeholder={"old\n*backup*\nDownloads"}
          />
        </section>

        <button
          type="button"
          onClick={onSave}
          className="rounded-md bg-md-cyan px-4 py-2 text-sm font-medium text-md-bg hover:bg-md-magenta hover:text-md-text"
        >
          Save settings
        </button>
      </div>
    </div>
  );
}
