# Developer tools

Debugging and art-iteration aids built into `rift-client`. None of them
change normal play: they are CLI flags or function keys.

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
