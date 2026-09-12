import { describe, expect, it } from "vitest";
import { decodeMeshBlob } from "./meshBlob";

function u32(n: number): number[] {
  const b = new ArrayBuffer(4);
  new DataView(b).setUint32(0, n, true);
  return [...new Uint8Array(b)];
}

function f32(n: number): number[] {
  const b = new ArrayBuffer(4);
  new DataView(b).setFloat32(0, n, true);
  return [...new Uint8Array(b)];
}

describe("decodeMeshBlob", () => {
  it("reads one triangle part", () => {
    const bytes = Uint8Array.from([
      0x4d, 0x44, 0x33, 0x4d,
      ...u32(1),
      ...u32(1),
      ...u32(3),
      ...u32(3),
      ...f32(1),
      ...f32(0),
      ...f32(0),
      ...f32(0),
      ...f32(0),
      ...f32(0),
      ...f32(1),
      ...f32(0),
      ...f32(0),
      ...f32(0),
      ...f32(1),
      ...f32(0),
      ...u32(0),
      ...u32(1),
      ...u32(2),
    ]);
    const parts = decodeMeshBlob(bytes);
    expect(parts).toHaveLength(1);
    expect(parts[0].vertices).toHaveLength(9);
    expect([...parts[0].indices]).toEqual([0, 1, 2]);
    expect(parts[0].color[0]).toBeCloseTo(1);
  });
});
