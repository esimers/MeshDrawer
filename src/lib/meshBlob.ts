import { toArrayBuffer } from "./bytes";

export type MeshPart = {
  vertices: Float32Array;
  indices: Uint32Array;
  color: [number, number, number];
};

export function decodeMeshBlob(data: Uint8Array): MeshPart[] {
  const buffer = toArrayBuffer(data);
  const view = new DataView(buffer);
  const magic = String.fromCharCode(
    view.getUint8(0),
    view.getUint8(1),
    view.getUint8(2),
    view.getUint8(3),
  );
  if (magic !== "MD3M") {
    throw new Error("invalid plate mesh payload");
  }
  const count = view.getUint32(8, true);
  let offset = 12;
  const parts: MeshPart[] = [];
  for (let i = 0; i < count; i += 1) {
    const vcount = view.getUint32(offset, true);
    offset += 4;
    const icount = view.getUint32(offset, true);
    offset += 4;
    const r = view.getFloat32(offset, true);
    offset += 4;
    const g = view.getFloat32(offset, true);
    offset += 4;
    const b = view.getFloat32(offset, true);
    offset += 4;
    const vertices = new Float32Array(vcount * 3);
    for (let v = 0; v < vertices.length; v += 1) {
      vertices[v] = view.getFloat32(offset, true);
      offset += 4;
    }
    const indices = new Uint32Array(icount);
    for (let t = 0; t < icount; t += 1) {
      indices[t] = view.getUint32(offset, true);
      offset += 4;
    }
    parts.push({ vertices, indices, color: [r, g, b] });
  }
  return parts;
}
