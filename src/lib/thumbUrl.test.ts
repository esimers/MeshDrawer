import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => `asset:${path}`,
}));

import { thumbUrl } from "./thumbUrl";

describe("thumbUrl", () => {
  it("joins thumbs dir and filename", () => {
    expect(thumbUrl("C:\\Users\\me\\AppData\\Local\\MeshDrawer\\thumbs", "12_cover.png")).toBe(
      "asset:C:\\Users\\me\\AppData\\Local\\MeshDrawer\\thumbs\\12_cover.png",
    );
  });
});
