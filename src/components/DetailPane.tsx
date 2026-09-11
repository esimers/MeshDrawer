import { invoke } from "@tauri-apps/api/core";
import { formatBytes } from "../lib/format";
import { extLabel, fileNameFromRel } from "../lib/files";
import { thumbUrl } from "../lib/thumbUrl";
import type { FileRow, SlicerApp } from "../types";
import { ModelViewer } from "./ModelViewer";

type Props = {
  file: FileRow;
  thumbsDir: string | null;
  slicers: SlicerApp[];
  onClose: () => void;
  onError: (message: string) => void;
};

export function DetailPane({ file, thumbsDir, slicers, onClose, onError }: Props) {
  const thumb =
    file.thumbPath && thumbsDir ? thumbUrl(thumbsDir, file.thumbPath) : null;
  const showViewer = file.ext === "stl" || file.ext === "obj";

  async function reveal() {
    try {
      await invoke("reveal_in_explorer", { id: file.id });
    } catch (err: unknown) {
      onError(err instanceof Error ? err.message : String(err));
    }
  }

  async function openSlicer(slicerId: string) {
    try {
      await invoke("open_in_slicer", { id: file.id, slicerId });
    } catch (err: unknown) {
      onError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <aside className="flex w-[380px] shrink-0 flex-col border-l border-zinc-800 bg-zinc-900/40">
      <div className="flex items-center justify-between border-b border-zinc-800 px-4 py-3">
        <h2 className="truncate text-sm font-medium">{fileNameFromRel(file.relPath)}</h2>
        <button
          type="button"
          onClick={onClose}
          className="text-xs text-zinc-400 hover:text-zinc-100"
        >
          Close
        </button>
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-4">
        {showViewer ? (
          <ModelViewer file={file} />
        ) : thumb ? (
          <img
            src={thumb}
            alt=""
            className="h-64 w-full rounded-md bg-zinc-950 object-contain"
          />
        ) : (
          <div className="flex h-64 items-center justify-center rounded-md bg-zinc-950 text-sm text-zinc-500">
            No preview yet
          </div>
        )}

        {!showViewer && file.ext === "3mf" ? (
          <p className="mt-2 text-xs text-zinc-500">
            Plate mesh preview is next. Embedded plate image is shown when the 3MF
            includes one.
            {file.plateCount > 1 ? ` ${file.plateCount} plates.` : ""}
          </p>
        ) : null}

        <dl className="mt-4 space-y-1 text-xs text-zinc-400">
          <div className="flex justify-between gap-3">
            <dt>Format</dt>
            <dd className="text-zinc-200">{extLabel(file.ext)}</dd>
          </div>
          <div className="flex justify-between gap-3">
            <dt>Size</dt>
            <dd className="text-zinc-200">{formatBytes(file.size)}</dd>
          </div>
          <div className="flex justify-between gap-3">
            <dt>Thumb</dt>
            <dd className="text-zinc-200">{file.thumbStatus}</dd>
          </div>
        </dl>
        <p className="mt-3 break-all font-mono text-[11px] text-zinc-500">{file.absPath}</p>
      </div>

      <div className="space-y-2 border-t border-zinc-800 p-4">
        {slicers
          .filter((s) => s.available)
          .map((slicer) => (
            <button
              key={slicer.id}
              type="button"
              onClick={() => void openSlicer(slicer.id)}
              className="w-full rounded-md bg-teal-600 px-3 py-2 text-sm font-medium text-white hover:bg-teal-500"
            >
              Open with {slicer.name}
            </button>
          ))}
        <button
          type="button"
          onClick={() => void reveal()}
          className="w-full rounded-md border border-zinc-700 px-3 py-2 text-sm hover:bg-zinc-800"
        >
          Reveal in Explorer
        </button>
      </div>
    </aside>
  );
}
