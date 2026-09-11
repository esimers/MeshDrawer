# MeshDrawer — Research Brief & Grok Build Plan (v1)

Personal-first Windows MVP. Competitive alternative to Meshory-class local 3D print libraries.
Implement via **Grok Build** only. No application feature code until this plan is approved.

---

## 1. Idea restatement

**Product:** **MeshDrawer** — a local desktop library that indexes STL / 3MF / OBJ (and companions) where they already live, with rendered/embedded thumbnails, a real 3D preview, multi-plate Bambu 3MF support, byte-level duplicate detection, search/tags, and one-click open in a slicer — so you stop re-downloading and hunting filenames.

**Target user (v1):** You — messy ~100GB local library, lots of multi-plate Bambu 3MFs, Windows machine.

**Problem:** Explorer doesn’t show geometry; slicer libraries are prep tools not archives; commercial organizers exist but the goal is to build rather than buy.

**Success for v1:** Point at folder(s) → browse a smooth gallery over ~100GB → preview STL/3MF (incl. plates) → find exact dupes → open in Bambu/Orca — all offline, files never uploaded/moved unless you ask.

**Constraints:** Windows first (Linux later, no Mac); local disk only; solo + Grok Build; sell later maybe — keep architecture clean; no Godot.

**Repo:** `Z:\OneDrive\Documents\GitHub\MeshDrawer` (slug `mesh-drawer` fine for npm/cargo if needed).

---

## 2. Users and jobs-to-be-done

**Primary:** You (power hoarder of print files).

**Jobs**
1. See what a file *is* without opening a slicer.
2. Find a model by memory (“that articulated dragon”) not `final_v2(3).stl`.
3. Preview multi-plate Bambu projects plate-by-plate.
4. Catch re-downloads (exact duplicates).
5. Send a file to the slicer in one click.

**Non-jobs (v1):** AI semantic search, geometry-near-dupes, NAS sync, cloud, storefront, STEP/FBX perfection, Mac.

---

## 3. Research findings

### Named / primary comps (public sites ~Sep 2026)

