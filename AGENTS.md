# AGENTS.md — working on MobaScape

This file is the contract for AI agents and new contributors. Read it before
touching code. When in doubt, prefer a smaller change that keeps the workspace
building and the cache reader testable.

## 1. What this project is

MobaScape is a 5v5 MOBA with an Old School RuneScape visual identity, built on a
custom Rust engine. It is **not** a RuneScape private server and **not** a
reskin of the 2006Scape Java client. The Java experiment is archived under
`docs/reference/` and is never built.

North star:

- 5v5 real-player PvP; bots are dev-only.
- Server-authoritative simulation at 30 Hz.
- Click-to-move (tap) plus hold-to-steer, QWER abilities.
- Cache-native terrain, scenery, models, animations, icons, and fonts.
- Data-driven champions/items/maps, hot-reloadable.
- Total conversion: boot directly into the MOBA.

## 2. Hard rules

1. **Never commit Jagex assets or caches.** `.cache/`, `assets/`, extracted
   models, textures, maps, sounds, and screenshots of game assets are
   gitignored. Do not add them. The repo ships code and docs only.
2. **Do not trust the client.** Damage, vision, cooldowns, gold, pathing,
   collision, and match results are server-owned. The client predicts only its
   own movement and cosmetics.
3. **The sim crate stays deterministic.** No wall-clock time, no OS RNG, no
   floating-point platform assumptions in `crates/sim`. Use a seeded RNG and
   fixed timestep. Headless matches must produce identical state hashes.
4. **Bots are not production gameplay.** Bots may fill dev matches and soak
   tests; a production match requires 10 humans.
5. **OSRS identity is a constraint, not a suggestion.** 1 tile = 128 world
   units; cache heights are ×8; flat per-face shading; nearest-neighbor textures
   when textures are added. Do not turn the game into a PBR fantasy renderer.
6. **Do not revive the Java server/client.** Port ideas, not code. The Java tree
   is reference only.
7. **Ask before creative direction changes.** Champion roster, map layout, HUD
   look, itemization, and game-feel changes are user decisions. Technical
   refactors and bug fixes do not need a check-in.

## 3. Repo map

```
crates/cache/    cache reader + decoders (terrain, locations, configs, models)
crates/tools/    rift-tools CLI (inspect, dump, export OBJ)
crates/client/   winit + wgpu client (world, animation, HUD, lane, combat)
crates/sim/      deterministic tile grid, collision, A* pathfinding
crates/appimage/ AppImage packer + runtime (tools/appimage/build.sh)
data/maps/       authored lane waypoints, structures, wave timing
crates/data/     (planned) serde content types + hot reload
crates/proto/    (planned) wire messages + snapshot encoding
crates/server/   (planned) match instances, lobby, auth-lite
crates/render/   (planned) shared OSRS-style renderer
crates/ui/       (planned) HUD, shop, scoreboard, lobby, minimap
crates/editor/   (planned) map editor + procedural generator
crates/bots/     (planned) dev slot fillers and headless soak tests
crates/mcp/      (planned) dev MCP server
data/            (planned) champions, abilities, items, maps, waves, UI
docs/            architecture, asset pipeline, roadmap, ADRs
docs/reference/  archived Java-era work; never built
```

## 4. Build and test

```sh
cargo check --workspace
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings   # target: clean
```

M1 commands:

```sh
cargo run -p rift-tools -- cache-info
cargo run -p rift-tools -- terrain 44 47
cargo run -p rift-tools -- locations 44 47
cargo run -p rift-tools -- object 1282
cargo run -p rift-tools -- find-npc monkey --limit 5
cargo run -p rift-tools -- npc 23
cargo run -p rift-tools -- sequence 222
cargo run -p rift-tools -- animate 7744 222
cargo run -p rift-tools -- model 1715
cargo test -p rift-sim
cargo run -p rift-client
cargo run -p rift-client -- --demo-combat
cargo run -p rift-client -- --demo-camp --timescale 4
cargo run -p rift-client -- --demo-kit --timescale 4
cargo run -p rift-client -- --timescale 20
```

Visual checks: use `--shots tools/shots/terrain.txt --present-mode immediate`
(one load, one PNG per preset, then exit) and `--hot-shaders`/F5 for
shader iteration. See `docs/DEV_TOOLS.md`.

