import { formatBytes } from "../lib/format";
import { fileNameFromRel } from "../lib/files";
import { thumbUrl } from "../lib/thumbUrl";
import type { DuplicateGroup } from "../types";

type Props = {
  groups: DuplicateGroup[];
  thumbsDir: string | null;
  hashing: boolean;
  onReveal: (id: number) => void;
  onSelect: (id: number) => void;
  onQuarantine: (id: number) => void;
  onIgnore: (sha256: string) => void;
};

export function DuplicateView({
  groups,
  thumbsDir,
  hashing,
  onReveal,
  onSelect,
  onQuarantine,
  onIgnore,
}: Props) {
  if (groups.length === 0) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center px-6 text-center text-sm text-md-muted">
        <p className="text-base text-md-text">No exact byte matches yet</p>
        <p className="mt-2 max-w-md">
          Duplicates are identical files (same SHA-256), not similar shapes.
          {hashing ? " Hashing is still running in the background." : ""}
        </p>
      </div>
    );
  }

  return (
    <div className="min-h-0 flex-1 overflow-auto px-4 py-4">
      <p className="mb-3 text-xs text-md-muted">
        Exact byte match · {groups.length} group{groups.length === 1 ? "" : "s"} · nothing is deleted
        automatically
      </p>
      <div className="space-y-4">
        {groups.map((group) => {
          const keep = group.files[0];
          const extras = group.files.slice(1);
          return (
            <section
              key={group.sha256}
              className="overflow-hidden rounded-lg border border-md-border bg-md-surface/60"
            >
              <header className="flex flex-wrap items-center justify-between gap-2 border-b border-md-border px-3 py-2">
                <div>
                  <p className="text-sm font-medium text-md-text">
                    {group.files.length} copies · {formatBytes(group.size)}
                  </p>
                  <p className="font-mono text-[11px] text-md-muted">{group.sha256.slice(0, 16)}…</p>
                </div>
                <button
                  type="button"
                  onClick={() => onIgnore(group.sha256)}
                  className="rounded-md border border-md-border px-2 py-1 text-xs hover:bg-md-border"
                >
                  Ignore group
                </button>
              </header>
              <ul className="divide-y divide-md-border">
                {keep ? (
                  <Durow
                    file={keep}
                    keep
                    thumbsDir={thumbsDir}
                    onReveal={onReveal}
                    onSelect={onSelect}
                    onQuarantine={onQuarantine}
                  />
                ) : null}
                {extras.map((file) => (
                  <Durow
                    key={file.id}
                    file={file}
                    keep={false}
                    thumbsDir={thumbsDir}
                    onReveal={onReveal}
                    onSelect={onSelect}
                    onQuarantine={onQuarantine}
                  />
                ))}
              </ul>
            </section>
          );
        })}
      </div>
    </div>
  );
}

function Durow({
  file,
  keep,
  thumbsDir,
  onReveal,
  onSelect,
  onQuarantine,
}: {
  file: DuplicateGroup["files"][0];
  keep: boolean;
  thumbsDir: string | null;
  onReveal: (id: number) => void;
  onSelect: (id: number) => void;
  onQuarantine: (id: number) => void;
}) {
  const src =
    file.thumbPath && thumbsDir ? thumbUrl(thumbsDir, file.thumbPath) : null;
  return (
    <li className="flex items-center gap-3 px-3 py-2">
      <button type="button" onClick={() => onSelect(file.id)} className="shrink-0">
        {src ? (
          <img src={src} alt="" className="h-12 w-12 rounded bg-md-bg object-contain" />
        ) : (
          <div className="flex h-12 w-12 items-center justify-center rounded bg-md-bg text-[10px] text-md-muted">
            {file.ext.toUpperCase()}
          </div>
        )}
      </button>
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm text-md-text">{fileNameFromRel(file.relPath)}</p>
        <p className="truncate font-mono text-[11px] text-md-muted">{file.absPath}</p>
      </div>
      {keep ? (
        <span className="shrink-0 rounded bg-md-violet/30 px-2 py-0.5 text-[10px] font-medium text-md-cyan">
          Keep
        </span>
      ) : (
        <button
          type="button"
          onClick={() => onQuarantine(file.id)}
          className="shrink-0 rounded-md border border-md-border px-2 py-1 text-xs hover:bg-md-border"
        >
          Quarantine
        </button>
      )}
      <button
        type="button"
        onClick={() => onReveal(file.id)}
        className="shrink-0 rounded-md border border-md-border px-2 py-1 text-xs hover:bg-md-border"
      >
        Reveal
      </button>
    </li>
  );
}
