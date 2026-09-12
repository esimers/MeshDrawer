# 3MF multi-plate spike

Date: 2026-09-11  
Library scanned: local MeshDrawer index (608 `.3mf` files).

## Method

ZIP listing + `Metadata/model_settings.config` plate blocks + Production Extension object parts under `3D/Objects/`. Mesh path is XML (`quick-xml`), not `lib3mf-core`.

`lib3mf-core` was skipped for this pass: Bambu/Orca color and plate mapping live in vendor `model_settings.config` / `project_settings.config`, and large projects split meshes into many `object_N.model` files. ZIP+XML lets us take only the objects for the selected plate.

## Samples

| File | Size | `<plate>` tags | plate PNGs | `.model` files |
|---|---:|---:|---:|---:|
| Pop+Can+Dispenser.3mf | 114 KB | 2 | 6 | 3 |
| Small_Spool_Adapter.3mf | 170 KB | 2 | 6 | 3 |
| 6U-Leg.3mf | 188 KB | 2 | 6 | 10 |
| 5+can+dispenser…3mf | 204 KB | 5 | 15 | 6 |
| rackadapter_all.3mf | 234 KB | 5 | 15 | 10 |
| Multiboard Demo Pack - 6x6 Tile.3mf | 180 MB | 8 | 24 | 36 |
| Finger+Treads+2.0+full+pack.3mf | 194 MB | 24 | 72 | 25 |
| Brick christmas Tree … 20percent.3mf | 195 MB | 24 | 72 | 448 |

All of these are Bambu/Orca Production Extension packages: root `3D/3dmodel.model` plus per-object `3D/Objects/object_*.model`. Plate PNGs exist (`Metadata/plate_N.png` plus small/top/pick variants).

## Results

- Embedded plate images are present and sufficient for the gallery and plate strip.
- Plate count from `<plate>` matches the user’s expectation on these files (2–24).
- Object files are the geometry source; loading every `.model` on a 448-part tree would hitch. The viewer only parses the selected plate’s object ids (and component children).
- Filament color is not in the 3MF Core mesh; it is read from `project_settings.config` `filament_colour` plus per-object `extruder` in `model_settings.config`. If that fails, the viewer uses the teal default.
- Giant trees (~200 MB, 24 plates) should still show plate thumbs immediately. Mesh preview may skip or cap at 1,000,000 triangles.

## Viewer policy

1. Always show the selected plate PNG when extracted.
2. Load that plate’s meshes/colors when XML parse succeeds.
3. If parse yields no triangles, keep the PNG and “Open in slicer”.