Cache location: `.cache/osrs-235/cache` by default. Pass `--cache` and
`--keys` to the tools if yours lives elsewhere. See
`docs/ASSET_PIPELINE.md`.

## 5. Coding conventions

- Rust 2024 edition, `rustfmt`, 4 spaces, no tabs.
- Libraries use `thiserror`; binaries use `anyhow`.
- Use `tracing` (`info!`, `warn!`, `debug!`), never `println!` in library code.
  `rift-tools` may print human-readable output.
- Prefer small modules with explicit data types over trait-heavy abstractions.
- Keep the cache reader allocation-conscious: it is the hot path for asset
  builds.
- Do not add a dependency without a reason. No `unsafe` outside the mmap in
  `crates/cache/src/store.rs` (and document it).
- Public functions get doc comments when the semantics are not obvious.
- Add a regression test for every decoder bug. Cache fixtures should be tiny
  byte arrays or generated groups, not committed game assets.

## 6. How to add things

### Add a cache decoder

1. Port the reference loader from RuneLite/OpenRS2 (BSD-2-Clause) with a comment
   and keep the field names close to the reference.
2. Put it in `crates/cache/src/<name>.rs`, export it from `lib.rs`.
3. Add a `rift-tools` subcommand to inspect it.
4. Verify against one known asset visually (see asset pipeline checklist).
5. Add unit tests for smart reads, opcode skipping, and bounds.

### Add a model/rendering feature

1. Decode into a plain data struct in `crates/cache`; do not put wgpu types in
   the cache crate.
2. Build engine-space vertices in `crates/client/src/scene.rs` (or later
   `crates/render`). Y is up in engine space; cache model Y is flipped.
3. Flat shading: one face = one shade, derived from the face normal. No
   smoothing groups until we deliberately add them.
4. Keep the `Vertex` layout minimal: position + color. Add attributes only with
   a measured need.

### Add a champion or item

1. Add JSON under `data/champions/` or `data/items/` (schema TBD in M1).
2. Reference cache IDs, not baked meshes: `npc`, `model`, `spotanim`, `obj`.
3. Keep ability logic in `crates/sim`; JSON only carries numbers and refs.
4. Run `rift-tools` validation and a headless bot match before opening a PR.

### Add a map

1. Use the editor (M5) or the map JSON schema in `docs/MAP_FORMAT.md`.
2. Never hardcode spawns, lanes, tower positions, or camps in Rust.
3. Bake collision/fog/nav separately from the display mesh.
4. Validate connectivity and spawn clearance before committing.

## 7. Testing requirements

- `cargo test --workspace` must pass.
- Cache changes: add a focused unit test; do not require the full cache in CI.
- Sim changes: add or update a golden state-hash test.
- Netcode changes: test with simulated 0/50/150 ms RTT and 5% loss.
- Renderer changes: include a headless screenshot or a manual visual check note.
- Performance: 1080p/60 target on the AMD 680M iGPU. Profile before optimizing.

## 8. Known M5.4 limitations

- Client renders a 3×3 region block (192×192 tiles); there is no terrain skirt
  or streaming beyond it, and the collision grid is approximate (water + object
  footprints, no cliffs/bridges).
- Three lanes with two towers and one inhibitor per lane, plus one shielded
  nexus per team. The lanes run diagonally between 28-tile castle courtyards
  in the bottom-left and top-right corners, inside a 282×282 crenellated
  perimeter wall with towers and gatehouses. The diagonal river has three
  bridge crossings. Inhibitors respawn after 120 s; while an enemy inhibitor
  is down, that lane's waves include a super minion (2.5x health, 2x damage,
  1.6x scale). Four jungle camps grant Might (+25% damage) or Swift (+20%
  speed) for 60 s. Brush and client-side fog of war are implemented.
- Nexus is damageable only after at least one enemy inhibitor falls.
- The HUD uses cache-native OSRS fonts. `crates/client/src/ui_assets.rs`
  bakes `p11_full`, `p12_full` and `b12_full` from the cache's font and sprite
  archives at startup: `p12_full` for interface text and overhead names,
  `b12_full` for hitmark numbers. The built-in 5x7 bitmap font (`crates/client/
  src/font.rs`) is only the fallback when those archives are unavailable.
  Combat feedback uses the cache's exact hitmarks: red damage, the gold-trimmed
  maximum hit, green poison, teal venom, orange disease, the purple heal cross,
  the blue miss and the white PK skull. Hitmarks stack down the unit, fade after
  1.2 s, and hits from another champion are tinted the reference client's dark
  cool tint. Overhead plates draw larger solid team/camp-coloured names with a
  level chip to the right of the health bar; enemy champions also carry a PK
  skull and a yellow "NO SAFE" tag. The assembled `--ui-live` interface reads
  the same hitmark and plate state from `crates/ui` (`hud/hitmarks.rs`,
  `hud/unit_plates.rs`) so the two paths stay in step.
