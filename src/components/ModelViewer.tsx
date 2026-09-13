import { useEffect, useRef, useState } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js";
import { OBJLoader } from "three/examples/jsm/loaders/OBJLoader.js";
import { invoke } from "@tauri-apps/api/core";
import { asUint8Array, toArrayBuffer } from "../lib/bytes";
import { decodeMeshBlob } from "../lib/meshBlob";
import type { FileRow } from "../types";

const VIEW_TRI_CAP = 1_000_000;

type Props = {
  file: FileRow;
  plateIndex?: number;
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

function addColoredParts(scene: THREE.Scene, payload: Uint8Array): THREE.BufferGeometry[] {
  const parts = decodeMeshBlob(payload);
  const geos: THREE.BufferGeometry[] = [];
  for (const part of parts) {
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.BufferAttribute(part.vertices, 3));
    if (part.indices.length > 0) {
      geo.setIndex(new THREE.BufferAttribute(part.indices, 1));
    }
    geo.computeVertexNormals();
    const mesh = new THREE.Mesh(
      geo,
      new THREE.MeshStandardMaterial({
        color: new THREE.Color(part.color[0], part.color[1], part.color[2]),
        metalness: 0.08,
        roughness: 0.42,
      }),
    );
    scene.add(mesh);
    geos.push(geo);
  }
  return geos;
}

function EnlargeIcon({ expanded }: { expanded: boolean }) {
  return (
    <svg viewBox="0 0 24 24" className="h-4 w-4" aria-hidden="true">
      {expanded ? (
        <path
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M9 3v6H3M21 9h-6V3M3 15h6v6M15 21v-6h6"
        />
      ) : (
        <path
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M3 9V3h6M15 3h6v6M3 15v6h6M15 21h6v-6"
        />
      )}
    </svg>
  );
}

export function ModelViewer({ file, plateIndex = 1 }: Props) {
  const hostRef = useRef<HTMLDivElement>(null);
  const rendererRef = useRef<THREE.WebGLRenderer | null>(null);
  const cameraRef = useRef<THREE.PerspectiveCamera | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [tooBig, setTooBig] = useState<number | null>(null);
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setExpanded(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) {
      return;
    }
    let cancelled = false;
    let renderer: THREE.WebGLRenderer | null = null;
    let controls: OrbitControls | null = null;
    let geos: THREE.BufferGeometry[] = [];
    let frame = 0;
    let ro: ResizeObserver | null = null;

    setLoading(true);
    setError(null);
    setTooBig(null);

    const cad = file.ext === "step" || file.ext === "stp" || file.ext === "fbx";
    const requestId = cad ? `${file.id}-${Date.now()}-${Math.random().toString(36).slice(2)}` : null;
    const load =
      file.ext === "3mf"
        ? invoke("load_plate_mesh", { fileId: file.id, plateIndex })
        : cad
          ? invoke("load_preview_mesh", { id: file.id, requestId })
          : invoke("read_model_bytes", { id: file.id });

    load
      .then((payload) => {
        if (cancelled || !hostRef.current) {
          return;
        }
        const bytes = asUint8Array(payload);

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

        if (file.ext === "3mf" || cad) {
          geos = addColoredParts(scene, bytes);
          if (geos.length === 0) {
            setError(
              file.ext === "3mf"
                ? "No mesh for this plate. Embedded image is shown above."
                : "Could not tessellate this CAD file.",
            );
            return;
          }
        } else {
          const geo = parseGeometry(file.ext, bytes);
          const tris = triangleCount(geo);
          if (tris > VIEW_TRI_CAP) {
            setTooBig(tris);
            geo.dispose();
            return;
          }
          geo.computeVertexNormals();
          scene.add(
            new THREE.Mesh(
              geo,
              new THREE.MeshStandardMaterial({
                color: 0x5eead4,
                metalness: 0.08,
                roughness: 0.42,
              }),
            ),
          );
          geos = [geo];
        }

        const box = new THREE.Box3();
        for (const geo of geos) {
          geo.computeBoundingBox();
          if (geo.boundingBox) {
            box.union(geo.boundingBox);
          }
        }
        const center = new THREE.Vector3();
        const size = new THREE.Vector3();
        box.getCenter(center);
        box.getSize(size);
        for (const geo of geos) {
          geo.translate(-center.x, -center.y, -center.z);
        }
        const maxDim = Math.max(size.x, size.y, size.z, 1);

        const camera = new THREE.PerspectiveCamera(40, width / height, maxDim / 100, maxDim * 20);
        camera.position.set(maxDim * 1.2, maxDim * 0.9, maxDim * 1.2);
        camera.lookAt(0, 0, 0);

        controls = new OrbitControls(camera, renderer.domElement);
        controls.enableDamping = true;
        rendererRef.current = renderer;
        cameraRef.current = camera;

        const fitSize = () => {
          const el = hostRef.current;
          if (!el || !rendererRef.current || !cameraRef.current) {
            return;
          }
          const nextW = el.clientWidth || 360;
          const nextH = el.clientHeight || 256;
          cameraRef.current.aspect = nextW / nextH;
          cameraRef.current.updateProjectionMatrix();
          rendererRef.current.setSize(nextW, nextH);
        };
        ro = new ResizeObserver(fitSize);
        ro.observe(host);

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
      if (requestId) {
        void invoke("cancel_preview", { requestId });
      }
      ro?.disconnect();
      cancelAnimationFrame(frame);
      controls?.dispose();
      geos.forEach((geo) => {
        geo.dispose();
      });
      rendererRef.current = null;
      cameraRef.current = null;
      if (renderer) {
        renderer.dispose();
        renderer.forceContextLoss();
        renderer.domElement.remove();
      }
      host.replaceChildren();
    };
  }, [file.id, file.ext, plateIndex]);

  return (
    <>
      {expanded ? (
        <button
          type="button"
          className="fixed inset-0 z-[70] bg-black/55"
          aria-label="Close enlarged viewer"
          onClick={() => setExpanded(false)}
        />
      ) : null}
      <div className={expanded ? "h-64" : undefined}>
        <div
          className={
            expanded
              ? "fixed inset-6 z-[80] overflow-hidden rounded-lg border border-md-border bg-md-bg shadow-[0_0_24px_rgba(255,43,214,0.25)]"
              : "relative h-64 w-full overflow-hidden rounded-md bg-md-bg"
          }
        >
          <button
            type="button"
            onClick={() => setExpanded((value) => !value)}
            title={expanded ? "Shrink viewer" : "Enlarge viewer"}
            aria-label={expanded ? "Shrink viewer" : "Enlarge viewer"}
            className="absolute right-2 top-2 z-10 rounded-md border border-md-border bg-md-surface/90 p-1.5 text-md-cyan hover:border-md-magenta hover:text-md-magenta"
          >
            <EnlargeIcon expanded={expanded} />
          </button>
          <div ref={hostRef} className="h-full w-full" />
          {loading ? (
            <p className="absolute inset-0 flex items-center justify-center text-sm text-md-muted">
              {file.ext === "step" || file.ext === "stp"
                ? "Tessellating STEP…"
                : "Loading preview…"}
            </p>
          ) : null}
          {error ? (
            <p className="absolute inset-0 flex items-center justify-center p-4 text-sm text-red-300">
              {error}
            </p>
          ) : null}
          {tooBig ? (
            <p className="absolute inset-0 flex items-center justify-center p-4 text-sm text-md-muted">
              Preview skipped ({tooBig.toLocaleString()} triangles; cap is 1,000,000). Open
              in a slicer to inspect.
            </p>
          ) : null}
        </div>
      </div>
    </>
  );
}
