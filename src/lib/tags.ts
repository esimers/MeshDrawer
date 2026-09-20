import type { FileRow } from "../types";

export type TagCoverage = {
  name: string;
  count: number;
};

export function tagCoverage(files: FileRow[]): TagCoverage[] {
  const map = new Map<string, number>();
  for (const file of files) {
    for (const tag of file.tags) {
      map.set(tag, (map.get(tag) ?? 0) + 1);
    }
  }
  return [...map.entries()]
    .sort((a, b) => a[0].localeCompare(b[0], undefined, { sensitivity: "base" }))
    .map(([name, count]) => ({ name, count }));
}