- **117HD Core** rendering is in: `SceneVertex` carries position, normal,
  colour, UV, AO, material, and texture ID; terrain uses smooth heightfield
  normals, models use flat per-face normals; the scene shader does sun +
  hemispheric ambient, height fog, sky/clouds, blob shadows, and up to 16 point
  lights. Model faces sample cache-native textures from index 9/8 with per-face
  planar UVs and nearest filtering.
- **M5.1 shadows**: a 1024px sun shadow map covers 60x60 tiles around the
  camera with PCF. Player, towers, inhibitors, nexuses, camps, and large static
  props are casters; small ground decor still uses blob shadows. The map
  updates every other frame and can be toggled with
  `--shadows off|blob|map`. Terrain and model vertices carry baked contact AO.
  Full screen-space SSAO and draw-distance streaming are not implemented yet.
- **M5.21/M5.22 presentation**: walkable river with a blended HD water surface
  (animated ripples, fresnel, sun specular), 4x MSAA resolved before post/bloom,
  static sun casters, cache model textures, imp/fairy lanes, cache-soil roads,
  red/wider jungle-camp bars, and camera-offset unit bars to prevent depth
  flicker. Static meshes are split into 16-tile AABB chunks and frustum-culled;
  static sun casters use low-poly trunk/canopy proxies. `--msaa 1|4`,
  `--present-mode fifo|immediate|mailbox` and `--benchmark <seconds>` support
  performance work. The engine backlog lives in `docs/ENGINE_ROADMAP.md`.
- `--graphics classic` preserves a flat baseline; `--debug-view albedo|normal|
  lighting|fog|lights|shadow|ao` helps tune the renderer.
- **UI pass**: the Fountain Shop is a centered OSRS-style window with category
  tabs, item-card grid, detail pane, Buy/Sell buttons, inventory row, and
  cache-rendered item icons. The HUD has resource, team, objectives, event-log,
  compact health/mana, inventory, and shop-button panels. The legacy HUD's
  visual assets are cache-native: `crates/client/src/ui_assets.rs` builds a
  2048×2048 atlas from cache sprites, cache item-model renders, cache ability
  icons, cache fonts and cache hitmarks; `ui.wgsl` samples it with nearest
  filtering. Panels and buttons without a clean cache sprite keep procedural
  geometry. Never commit `assets/`.
- **Match flow**: hero select -> LOCK IN -> 5v5 lobby -> match. The hero-select
  screen is the reference layout (filter tabs ALL/MELEE/RANGED/MAGIC, roster
  grid, side rosters, ability detail panel). The lobby shows both lineups as
  two rows of five hero cards with RADIANT/DIRE side tabs and the red
  "LOADING - PLEASE WAIT..." button; it auto-starts after eight seconds of
  wall-clock time, or the player can click the button or press Enter.
- Four data-driven champions (`monkey`, `jad`, `kree`, `ranger`) with distinct
  models, stats, and ability numbers. A hero-select screen chooses among them
  and bakes cache-model portraits. King Awowogei uses Bananarang (Q), Vine
  Swing (W), Banana Peel (E) and Greegree (R)
  (`crates/client/src/animated/awowogei.rs`); the Blowpipe Ranger uses Toxic Dart, Dart Volley, Footwork and
  Deadshot; Jad and Kree keep the shared Q bolt and R meteor. Kree has ranged
  auto-attacks. No passives yet. Champion files also carry a real `armor` stat
  and movement weight:
  `weight`, optional `turn_time` / `accel_time` / `stop_time`, `stride`, and
  `bounce`. The player turns and accelerates with weight, coasts to a stop,
  and drives its walk cycle from distance covered. `walk_sway` scales torso
  side rotation and `walk_arm` scales local arm transforms; King Awowogei's
  current baseline is 0 for both. `attack_windup` adds committed attack timing
  with an orange charge ring during the wind-up.
