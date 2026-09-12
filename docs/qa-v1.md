# MeshDrawer v1 QA checklist

Run against a **small subset** first, then the full ~100GB library.

## Install / launch

- [ ] `npm run tauri build` produces an NSIS installer (and MSI if WiX is installed)
- [ ] Install for the current user; app starts as **MeshDrawer**
- [ ] Data lives under `%LOCALAPPDATA%\MeshDrawer\` (DB + thumbs), not in the library folders

## Index

- [ ] Add a library folder; nested `.stl` / `.3mf` / `.obj` appear
- [ ] Mixed-case extensions (`.STL`) are indexed
- [ ] Pause / Resume during scan
- [ ] Rescan picks up new files and drops deleted ones
- [ ] Unreadable or locked files increment the error banner; gallery cards can show a red dot
- [ ] Exclude patterns in Settings skip those paths on the next rescan

## Gallery / preview

- [ ] Grid scrolls smoothly with thousands of cards
- [ ] 3MF cards show embedded plate thumbs
- [ ] STL/OBJ mesh thumbs fill in after scan (huge meshes skipped)
- [ ] Click a card: 3D viewer (STL/OBJ) or plate image (3MF)
- [ ] Close the detail pane; GPU preview unloads (no leftover canvas)
- [ ] Drag the split to resize; width persists after restart
- [ ] Right-click: Open with slicer, Reveal, Rename, Delete (Recycle Bin), Properties

## Multi-plate 3MF

- [ ] Small multi-plate file: plate strip, thumbs follow selection
- [ ] Mesh for the selected plate loads, or PNG + Open in slicer if parse fails
- [ ] A large 24-plate project still shows plate images without freezing the UI

## Duplicates

- [ ] Hashing runs in the background (smallest files first)
- [ ] Duplicate groups are **exact byte matches** only
- [ ] Quarantine moves extras to `.meshdrewer-quarantine` (or Settings path)
- [ ] Ignore group hides the set
- [ ] Nothing is auto-deleted

## Organize

- [ ] Search matches name, path, tags, notes
- [ ] Format / status / tag / size filters work together
- [ ] Status, tags, and notes persist after restart

## Settings / slicers

- [ ] Bambu Studio / Orca / system default open the selected file
- [ ] Custom slicer paths in Settings are used after Save
- [ ] Remove a library folder; those files leave the gallery (files stay on disk)

## Full library

- [ ] Point at the full hoard; first scan can be paused
- [ ] Gallery remains usable before hashing/thumbs finish
- [ ] After a full pass: thumbs, plates, and duplicate groups look sane
