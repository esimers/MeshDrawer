# Grok Build — MeshDrawer icon + synthwave theme

**Goal:** Install the locked neon app icon and lightly restyle the UI to match its synthwave palette. Do not change library/scan/dupe logic.

## Source icon
- Use: `docs/meshdrawer-icon-locked.png`
- (Same art as `docs/meshdrawer-icon-f-neon-drawer-explosion.png` — synthwave drawer with neon cubes bursting out.)
- Also present at `C:\Users\earle\MeshDrawer-icons\meshdrawer-icon-locked.png` if needed.

## Part 1 — Tauri icons
1. From repo root, generate the full icon set from the locked PNG:
   `npm run tauri -- icon docs/meshdrawer-icon-locked.png`
2. Confirm `src-tauri/icons/` updates: `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.png`, `icon.ico`, `icon.icns`, and Square*/StoreLogo if the CLI writes them.
3. Keep `src-tauri/tauri.conf.json` bundle.icon paths as-is unless the CLI requires a path tweak.
4. If `tauri icon` refuses opaque RGB, convert the source to a square transparent-capable PNG first (still 1024×1024), then re-run. Prefer a dark transparent master; do not invent a new logo.

## Part 2 — Synthwave theme (light pass)
Current UI is zinc dark + teal accents (`bg-zinc-950`, `bg-teal-600`, etc.) in `App.tsx` and `src/components/*`.

### Palette (from locked icon F)
| Token | Hex | Use |
|---|---|---|
| `--md-bg` | `#0B0A14` | App background (near-black navy) |
| `--md-surface` | `#14121F` | Header/footer/panels |
| `--md-border` | `#2A2438` | Dividers / default borders |
| `--md-text` | `#F5F0FF` | Primary text |
| `--md-muted` | `#9B93B0` | Secondary text |
| `--md-cyan` | `#22D3EE` | Primary accent (focus, selected, progress) |
| `--md-magenta` | `#FF2BD6` | Secondary accent (hover highlight, active glow) |
| `--md-violet` | `#7C3AED` | Subtle glows / badges |
| `--md-danger` | keep amber/red warnings as-is |

### Implementation
1. In `src/App.css` (Tailwind v4 `@import "tailwindcss"`), define a small `@theme` (or CSS variables + utility aliases) for the tokens above. Map Tailwind-ish names if useful, e.g. `md-bg`, `md-cyan`, `md-magenta`.
2. Replace **teal** accent usages with cyan/magenta:
   - Primary buttons: magenta fill OR cyan fill with magenta hover — pick one consistent pattern (recommend `bg-[var(--md-cyan)]` / hover slightly brighter; selected borders `border-[var(--md-magenta)]` or cyan).
   - Focus rings / selected gallery cards / active nav: cyan border + soft magenta/violet glow (`shadow-[0_0_12px_rgba(255,43,214,0.35)]`) sparingly.
   - Progress bar: cyan.
   - Menu hover (`FileContextMenu`): magenta/violet instead of `hover:bg-teal-700`.
3. Swap zinc surfaces to the new bg/surface/border tokens where it’s the app chrome (`App.tsx` main/header/footer, FilterBar, Gallery shells, DetailPane chrome, SettingsPanel, Dialogs). Keep readability high — this is a tint, not a vaporwave wallpaper.
4. Do **not** restyle the Three.js viewer canvas contents; only surrounding chrome.
5. Touch files as needed: `App.css`, `App.tsx`, `components/DetailPane.tsx`, `Dialogs.tsx`, `DuplicateView.tsx`, `FileContextMenu.tsx`, `FilterBar.tsx`, `Gallery.tsx`, `SettingsPanel.tsx`, `SplitHandle.tsx`.
6. Out of scope: auth, new features, layout rewrites, light mode, rewriting copy.

## Done when
- [ ] `src-tauri/icons/*` regenerated from locked icon
- [ ] App chrome uses `#0B0A14` / cyan / magenta instead of teal
- [ ] Primary actions and selection states readable at a glance
- [ ] `npm run build` (or `tsc`) still passes

## Notes
- Windows-first personal MVP; icon.ico matters most.
- Keep changes minimal and cohesive with the locked Flux icon F aesthetic.
