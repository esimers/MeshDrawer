import type { FileFilter } from "../lib/search";

type Props = {
  filter: FileFilter;
  tags: string[];
  resultCount: number;
  totalCount: number;
  onChange: (next: FileFilter) => void;
};

export function FilterBar({ filter, tags, resultCount, totalCount, onChange }: Props) {
  return (
    <div className="flex flex-wrap items-center gap-2 border-b border-zinc-800 bg-zinc-900/70 px-4 py-2">
      <input
        value={filter.query}
        onChange={(event) => onChange({ ...filter, query: event.target.value })}
        placeholder="Search name, path, tags, notes…"
        className="min-w-[180px] flex-1 rounded-md border border-zinc-700 bg-zinc-950 px-3 py-1.5 text-sm outline-none focus:border-teal-500"
      />
      <select
        value={filter.ext}
        onChange={(event) => onChange({ ...filter, ext: event.target.value as FileFilter["ext"] })}
        className="rounded-md border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-sm"
      >
        <option value="all">All formats</option>
        <option value="stl">STL</option>
        <option value="3mf">3MF</option>
        <option value="obj">OBJ</option>
      </select>
      <select
        value={filter.status}
        onChange={(event) =>
          onChange({ ...filter, status: event.target.value as FileFilter["status"] })
        }
        className="rounded-md border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-sm"
      >
        <option value="all">Any status</option>
        <option value="unset">Unset</option>
        <option value="unprinted">Unprinted</option>
        <option value="printed">Printed</option>
        <option value="favorite">Favorite</option>
      </select>
      <select
        value={filter.tag}
        onChange={(event) => onChange({ ...filter, tag: event.target.value })}
        className="rounded-md border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-sm"
      >
        <option value="">Any tag</option>
        {tags.map((tag) => (
          <option key={tag} value={tag}>
            {tag}
          </option>
        ))}
      </select>
      <input
        value={filter.minMb}
        onChange={(event) => onChange({ ...filter, minMb: event.target.value })}
        placeholder="Min MB"
        className="w-20 rounded-md border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-sm"
      />
      <input
        value={filter.maxMb}
        onChange={(event) => onChange({ ...filter, maxMb: event.target.value })}
        placeholder="Max MB"
        className="w-20 rounded-md border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-sm"
      />
      <span className="text-xs text-zinc-500">
        {resultCount}/{totalCount}
      </span>
    </div>
  );
}
