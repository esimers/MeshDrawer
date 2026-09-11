import { useEffect, useRef, useState } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js";
import { OBJLoader } from "three/examples/jsm/loaders/OBJLoader.js";
import { invoke } from "@tauri-apps/api/core";
import { asUint8Array, toArrayBuffer } from "../lib/bytes";
import type { FileRow } from "../types";

const VIEW_TRI_CAP = 1_000_000;

type Props = {
  file: FileRow;
};

function parseGeometry(ext: string, bytes: Uint8Array): THREE.BufferGeometry {
  if (ext === "stl") {
    const geo = new STLLoader().parse(toArrayBuffer(bytes));
    geo.computeVertexNormals();
    return geo;
  }
  const text = new TextDecoder("utf-8", { fatal: false }).decode(bytes);
  const group = new OBJLoader().parse(text);
  const geos: THREE.BufferGeometry[] = [];
  group.traverse((child) => {
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

function triangleCount(geo: THREE.BufferGeometry): number {
  const index = geo.getIndex();
  if (index) {
    return Math.floor(index.count / 3);
  }
  const pos = geo.getAttribute("position");
  return pos ? Math.floor(pos.count / 3) : 0;
}

export function ModelViewer({ file }: Props) {
  const hostRef = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [tooBig, setTooBig] = useState<number | null>(null);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) {
      return;
    }
    let cancelled = false;
    let renderer: THREE.WebGLRenderer | null = null;
    let controls: OrbitControls | null = null;
    let geo: THREE.BufferGeometry | null = null;
    let frame = 0;

    setLoading(true);
    setError(null);
    setTooBig(null);

    invoke("read_model_bytes", { id: file.id })
      .then((payload) => {
        if (cancelled || !hostRef.current) {
          return;
        }
        const bytes = asUint8Array(payload);
        geo = parseGeometry(file.ext, bytes);
        const tris = triangleCount(geo);
        if (tris > VIEW_TRI_CAP) {
          setTooBig(tris);
          geo.dispose();
          geo = null;
          return;
        }

        const width = host.clientWidth || 360;
        const height = host.clientHeight || 256;
        renderer = new THREE.WebGLRenderer({ antialias: true });
        renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
        renderer.setSize(width, height);
        renderer.setClearColor(0x09090b, 1);
        host.replaceChildren(renderer.domElement);

        const scene = new THREE.Scene();
        scene.add(new THREE.AmbientLight(0xffffff, 0.55));
        const key = new THREE.DirectionalLight(0xffffff, 1.05);
        key.position.set(4, 6, 5);
        scene.add(key);

        geo.computeBoundingBox();
        const box = geo.boundingBox ?? new THREE.Box3();
        const center = new THREE.Vector3();
        const size = new THREE.Vector3();
        box.getCenter(center);
        box.getSize(size);
        geo.translate(-center.x, -center.y, -center.z);
        const maxDim = Math.max(size.x, size.y, size.z, 1);

        const camera = new THREE.PerspectiveCamera(40, width / height, maxDim / 100, maxDim * 20);
        camera.position.set(maxDim * 1.2, maxDim * 0.9, maxDim * 1.2);
        camera.lookAt(0, 0, 0);

        const mesh = new THREE.Mesh(
          geo,
          new THREE.MeshStandardMaterial({
            color: 0x5eead4,
            metalness: 0.08,
            roughness: 0.42,
          }),
        );
        scene.add(mesh);

        controls = new OrbitControls(camera, renderer.domElement);
        controls.enableDamping = true;

        const tick = () => {
          if (!renderer || !controls) {
            return;
          }
          controls.update();
          renderer.render(scene, camera);
          frame = requestAnimationFrame(tick);
        };
        tick();
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
      cancelAnimationFrame(frame);
      controls?.dispose();
      renderer?.dispose();
      geo?.dispose();
      host.replaceChildren();
    };
  }, [file.id, file.ext]);

  return (
    <div className="relative h-64 w-full overflow-hidden rounded-md bg-zinc-950">
      <div ref={hostRef} className="h-full w-full" />
      {loading ? (
        <p className="absolute inset-0 flex items-center justify-center text-sm text-zinc-400">
          Loading preview…
        </p>
      ) : null}
      {error ? (
        <p className="absolute inset-0 flex items-center justify-center p-4 text-sm text-red-300">
          {error}
        </p>
      ) : null}
      {tooBig ? (
        <p className="absolute inset-0 flex items-center justify-center p-4 text-sm text-zinc-400">
          Preview skipped ({tooBig.toLocaleString()} triangles; cap is 1,000,000). Open
          in a slicer to inspect.
        </p>
      ) : null}
    </div>
  );
}