| Product | Fit | Model |
|---|---|---|
| [Meshory](https://meshory.com/) | Reference UI/ambition: thumbs, 3D viewer, multi-part Bambu 3MF, content and geometry dupes, archives, optional AI via OpenRouter; claims TB-scale hardening | Local; one-time / early-access framing |
| [Printarian](https://ampo3d.com/pages/printarian) | Closest Windows peer: multi-plate 3MF + colors, byte dupes, print tracking; ~54k-file stress | Trial (50 models) → one-time |
| [Modelist](https://modelist.app/) | Gallery, ZIP peek, byte/name/perceptual dupes, slicer launch | €20 v1 / €45 lifetime |
| Printventory / PrintVault | OSS / free-ish; hash dupes; more DIY | Free / open |

**Market read (opinion):** Category is real; Meshory/Printarian set the bar. Building for yourself is rational if you refuse to buy and want control. Competing commercially later means matching multi-plate + scale + polish.

**Tech clues:** Rust 3MF ecosystem includes `lib3mf-core` (vendor/Bambu-aware parsing) and tools that extract plate thumbnails from Bambu/Orca 3MFs — evaluate licenses before depending.

---

## 4. Recommendation

**Build MeshDrawer** personal MVP on **Tauri 2 + React + Rust + three.js**.

**Reasons:** Local-first Windows desktop; Rust for scan/hash/~100GB; React gallery is Grok-Build-friendly; three.js for viewer; Linux later via Tauri.

**Don’t:** Clone Meshory feature-for-feature in v1 (geometry AI dupes, every archive format).  
**Don’t:** Use Godot as the app shell.  
**Do:** Prefer **embedded 3MF plate PNGs** before expensive mesh renders.

---

## 5. MVP definition (in / out)

### In (v1)
- Watch/index user-selected folders in place (no forced move/rename)
- Formats: **STL, 3MF, OBJ** + sidecar images/PDF/txt as companions
- SQLite metadata DB (path, size, mtime, hash, tags, notes, thumb path, plate count…)
- Gallery + search (name, path, tags, format, size)
- Thumbnails: extract embedded 3MF/plate images first; background mesh render for STL/OBJ
- 3D viewer: rotate/pan/zoom; **multi-plate 3MF** plate sidebar + part colors when present
- Duplicates: **SHA-256** exact match + safe review UI (quarantine/trash, don’t auto-delete)
- “Open with” Bambu Studio / Orca / system default
- Progress UI for initial scan of ~100GB

### Out (v1)
- Geometry / perceptual near-dupes
- AI tagging / OpenRouter
- RAR/7z deep browse (ZIP optional if easy; else v1.1)
- STEP / FBX / 3DM
- Cloud, accounts, licensing
- Linux packaging (design for it, ship later)
- Print farm queue / packing lists

---

## 6. Proposed architecture and stack

| Layer | Choice | Why |
|---|---|---|
| Shell | **Tauri 2** (Windows) | Native, local FS, lighter than Electron |
| UI | React + TypeScript + Vite + Tailwind | Fast for Grok Build; virtualized grid |
| Local DB | SQLite via Rust (`sqlx` / `rusqlite`) | Metadata only; files stay on disk |
| Index / hash / 3MF | **Rust** commands + background tasks | 100GB scan without freezing UI |
| Viewer | **three.js** (or react-three-fiber) in webview | STL/OBJ/mesh preview |
| 3MF | ZIP+XML parse; evaluate `lib3mf-core` / Bambu thumbnail extract | Multi-plate + embedded PNGs |
| Thumbs cache | App data dir (`%LOCALAPPDATA%\MeshDrawer\thumbs`) | Don’t rewrite source folders |

**Data model (core):** `roots`, `files`, `plates` (for 3MF), `tags`, `file_tags`, `duplicates` (hash groups), `settings`.

**License rule:** MIT/Apache-2.0 deps preferred; verify any 3MF crate before locking.

---

## 7. Risks and unknowns

| Risk | Mitigation |
|---|---|
| Multi-plate Bambu quirks | Spike early: open 5 of *your* real 3MFs; extract plate PNGs + mesh |
| 100GB first scan feels forever | Incremental index, priority thumbs for viewport, pause/resume |
| Mesh render CPU/GPU blowups | Cap triangle count; deferred render; prefer embedded thumbs |
| False confidence on “dupes” | Label v1 as **exact byte** only; geometry later |
| Path moves break DB | Reconcile by hash + relative path; “relink root” |
| Tauri + large webview memory | Virtualize gallery; unload viewer when closed |

**Unknowns to spike in week 1:** Best 3MF crate for *your* Bambu files; slicer CLI/URI open on Windows.

---

## 8. Grok Build plan

Phased, numbered, actionable. Repo: **MeshDrawer**.

### Phase 0 — Init
1. Scaffold Tauri 2 + React + TS + Tailwind + Vitest; app display name **MeshDrawer**.
2. README: personal MVP goals, non-goals, MIT.
3. App data paths + SQLite bootstrap migration.

### Phase 1 — Index shell
4. “Add library folder” → store root; recursive scan STL/3MF/OBJ.
5. Persist `files` rows (path, size, mtime, ext); ignore patterns in settings.
6. Gallery list (virtualized) with filename + size; empty state.
7. Rescan / watch folder for creates/deletes (basic).

### Phase 2 — Thumbs + viewer
8. Extract embedded images from 3MF when present → thumb cache.
9. STL/OBJ: background mesh thumbnail renderer (queue).
10. Detail pane: three.js viewer for single mesh.
11. Open file location + Open with slicer (configurable paths).

### Phase 3 — Multi-plate 3MF (priority)
12. Parse plate list from Bambu/Orca 3MF; store `plates`.
13. UI: plate picker; show embedded plate thumbnail.
14. Load selected plate meshes/colors into viewer when feasible.
15. Spike doc: results on sample of user’s real multi-plate files.

### Phase 4 — Duplicates
16. Streaming SHA-256 for each file; store hash.
17. Duplicate groups UI; actions: reveal, quarantine/move to folder, ignore.
18. Never auto-delete.

### Phase 5 — Organize
19. Tags + notes; filter/search.
20. Optional status: not printed / printed / favorite.
21. Settings: roots, slicer paths, thumb quality, exclude patterns.

### Phase 6 — Hardening for ~100GB
22. Scan progress, pause, errors (unreadable files).
23. DB indexes; perf pass on gallery scroll.
24. Windows installer via Tauri bundler.
25. Manual QA checklist on a real 100GB subset then full library.

### Run (dev)
```bash
npm install
npm run tauri dev
npm test
npm run tauri build
```

---

## 9. First Grok Build prompt (copy-paste)

```
Plan mode only — do not write application feature code until the plan is approved.

Build Windows desktop app **MeshDrawer** (Tauri 2 + React + TypeScript + Tailwind + Rust backend + SQLite + three.js viewer).
Repo: MeshDrawer.

Product: Personal local 3D print file library (~100GB). Index folders in place (do not move/upload files). Gallery with thumbnails, search/tags, real 3D preview, multi-plate Bambu/Orca 3MF plate preview (embedded plate images + viewer), SHA-256 exact duplicate finder with quarantine (no auto-delete), open in Bambu Studio/Orca/system default.

v1 formats: STL, 3MF, OBJ (+ companion images/pdf/txt). Out of v1: Godot, AI tagging, geometry near-dupes, STEP/FBX, cloud, Mac, Linux packaging, RAR/7z (ZIP optional).

Prefer embedded 3MF/plate thumbnails before expensive mesh renders. Background worker queue for hashing and thumb generation. Virtualize the gallery.

Evaluate MIT/Apache Rust 3MF libraries (e.g. lib3mf-core) against real Bambu multi-plate files; document spike results.

Deliver a phased implementation plan: init, folder index, thumbs+viewer, multi-plate 3MF, SHA-256 dupes, tags/search, 100GB hardening, Windows build. Stop for review before scaffolding.
```

---

## Status

Plan v1 locked for MeshDrawer. Paste section 9 into Grok Build plan mode when ready.
