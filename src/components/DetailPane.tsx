import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { formatBytes } from "../lib/format";
import { extLabel, fileNameFromRel, isCadPreviewExt } from "../lib/files";
import { thumbUrl } from "../lib/thumbUrl";
import type { FileRow, PlateRow, SlicerApp } from "../types";
import { ModelViewer } from "./ModelViewer";

type Props = {
  file: FileRow;
  thumbsDir: string | null;
  slicers: SlicerApp[];
  onClose: () => void;
  onError: (message: string) => void;
  onPlateThumb: (fileId: number, thumbPath: string | null) => void;
  onFileChange: (file: FileRow) => void;
};

export function DetailPane({
  file,
  thumbsDir,
  slicers,
  onClose,
  onError,
  onPlateThumb,
  onFileChange,
}: Props) {
  const [plates, setPlates] = useState<PlateRow[]>([]);
  const [plateIndex, setPlateIndex] = useState(1);
  const [tagDraft, setTagDraft] = useState("");
  const [notes, setNotes] = useState(file.notes ?? "");
  const is3mf = file.ext === "3mf";
  const showStlViewer =
    file.ext === "stl" || file.ext === "obj" || isCadPreviewExt(file.ext);
  const is3dm = file.ext === "3dm";

  useEffect(() => {
    setNotes(file.notes ?? "");
    setTagDraft("");
    setPlateIndex(1);
    setPlates([]);
    if (!is3mf) {
      return;
    }
    invoke<PlateRow[]>("list_plates", { fileId: file.id })
      .then((rows) => {
        setPlates(rows);
        const first = rows[0]?.plateIndex ?? 1;
        setPlateIndex(first);
        onPlateThumb(file.id, rows[0]?.thumbPath ?? file.thumbPath);
      })
      .catch((err: unknown) => {
        onError(err instanceof Error ? err.message : String(err));
      });
  }, [file.id, file.ext, file.thumbPath, is3mf, onError, onPlateThumb]);

  const selectedPlate = plates.find((p) => p.plateIndex === plateIndex);
  const plateThumbName = selectedPlate?.thumbPath ?? file.thumbPath;
  const thumb =
    plateThumbName && thumbsDir ? thumbUrl(thumbsDir, plateThumbName) : null;

  function selectPlate(plate: PlateRow) {
    setPlateIndex(plate.plateIndex);
    onPlateThumb(file.id, plate.thumbPath);
  }

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
    <aside className="flex h-full min-w-0 flex-1 flex-col bg-md-surface/40">
      <div className="flex items-center justify-between border-b border-md-border px-4 py-3">
        <h2 className="truncate text-sm font-medium">{fileNameFromRel(file.relPath)}</h2>
        <button
          type="button"
          onClick={onClose}
          className="text-xs text-md-muted hover:text-md-text"
        >
          Close
        </button>
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-4">
        {is3mf && thumb ? (
          <img
            src={thumb}
            alt=""
            className="mb-3 h-40 w-full rounded-md bg-md-bg object-contain"
          />
        ) : null}

        {showStlViewer ? (
          <div className="min-h-64">
            <ModelViewer file={file} />
          </div>
        ) : null}
        {is3dm ? (
          <p className="mt-2 text-xs text-md-muted">
            Rhino .3dm is indexed, but mesh preview is not available yet. Use a sidecar
            image if you have one, or Open with the default app.
          </p>
        ) : null}
        {is3mf ? (
          <div className="min-h-64">
            <ModelViewer file={file} plateIndex={plateIndex} />
          </div>
        ) : null}
        {!showStlViewer && !is3mf && !thumb ? (
          <div className="flex h-64 items-center justify-center rounded-md bg-md-bg text-sm text-md-muted">
            No preview yet
          </div>
        ) : null}

        {plates.length > 1 ? (
          <div className="mt-3">
            <p className="mb-2 text-xs uppercase tracking-wide text-md-muted">Plates</p>
            <div className="flex gap-2 overflow-x-auto pb-1">
              {plates.map((plate) => {
                const src =
                  plate.thumbPath && thumbsDir
                    ? thumbUrl(thumbsDir, plate.thumbPath)
                    : null;
                const active = plate.plateIndex === plateIndex;
                return (
                  <button
                    key={plate.plateIndex}
                    type="button"
                    onClick={() => selectPlate(plate)}
                    className={`w-20 shrink-0 overflow-hidden rounded border ${
                      active ? "border-md-magenta shadow-[0_0_12px_rgba(255,43,214,0.35)]" : "border-md-border"
                    }`}
                    title={plate.name || `Plate ${plate.plateIndex}`}
                  >
                    {src ? (
                      <img src={src} alt="" className="h-14 w-full object-cover" />
                    ) : (
                      <div className="flex h-14 items-center justify-center bg-md-bg text-[10px] text-md-muted">
                        {plate.plateIndex}
                      </div>
                    )}
                    <span className="block truncate px-1 py-0.5 text-[10px] text-md-text">
                      {plate.name || `Plate ${plate.plateIndex}`}
                    </span>
                  </button>
                );
              })}
            </div>
          </div>
        ) : null}

        <dl className="mt-4 space-y-1 text-xs text-md-muted">
          <div className="flex justify-between gap-3">
            <dt>Format</dt>
            <dd className="text-md-text">{extLabel(file.ext)}</dd>
          </div>
          <div className="flex justify-between gap-3">
            <dt>Size</dt>
            <dd className="text-md-text">{formatBytes(file.size)}</dd>
          </div>
          {file.plateCount > 0 ? (
            <div className="flex justify-between gap-3">
              <dt>Plates</dt>
              <dd className="text-md-text">{file.plateCount}</dd>
            </div>
          ) : null}
        </dl>
        <p className="mt-3 break-all font-mono text-[11px] text-md-muted">{file.absPath}</p>

        <label className="mt-4 block text-xs uppercase tracking-wide text-md-muted">
          Status
          <select
            value={file.status}
            onChange={(event) => {
              void invoke<FileRow>("set_file_status", {
                id: file.id,
                status: event.target.value,
              })
                .then(onFileChange)
                .catch((err: unknown) => {
                  onError(err instanceof Error ? err.message : String(err));
                });
            }}
            className="mt-1 w-full rounded-md border border-md-border bg-md-bg px-2 py-1.5 text-sm text-md-text"
          >
            <option value="unset">Unset</option>
            <option value="unprinted">Unprinted</option>
            <option value="printed">Printed</option>
            <option value="favorite">Favorite</option>
          </select>
        </label>

        <div className="mt-4">
          <p className="text-xs uppercase tracking-wide text-md-muted">Tags</p>
          <div className="mt-1 flex flex-wrap gap-1">
            {file.tags.map((tag) => (
              <button
                key={tag}
                type="button"
                onClick={() => {
                  void invoke<FileRow>("remove_file_tag", { id: file.id, name: tag })
                    .then(onFileChange)
                    .catch((err: unknown) => {
                      onError(err instanceof Error ? err.message : String(err));
                    });
                }}
                className="rounded-full bg-md-violet/40 px-2 py-0.5 text-xs text-md-cyan hover:bg-md-magenta/40"
                title="Remove tag"
              >
                {tag} ×
              </button>
            ))}
          </div>
          <form
            className="mt-2 flex gap-2"
            onSubmit={(event) => {
              event.preventDefault();
              const name = tagDraft.trim();
              if (!name) {
                return;
              }
              void invoke<FileRow>("add_file_tag", { id: file.id, name })
                .then((row) => {
                  setTagDraft("");
                  onFileChange(row);
                })
                .catch((err: unknown) => {
                  onError(err instanceof Error ? err.message : String(err));
                });
            }}
          >
            <input
              value={tagDraft}
              onChange={(event) => setTagDraft(event.target.value)}
              placeholder="Add tag"
              className="min-w-0 flex-1 rounded-md border border-md-border bg-md-bg px-2 py-1 text-sm"
            />
            <button type="submit" className="rounded-md border border-md-border px-2 text-xs">
              Add
            </button>
          </form>
        </div>

        <label className="mt-4 block text-xs uppercase tracking-wide text-md-muted">
          Notes
          <textarea
            value={notes}
            onChange={(event) => setNotes(event.target.value)}
            onBlur={() => {
              if ((file.notes ?? "") === notes) {
                return;
              }
              void invoke<FileRow>("set_file_notes", { id: file.id, notes })
                .then(onFileChange)
                .catch((err: unknown) => {
                  onError(err instanceof Error ? err.message : String(err));
                });
            }}
            rows={4}
            className="mt-1 w-full rounded-md border border-md-border bg-md-bg px-2 py-1.5 text-sm text-md-text"
          />
        </label>
      </div>

      <div className="space-y-2 border-t border-md-border p-4">
        {slicers
          .filter((s) => s.available)
          .map((slicer) => (
            <button
              key={slicer.id}
              type="button"
              onClick={() => void openSlicer(slicer.id)}
              className="w-full rounded-md bg-md-cyan px-3 py-2 text-sm font-medium text-md-bg hover:bg-md-magenta hover:text-md-text"
            >
              Open with {slicer.name}
            </button>
          ))}
        <button
          type="button"
          onClick={() => void reveal()}
          className="w-full rounded-md border border-md-border px-3 py-2 text-sm hover:bg-md-border"
        >
          Reveal in Explorer
        </button>
      </div>
    </aside>
  );
}
