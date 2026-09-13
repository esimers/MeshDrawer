import { invoke } from "@tauri-apps/api/core";
import * as THREE from "three";
import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js";
import { OBJLoader } from "three/examples/jsm/loaders/OBJLoader.js";
import { asUint8Array, toArrayBuffer } from "./bytes";
import { decodeMeshBlob } from "./meshBlob";
import { isCadPreviewExt, isMeshThumbExt } from "./files";
import type { FileRow } from "../types";

export const THUMB_TRI_CAP = 250_000;
const THUMB_SIZE = 256;
const THUMB_MAX_BYTES = 80 * 1024 * 1024;

function triangleCount(geo: THREE.BufferGeometry): number {
  const index = geo.getIndex();
  if (index) {
    return Math.floor(index.count / 3);
  }
  const pos = geo.getAttribute("position");
  return pos ? Math.floor(pos.count / 3) : 0;
}

function fitCamera(geo: THREE.BufferGeometry, camera: THREE.PerspectiveCamera) {
  geo.computeBoundingBox();
  const box = geo.boundingBox ?? new THREE.Box3();
  const center = new THREE.Vector3();
  const size = new THREE.Vector3();
  box.getCenter(center);
  box.getSize(size);
  geo.translate(-center.x, -center.y, -center.z);
  const maxDim = Math.max(size.x, size.y, size.z, 1);
  camera.position.set(maxDim * 1.15, maxDim * 0.9, maxDim * 1.15);
  camera.near = maxDim / 100;
  camera.far = maxDim * 20;
  camera.lookAt(0, 0, 0);
  camera.updateProjectionMatrix();
}

function geometryFromBytes(ext: string, bytes: Uint8Array): THREE.BufferGeometry {
  if (ext === "stl") {
    const loader = new STLLoader();
    const geo = loader.parse(toArrayBuffer(bytes));
    geo.computeVertexNormals();
    return geo;
  }
  if (ext === "obj") {
    const text = new TextDecoder("utf-8", { fatal: false }).decode(bytes);
    const obj = new OBJLoader().parse(text);
    const geos: THREE.BufferGeometry[] = [];
    obj.traverse((child) => {
      const mesh = child as THREE.Mesh;
      if (mesh.isMesh && mesh.geometry) {
        geos.push(mesh.geometry);
      }
    });
    if (geos.length === 0) {
      throw new Error("OBJ has no mesh");
    }
    const geo = geos[0];
    geo.computeVertexNormals();
    return geo;
  }
  throw new Error(`no mesh thumbnail for .${ext}`);
}

function renderPng(geo: THREE.BufferGeometry): Promise<Uint8Array> {
  const canvas = document.createElement("canvas");
  canvas.width = THUMB_SIZE;
  canvas.height = THUMB_SIZE;
  canvas.style.position = "fixed";
  canvas.style.left = "-1000px";
  canvas.style.top = "0";
  document.body.appendChild(canvas);

  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    alpha: false,
    preserveDrawingBuffer: true,
  });
  renderer.setSize(THUMB_SIZE, THUMB_SIZE, false);
  renderer.setClearColor(0x18181b, 1);

  const scene = new THREE.Scene();
  scene.add(new THREE.AmbientLight(0xffffff, 0.55));
  const key = new THREE.DirectionalLight(0xffffff, 0.9);
  key.position.set(2, 3, 4);
  scene.add(key);

  const camera = new THREE.PerspectiveCamera(35, 1, 0.1, 1000);
  fitCamera(geo, camera);

  const mesh = new THREE.Mesh(
    geo,
    new THREE.MeshStandardMaterial({
      color: 0x5eead4,
      metalness: 0.05,
      roughness: 0.45,
    }),
  );
  scene.add(mesh);
  renderer.render(scene, camera);

  return new Promise((resolve, reject) => {
    canvas.toBlob(
      (blob) => {
        renderer.dispose();
        canvas.remove();
        if (!blob) {
          reject(new Error("failed to encode thumbnail"));
          return;
        }
        blob
          .arrayBuffer()
          .then((buf) => resolve(new Uint8Array(buf)))
          .catch(reject);
      },
      "image/png",
    );
  });
}

export async function renderOneMeshThumb(file: FileRow): Promise<void> {
  if (file.size > THUMB_MAX_BYTES) {
    await invoke("set_thumb_status", {
      id: file.id,
      status: "skipped",
      triangleCount: null,
    });
    return;
  }

  let geo: THREE.BufferGeometry;
  if (isCadPreviewExt(file.ext)) {
    try {
      const payload = await invoke("load_preview_mesh", { id: file.id });
      const parts = decodeMeshBlob(asUint8Array(payload));
      if (parts.length === 0) {
        throw new Error("CAD tessellation produced no mesh");
      }
      geo = new THREE.BufferGeometry();
      geo.setAttribute("position", new THREE.BufferAttribute(parts[0].vertices, 3));
      if (parts[0].indices.length > 0) {
        geo.setIndex(new THREE.BufferAttribute(parts[0].indices, 1));
      }
      geo.computeVertexNormals();
    } catch (err) {
      await invoke("set_thumb_status", {
        id: file.id,
        status: "failed",
        triangleCount: null,
      });
      throw err;
    }
  } else {
    const payload = await invoke("read_model_bytes", { id: file.id });
    geo = geometryFromBytes(file.ext, asUint8Array(payload));
  }
  const tris = triangleCount(geo);
  if (tris > THUMB_TRI_CAP) {
    geo.dispose();
    await invoke("set_thumb_status", {
      id: file.id,
      status: "skipped",
      triangleCount: tris,
    });
    return;
  }

  try {
    const png = await renderPng(geo);
    await invoke("save_thumb_png", { id: file.id, png: Array.from(png) });
  } catch (err) {
    await invoke("set_thumb_status", {
      id: file.id,
      status: "failed",
      triangleCount: tris,
    });
    throw err;
  } finally {
    geo.dispose();
  }
}

export function needsMeshThumb(file: FileRow): boolean {
  return file.thumbStatus === "none" && isMeshThumbExt(file.ext);
}
