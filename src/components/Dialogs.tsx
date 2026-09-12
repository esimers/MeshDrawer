import { useEffect, useRef, useState, type ReactNode } from "react";
import { formatBytes } from "../lib/format";
import { extLabel, fileNameFromRel, fileStem, formatModified } from "../lib/files";
import type { FileRow } from "../types";

export function RenameDialog({
  file,
  onCancel,
  onSubmit,
}: {
  file: FileRow;
  onCancel: () => void;
  onSubmit: (name: string) => void;
}) {
  const [value, setValue] = useState(fileStem(file.relPath));
  const inputRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    inputRef.current?.select();
  }, []);

  return (
    <Modal title="Rename" onClose={onCancel}>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          onSubmit(value);
        }}
      >
        <label className="block text-sm text-zinc-300">
          New name
          <span className="mt-2 flex overflow-hidden rounded-md border border-zinc-700 bg-zinc-950">
            <input
              ref={inputRef}
              value={value}
              onChange={(event) => setValue(event.target.value)}
              className="min-w-0 flex-1 bg-transparent px-3 py-2 text-zinc-100 outline-none"
            />
            <span className="border-l border-zinc-800 px-3 py-2 text-zinc-500">
              .{file.ext}
            </span>
          </span>
        </label>
        <div className="mt-5 flex justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="rounded-md border border-zinc-700 px-3 py-1.5 text-sm hover:bg-zinc-800"
          >
            Cancel
          </button>
          <button
            type="submit"
            className="rounded-md bg-teal-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-teal-500"
          >
            Rename
          </button>
        </div>
      </form>
    </Modal>
  );
}

export function DeleteDialog({
  file,
  onCancel,
  onConfirm,
}: {
  file: FileRow;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <Modal title="Delete" onClose={onCancel}>
      <p className="text-sm text-zinc-300">
        Move <span className="font-medium text-zinc-100">{fileNameFromRel(file.relPath)}</span> to
        the Recycle Bin?
      </p>
      <p className="mt-2 text-xs text-zinc-500">
        MeshDrawer will drop it from the library. You can restore the file from the Recycle Bin.
      </p>
      <div className="mt-5 flex justify-end gap-2">
        <button
          type="button"
          onClick={onCancel}
          className="rounded-md border border-zinc-700 px-3 py-1.5 text-sm hover:bg-zinc-800"
        >
          Cancel
        </button>
        <button
          type="button"
          onClick={onConfirm}
          className="rounded-md bg-red-700 px-3 py-1.5 text-sm font-medium text-white hover:bg-red-600"
        >
          Delete
        </button>
      </div>
    </Modal>
  );
}

export function PropertiesDialog({
  file,
  onClose,
  onExplorer,
  onWindowsProps,
}: {
  file: FileRow;
  onClose: () => void;
  onExplorer: () => void;
  onWindowsProps: () => void;
}) {
  const folder = file.absPath.replace(/[\\/][^\\/]+$/, "") || file.absPath;
  return (
    <Modal title="Properties" onClose={onClose}>
      <dl className="space-y-2 text-sm">
        <Prop label="Name" value={fileNameFromRel(file.relPath)} />
        <Prop label="Type" value={`${extLabel(file.ext)} file`} />
        <Prop label="Location" value={folder} />
        <Prop label="Size" value={formatBytes(file.size)} />
        <Prop label="Modified" value={formatModified(file.mtimeMs)} />
        {file.plateCount > 0 ? <Prop label="Plates" value={String(file.plateCount)} /> : null}
        <Prop label="Thumbnail" value={file.thumbStatus} />
      </dl>
      <p className="mt-3 break-all font-mono text-[11px] text-zinc-500">{file.absPath}</p>
      <div className="mt-5 flex flex-wrap justify-end gap-2">
        <button
          type="button"
          onClick={onExplorer}
          className="rounded-md border border-zinc-700 px-3 py-1.5 text-sm hover:bg-zinc-800"
        >
          Show in Explorer
        </button>
        <button
          type="button"
          onClick={onWindowsProps}
          className="rounded-md border border-zinc-700 px-3 py-1.5 text-sm hover:bg-zinc-800"
        >
          Windows properties
        </button>
        <button
          type="button"
          onClick={onClose}
          className="rounded-md bg-teal-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-teal-500"
        >
          Close
        </button>
      </div>
    </Modal>
  );
}

function Prop({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[88px_1fr] gap-2">
      <dt className="text-zinc-500">{label}</dt>
      <dd className="break-all text-zinc-100">{value}</dd>
    </div>
  );
}

function Modal({
  title,
  onClose,
  children,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
}) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50 p-4">
      <div
        role="dialog"
        aria-modal="true"
        className="w-full max-w-md rounded-lg border border-zinc-700 bg-zinc-900 p-4 shadow-2xl"
      >
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-sm font-semibold">{title}</h2>
          <button type="button" onClick={onClose} className="text-xs text-zinc-400 hover:text-white">
            Close
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
