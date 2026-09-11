# MeshDrawer

Personal local 3D print file library for Windows. Point it at folders that already hold STL / 3MF / OBJ files; browse a gallery, preview (including multi-plate Bambu/Orca 3MF), find exact byte duplicates, and open in Bambu Studio or Orca Slicer. Files stay on disk. Nothing is uploaded or moved unless you ask.

This is a personal MVP, implemented with Tauri 2 + React + TypeScript + Tailwind + a Rust/SQLite backend.

## Goals (v1)

- Index user-selected folders **in place** (no forced move/rename)
- Formats: STL, 3MF, OBJ + sidecar images / PDF / txt
- Gallery + search (name, path, tags, format, size)
- Thumbnails: extract embedded 3MF/plate PNGs first; mesh-render STL/OBJ in the background
- 3D viewer with multi-plate 3MF plate picker
- SHA-256 exact duplicate finder with quarantine (never auto-delete)
- Open with Bambu Studio / Orca / system default
- Offline only

## Non-goals (v1)

- Geometry or perceptual near-duplicates
- AI tagging / cloud / accounts
- STEP, FBX, 3DM
- RAR / 7z (and ZIP library browse — 3MF is still parsed as ZIP internally)
- macOS; Linux packaging (architecture should not block it later)
- Godot as the app shell

## Develop

Prerequisites: Node 22+, Rust stable ≥ 1.88, Windows 10/11 with WebView2, MSVC build tools.

```bash
npm install
npm run tauri dev
```

```bash
npm test
cd src-tauri && cargo test
```

```bash
npm run tauri build
```

App data (database + thumbnail cache) lives at `%LOCALAPPDATA%\MeshDrawer\`. Source folders are never written to.

## License

MIT
