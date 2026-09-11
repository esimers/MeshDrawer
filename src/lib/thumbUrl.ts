import { convertFileSrc } from "@tauri-apps/api/core";

export function thumbUrl(thumbsDir: string, name: string): string {
  const trimmed = thumbsDir.replace(/[\\/]+$/, "");
  const abs = `${trimmed}\\${name}`;
  return convertFileSrc(abs);
}