- Audio: cache-synthesised sound effects (`rift_cache::sound`,
  `crates/client/src/audio.rs`). Units play their animations' cache frame
  sounds; shared hit, release, cast, shop and UI sounds live in
  `data/sounds.json` and a champion's `sounds` map overrides them by name.
  Only King Awowogei has hand-picked ability sounds so far.
- Ambient, non-combat scenery creatures (butterflies, monkey-riding pasture
  cows, shore crabs) share the unit renderer but use `UnitKind::Ambient`, team
  2, no health bars, and no targeting or attacks.
- Six data-driven items in `data/items/items.json`, six inventory slots, gold
  spending, hover tooltips, and a fountain-range shop (`B`, then `1`–`6`).
  Passive gold is 8/s and the fountain heals 6% max HP/MP per second within
  900 units. Purchases are client-side in this prototype; no selling,
  combining, or passives yet.
- Fog/brush is **client-side** in this prototype. A 192×192 visibility grid is
  computed from allied units/structures, enemy/neutral units outside vision are
  hidden, brush rectangles hide units unless the player is close, and a dark fog
  mesh covers unseen tiles. The minimap draws 8×8-tile fog blocks and supports
  click-to-pan. Server-authoritative fog is still required.
- Model formats 1 and old-format models are not implemented yet.
- Model textures are sampled, but texture alpha cut-outs are not: leaf and
  flower sprites are drawn opaque. The `0xff00ff` terrain overlay sentinel
  still uses a flat fallback colour. Dead-tree trunks read as pale spikes.
- Animaya is not applied; only classic frame/framemap animation works.
- Sound effects are cache-synthesised OSRS sounds (`rift_cache::sound`)
  played through `rodio` (`crates/client/src/audio.rs`); champion files
  name them in `sounds`. Only King Awowogei uses them so far. No music.
- No netcode or map editor. Ambient NPCs idle only.
- The legacy HUD no longer reads visual assets from `assets/osrs-ui`: item
  icons are cache item-model renders, ability icons are champion cache sprite
  ids, and the score-strip sword/skull icons are cache sprites. The old
  external tree is retired for visuals; only cache content is used.
- The cache has no clean generic shop-tab/button sprite for every role, so
  those panels keep the procedural OSRS-styled geometry until a suitable
  cache interface sprite is selected.
- Terrain color is blurred per vertex (two separable box-blur passes) and the
  scene shader samples pre-baked OSRS ground sprites plus procedural detail.
  Terrain is **not** cache-native per-tile texture blending yet; ground-layer
  decoding and anisotropic blending remain M5.3b.
- M5.4 currently ships an offscreen post pass with FXAA, cheap bloom, color
  grading, and dithering. Full SSAO, anisotropic filtering, and chunked
  draw-distance/LOD are still pending. The post pass samples the scene with corrected NDC UVs and the
  HUD is rendered after post so text/icons stay crisp.
- LZMA groups are implemented but not exercised by the pinned cache.
- Config archives use real file IDs from the reference table; keep it that way.
- The viewer follows the player by default. Arrow keys pan; `F` toggles follow.
- Use ASCII in window/UI strings until fonts are wired.
- 117HD Core is still a 3x3 region block; the distant ground plane and fog hide
  the edge from the default camera, but low pitch views still show the block
  boundary and the hard fog-of-war mesh. A chunked draw-distance/LOD milestone
  is required for true 117HD range.

## 9. Commit / PR style

- Commits: `area: imperative summary` (e.g. `cache: decode type 2 model offsets`).
- One logical change per commit; include the test that proves it.
- PR description: what changed, how it was verified, known gaps, and any asset
  IDs used for manual checks.
- Do not mix refactors with behavior changes.
- **End every task with a commit and push.** Before reporting completion, run
  `git status`. If there are changes, commit them with a scoped message and run
  `git push origin main`. Never leave a task with uncommitted work unless the
  user explicitly asks you to. Verify with `git status` (clean) and
  `git ls-remote --heads origin main` (matches local `HEAD`).

## 10. Escalation

Ask the user before:

- changing the champion roster, map layout, camera, or control scheme;
- adding a new external service, account system, or telemetry;
- committing anything derived from Jagex assets;
- making the repository public or changing its license;
- large dependency or architecture changes.
