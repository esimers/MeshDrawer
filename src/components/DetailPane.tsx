import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { formatBytes } from "../lib/format";
import { extLabel, fileNameFromRel } from "../lib/files";
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
};

export function DetailPane({
  file,
  thumbsDir,
  slicers,
  onClose,
  onError,
  onPlateThumb,
}: Props) {
  const [plates, setPlates] = useState<PlateRow[]>([]);
  const [plateIndex, setPlateIndex] = useState(1);
  const is3mf = file.ext === "3mf";
  const showStlViewer = file.ext === "stl" || file.ext === "obj";

  useEffect(() => {
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
    <aside className="flex h-full min-w-0 flex-1 flex-col bg-zinc-900/40">
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
        {is3mf && thumb ? (
          <img
            src={thumb}
            alt=""
            className="mb-3 h-40 w-full rounded-md bg-zinc-950 object-contain"
          />
        ) : null}

        {showStlViewer ? (
          <div className="min-h-64">
            <ModelViewer file={file} />
          </div>
        ) : null}
        {is3mf ? (
          <div className="min-h-64">
            <ModelViewer file={file} plateIndex={plateIndex} />
          </div>
        ) : null}
        {!showStlViewer && !is3mf && !thumb ? (
          <div className="flex h-64 items-center justify-center rounded-md bg-zinc-950 text-sm text-zinc-500">
            No preview yet
          </div>
        ) : null}

        {plates.length > 1 ? (
          <div className="mt-3">
            <p className="mb-2 text-xs uppercase tracking-wide text-zinc-500">Plates</p>
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
                      active ? "border-teal-400" : "border-zinc-700"
                    }`}
                    title={plate.name || `Plate ${plate.plateIndex}`}
                  >
                    {src ? (
                      <img src={src} alt="" className="h-14 w-full object-cover" />
                    ) : (
                      <div className="flex h-14 items-center justify-center bg-zinc-950 text-[10px] text-zinc-400">
                        {plate.plateIndex}
                      </div>
                    )}
                    <span className="block truncate px-1 py-0.5 text-[10px] text-zinc-300">
                      {plate.name || `Plate ${plate.plateIndex}`}
                    </span>
                  </button>
                );
              })}
            </div>
          </div>
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
          {file.plateCount > 0 ? (
            <div className="flex justify-between gap-3">
              <dt>Plates</dt>
              <dd className="text-zinc-200">{file.plateCount}</dd>
            </div>
          ) : null}
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
