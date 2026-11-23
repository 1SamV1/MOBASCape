# Developer tools

Debugging and art-iteration aids built into `rift-client`. None of them
change normal play: they are CLI flags or function keys.

## AppImage launcher (`tools/appimage`)

`tools/appimage/build.sh` writes `dist/MobaScape-x86_64.AppImage`
(`--install` also copies it to `~/Applications` and adds a menu entry).
The AppImage does not freeze a build. Every launch it:

1. finds the checkout (`$MOBASCAPE_REPO`, then `~/.config/mobascape/repo`,
   then the path baked in at build time, else a folder picker);
2. fast-forwards to the upstream branch (`sam/main`) if the tracked tree is
   clean (local edits are never touched; offline just skips this);
3. runs an incremental `cargo build --release -p rift-client` (about 0.2 s
   when nothing changed; a progress window appears for longer builds);
4. runs the client from the checkout root, by default with `--ui-live`.
   Any arguments replace that default and are passed through.

If the build fails, it runs the last good binary in `target/release`, then
the snapshot bundled in the AppImage. `MOBASCAPE_NO_PULL=1` and
`MOBASCAPE_NO_BUILD=1` turn steps 2 and 3 off. The log is at
`~/.local/state/mobascape/launcher.log`. Rebuild the AppImage only when
`AppRun` changes or the checkout moves. It contains no cache or `assets/`
(the OSRS cache stays in `.cache/`).

Packing needs no external tools. `crates/appimage` (`rift-appimage`) writes
the squashfs (gzip, via `backhand`) and appends it to its own runtime
(`rift-appimage-runtime`). The runtime is a standard type-2 layout: `AI\x02`
at ELF offset 8, with the squashfs starting at the end of the ELF section
headers. It extracts once into `~/.cache/rift-appimage/<content hash>`
(older copies are removed) and runs `AppRun`, so FUSE is not needed. A warm
start adds about 20 ms. `--appimage-extract [DIR]`, `--appimage-offset` and
`--appimage-version` behave like the reference runtime's.

## Batch screenshots (`--shots`)

One load, many camera presets, one PNG each, then the client exits.

```sh
cargo run --release -p rift-client -- --champion monkey --ui-live \
  --window-size 1600x900 --present-mode immediate \
  --shots tools/shots/terrain.txt --shot-dir .cache/shots
```

A shot list has one preset per line (`#` starts a comment):

```text
# name      tile_x tile_y [pitch=R] [yaw=R] [distance=U] [view=NAME]
base        2730   3150
low_angle   2860   3000   pitch=0.35 distance=2400
jungle_ao   2830   3080   view=ao
```

- The hero is teleported to the tile and the camera follows it. Options
  that are left out use `--pitch` / `--distance` (or the defaults), with a
  yaw of 0 and the `--debug-view` view.
- `--screenshot-after N` is the warm-up per shot (default 90 frames).
- Use `--present-mode immediate`: with vsync, a hidden or unfocused window
  can stall presentation, and several clients launched at once block each
  other.
- `tools/shots/terrain.txt` covers the base, lane, jungle, river, a low
  angle and an AO view. Six shots take about 12 s, compared with about
  2 minutes per shot before.
- Output goes under `.cache/`, which is gitignored. Screenshots of cache art
  must never be committed.

`--screenshot PATH` now also exits once the frame is written (it used to keep
running until it was killed).

## Shader hot reload

- `--hot-shaders` watches `crates/client/src/scene.wgsl` and recompiles it
  on save (it checks every 15 frames).
- **F5** reloads it on demand, with or without the flag.
- The WGSL is parsed and validated with naga first. A broken edit logs the
  naga diagnostic, with line and column, and keeps the old pipelines, so the
  client never panics. The reload rebuilds the scene and water pipelines in
  about 10 ms.
- Only `scene.wgsl` is hot. Rust-side changes (terrain build, meshes) still
  need a rebuild.

## Hotkeys (in the match and in spectator mode)

| Key | Action |
|---|---|
| F1 | Log the hotkey list |
| F2 / Shift+F2 | Next / previous debug view: lit, albedo, normal, lighting, fog, lights, shadow, ao |
| F5 | Reload `scene.wgsl` |
| F7 | Log the hero tile and camera as a ready-to-paste `--shots` line |
| F9 | Screenshot to `.cache/screenshots/shot-<ms>.png` |
| F10 | Pause or resume the simulation (rendering continues) |

Typical loop: walk to a spot, press F7 and paste the line into a shot list.
Edit `scene.wgsl` and let `--hot-shaders` reload it, or press F5. Then
re-run `--shots` for before/after captures.

## Startup timing

The log prints each startup phase: `startup: cache open`, `scene build`,
`animated scene`, `textures + renderer`, and `first frame after N s`. Use
it to see where load time goes before optimising.

## Combat checks

- `--demo-attack`: park the hero 1,000 units from the nearest jungle camp
  and basic-attack it (attack clip, reach, projectile, impact).
- `--hold-indicator SLOT`: draw the held ability indicator for slot 0-3
  aimed at that camp, as if right mouse were held.
- `--balance-report docs/BALANCE.md`: write the paper balance report
  (effective health, basic and ability DPS, burst, shields, mana per
  rotation at level 1 / rank 1 and level 11 / top ranks) and exit without
  opening a window. Numbers at least 1.35x or at most 0.7x their group's
  median (melee or ranged) are flagged. Re-run after changing champion
  numbers; the model is described at the top of
  `crates/client/src/balance.rs`.
- `--demo-awowogei` (with `--champion monkey`): spawn a rival beside the
  king and cast Bananarang at it (1 s), a Banana Peel under it (3.2 s),
  Greegree (4.4 s) and Vine Swing past it (5.6 s); at 9 s it logs the
  king's health and position, the rival's health and what is still in
  play. Screenshot frames ~130 (banana in flight), ~310 (gorilla) and
  ~386 (swinging).
- `--mute`: no sound. `--shots` and `--benchmark` are always silent.
- `rift-tools sound ID --out sound.wav` decodes a cache sound effect,
  prints its instruments and writes it as a WAV to audition.
  `rift-tools sounds` decodes all of them and lists every animation
  that plays one; `rift-tools sounds --sequences A,B --out dir` lists
  (and writes) the sounds of particular animations.
- `--demo-ranks SLOT`: level the hero to 7, spend four skill points (Q
  twice, W and R once), leave two unspent so the slots glow, and hold the
  cursor on slot 0-3 so its ability tooltip is in the screenshot.
- Combine with `--screenshot out.png --screenshot-after N` and step `N` to
  capture a projectile in flight.
- `rift-tools spotanim ID`, `rift-tools spotanims --from A --to B --sheet
  out.png` and `rift-tools skeleton-anims SEQ --model M --sheet out.png`
  inspect projectile graphics and find attack clips.

## Existing switches (for reference)

`--debug-view`, `--debug-hud`, `--gpu-timing`, `--benchmark N`,
`--graphics classic`, `--shadows off|blob|map`, `--msaa 1|4`,
`--render-scale`, `--spawn X Y`, `--pitch`, `--distance`, `--timescale`
and the `--demo-*` scenarios are listed in the README.
