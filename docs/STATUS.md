# Status

Last updated: per-pass GPU timings, 2026-11-21.

## 2026-11-21 — Per-pass GPU timings from timestamp queries

- `--gpu-timing` (and every `--benchmark` run) now enables `wgpu` timestamp
  queries when the adapter advertises `TIMESTAMP_QUERY`. Each render pass
  writes one begin/end pair, and one frame in 61 resolves the query set into a
  small buffer that is mapped asynchronously. The CPU never waits on the GPU.
- `benchmark gpu:` reports `shadow`, `scene`, `bloom bright`, `bloom h`,
  `bloom v`, `post`, `hud` and `interface` averages plus their measured sum.
  Interactive runs log a `gpu passes:` line every five samples. Adapters
  without timestamp support log one warning and continue normally.
- Measured on the 680M at 1920×1008 with auto scene scale: **12.00 ms** GPU
  frame, **scene 10.33 ms**, **bloom 0.74 ms**, **post 0.79 ms**, **shadow
  0.05 ms**, HUD/interface **0.09 ms**. The scene pass is the 60 fps target;
  the CPU benchmark stays at p50 12.61 ms / 79 fps.
- Sampling uses a prime interval so it does not line up with the shadow's
  every-other-frame cadence, and a reset at benchmark start discards a
  pre-warm-up readback.
- No new visual assets were introduced; this change only reads counters.
- Verified: 262 tests pass, 4 ignored; workspace clippy clean.

## 2026-11-21 — Live shop delivery control

- The live shop's detail pane now has a `DELIVER (N)` / `DELIVERING` /
  `RETURNING` control above Buy when purchases are waiting or in transit.
  Clicking it calls the same `AnimatedScene::call_delivery` path as the old
  HUD's button.
- The control uses the theme's scraped `button.stone` panel and cache font;
  no new visual assets were introduced.
- The shop bridge now carries the delivery label from `PlayerHud`, and the
  click path reads the live animated state so the label and action agree.
- Added a delivery-label test and a hit-test for the control.
- Verified: 259 tests pass, 4 ignored; workspace clippy clean. Capture:
  `/tmp/opencode/ui_live_delivery.png`.

## 2026-11-21 — Live minimap uses the cache terrain picture

- The live minimap now receives the terrain picture baked from the cached
  terrain colours, fitted from 256×256 to the component’s 254×254 slot. The
  picture is drawn before the river, lanes, structures and dots, so the live
  map reads like the game instead of a black panel.
- `theme_file::load_with_items_and_minimap` adds the `minimap.terrain` role;
  the sample theme still has no such role and keeps its dark sample map.
- No new visual assets were introduced: the picture is generated from the
  cache terrain and resized with nearest filtering to preserve the pixel look.
- Verified: 258 tests pass, 4 ignored; workspace clippy clean. Capture:
  `/tmp/opencode/ui_live_minimap.png`.

## 2026-11-21 — Neutral camp bars in the live unit plates

- The live bridge no longer drops every team 2 unit frame. Neutral camps and
  dummies now become small minion-style plates, so suppressing the legacy HUD
  no longer removes their health bars.
- The plates use the existing scraped cache theme fonts and colours; no new
  visual assets were introduced.
- Updated the plate conversion test to assert a neutral camp stays visible.
- Verified: 257 tests pass, 4 ignored; workspace clippy clean. Capture:
  `/tmp/opencode/ui_live_campbars.png`.

## 2026-11-21 — Live interface suppresses the legacy HUD

- Under `--ui-live` the client no longer builds the old `hud.rs` HUD, its
  shop or its minimap. The assembled interface is the only interface drawn,
  so panels and bars are not duplicated underneath it.
- The benchmark's `hud` phase is now 0 ms in that mode; `--ui-live` at
  1920×1008 still measures p50 12.84 ms / 78 fps with auto scene scale,
  MSAA 4 and sun shadows.
- The old HUD remains the running default outside `--ui-live`. Known live-HUD
  gaps are the empty team frames (one hero a side), neutral camp plates, the
  delivery control and the old HUD's debug/stats surfaces.
- Verified: 257 tests pass, 4 ignored; workspace clippy clean. Capture:
  `/tmp/opencode/ui_live_clean.png`.

## 2026-11-21 — Live objectives list buff camps only

- The live objectives panel was listing every `UnitKind::Camp`, including the
  scattered critter camps whose `camp_buff` is `None`. That produced fifteen
  camp lines and pushed the panel past its layout box.
- The bridge now includes only the eight real buff camps (`camp_buff` is
  `Some(Might|Swift)`), so the panel is the intended ten objective lines plus
  its second lines and fits the screen.
- Added a pure `is_objective_camp` regression test.
- Verified: `cargo test --workspace` 257 tests pass, 4 ignored; workspace
  clippy is clean. Capture: `/tmp/opencode/ui_live_camps.png`.

## 2026-11-21 — UI: flat panels are one rectangle

- Solid theme panels used a 1×1 image and repeated it once per interface
  pixel. At 1080p the live HUD built **162,830 quads / 651,320 vertices** and
  spent ~46.7 ms building plus 23–27 ms uploading them every frame.
- `Panel` now carries an optional solid colour, `ThemeBuilder::solid_panel`
  stores flat roles directly, and `DrawList::panel` draws them as one
  rectangle. The same live HUD frame is **2,327 quads / 9,308 vertices**, with
  the UI build at ~0.9 ms and the upload under 1 ms.
- `--ui-live` at 1920×1008, auto scene scale, MSAA 4 and sun shadows is
  **p50 12.71 ms / 78.1 fps** (`ui=0.88 ms`, `submit=8.05 ms`). This was
  ~19 fps before the fast path.
- Added a regression test that a solid 1024×768 panel is one quad.
- Verified: `cargo test --workspace` 256 tests pass, 4 ignored; workspace
  clippy is clean. Capture: `/tmp/opencode/ui_live_fast.png`.

## 2026-11-21 — Engine: 1080p 60 fps pass

- Terrain albedo no longer does the per-fragment multi-octave noise stack:
  the cache grass sprites, vertex biome tint and the existing classification
  carry the ground. Fog banks and grass normal noise were removed, and the
  sun shadow PCF went from four taps to two.
- Dynamic unit animation is culled to the camera view radius plus a 2,000-unit
  margin (5,000–14,000 depending on distance and pitch); the remaining units
  animate into a reused face-colour cache, per-unit pose scratch buffers and
  an sRGB lookup table instead of paying HSL and `powf` per vertex.
- MSAA 4 already resolves edges, so the post pass now skips its full-screen
  FXAA taps on that path; `--msaa 1` still uses FXAA.
- Static model and water chunks now distance-cull at 16,000 units, scenery
  blob shadows at 14,000 and static shadow casters at 9,000. Terrain still
  draws to the horizon. `--draw-distance` overrides the model/water value.
- Added `--window-size WIDTHxHEIGHT`, `--render-scale auto|0.5..1.0`, and a
  five-phase benchmark split (`sim`, `mesh`, `hud`, `ui`, `submit`). `auto`
  targets 1.4M scene pixels and leaves post and HUD at native resolution.
- Measured on the Ryzen 5 6600U / Radeon 680M, release, immediate present,
  MSAA 4 and sun shadows on:
  - 1280×720: p50 **9.30 ms**, p95 10.44 ms, 106.8 fps (scene 1.00×).
  - 1920×1008: p50 **13.22 ms**, p95 14.77 ms, 75.2 fps (scene 1578×828,
    0.82×; HUD native).
  - Before this pass the same 1920×1008 run was p50 61.5 ms.
- Native 1920×1008 with `--render-scale 1.0 --msaa 1` remains around
  p50 17.2 ms (58 fps); the auto scale is what buys the 60 fps margin on
  this iGPU.
- Verified: `cargo test --workspace` 255 tests pass, 4 ignored; workspace
  clippy is clean. Captures: `/tmp/opencode/perf1080_final.png` and
  `/tmp/opencode/draw16k.png`.

## 2026-11-21 — Cache-native item icons and expanded item pool

- Added `crates/cache/src/item.rs`: the item definition decoder reads the
  inventory model, zoom, rotations, offsets, resize, light/contrast and
  palette swaps from config archive 10. `rift-tools item <id>` prints them.
- Added `crates/client/src/item_icons.rs`: each item's inventory model is
  rendered to a 32 by 32 OSRS-style icon with its recolours, scale and
  rotation, plus the one-pixel dark edge.
- `theme_file::load_with_items` bakes an `icon.item.<key>` role for every
  item in `data/items/items.json` that has a cache `obj` id. The stage 4
  inventory and shop rows draw those cache icons; the colour/initial remains
  only as a missing-asset fallback.
- The item pool grew from 6 to 89 entries: bronze-to-dragon melee, shortbows
  to twisted bow, air staves to Tumeken's shadow, platebodies and robes,
  rings, amulets and boots from early to end game. Every entry carries its
  real OSRS cache item id.
- The live shop list now scrolls with the mouse wheel and resets on category
  change; the footer reads the live gold and the item rows carry live icons.
- Added the `text.dim` OSRS palette role and used it for objective detail
  lines instead of a raw colour.
- The old HUD's legacy shop grid still shows the first six entries of a
  category; the full list is browsable in `--ui-live` until that grid retires.
  Its affordability table is now sized to the whole pool, so the extra items
  cannot index past the old six.
- Verified: `cargo test --workspace` 253 tests pass, 4 ignored; workspace
  clippy is clean. Capture: `/tmp/opencode/ui_live_osrs_final.png` (six
  categories, cache-baked icons, live gold and the scrolled melee list).

## 2026-11-21 — Live shop capture and clean layer

- Captured `--ui-live --demo-shop` at 1280×720: the six category rows, live
  item list and detail, gold `413`, and the assembled inventory's Shop button
  all come from the match.
- While the assembled interface is live, the old HUD no longer draws its own
  shop window or tooltip. The duplicate old tooltip that floated over the new
  item rows is gone; the old HUD remains for the components not yet replaced.
- Verified capture: `/tmp/opencode/ui_live_categories_clean.png`.
- Tests are unchanged at 250 passing, 3 ignored; workspace clippy is clean.

## 2026-11-21 — Live shop categories

- `ShopState` now carries the category rows and the selected index instead of
  a single category label. The stage 3 sample still shows its one category;
  the live bridge fills the six item categories.
- The live shop overlay draws the category rows down the left panel, highlights
  the current one and hit-tests them above the item rows. Choosing a category
  re-filters the list and selects its first item, matching the old HUD.
- Verified: `cargo test --workspace` 250 tests pass, 3 ignored; workspace
  clippy is clean.

## 2026-11-21 — Assembled inventory opens the shop

- The live interface now caches the `Layout` from its draw pass, so click
  routing uses the rectangles that were actually drawn rather than rebuilding
  them from the sample state.
- The assembled inventory's Shop button is hit-tested with the same geometry
  that draws it (`inventory::shop_button` and `shop_button_at`). Clicking it
  opens the live shop overlay; clicks elsewhere on the inventory panel are
  consumed so they do not fall through to the old HUD or the world.
- Verified: `cargo test --workspace` 250 tests pass, 3 ignored; workspace
  clippy is clean.

## 2026-11-21 — Live inventory and gold

- The stage 4 inventory is bridged: its six slots now come from
  `ItemShop::owned`, in slot order, with each item's live name and colour.
  Empty slots stay empty, and an owned index that no longer resolves is
  skipped rather than drawn as a broken item.
- The inventory's gold line and the shop overlay's footer now read
  `PlayerHud::gold`. Buying through the new shop overlay updates both on the
  next frame.
- The shop footer hint now says "Click a row, then Buy." instead of claiming
  buying still lives in the old HUD.
- This completes the stage 4 data bridge: every `HudState` slice is filled
  from the match. The old HUD is still the running default while click
  routing and final parity are finished.
- Verified: `cargo test --workspace` 249 tests pass, 3 ignored; workspace
  clippy is clean.

## 2026-11-20 — Interface stage 3 assembled

- Merged the interface stage 3 branch (steps A, B and 1 through 15) onto the
  M5.6 client and map. The stage 3 work is in `crates/ui`: a cache-backed
  theme, scale by fit, the placing/collapse pass, every component, the
  assembled whole, and the review sheet in `docs/INTERFACE_STAGE3_REVIEW.md`.
- `--ui-theme-sheet` draws every role; `--ui-layout` draws the assembled
  sample interface over the game. Stage 3 was accepted at the merge of PR
  #1. The old `crates/client/src/hud.rs` and its 3D unit bars remain the
  running default until stage 4 connects each component to the match.
- The interface owns its own pass in `crates/client/src/ui_pass.rs`; the
  theme loader is `crates/client/src/theme_file.rs`. Both sit beside the
  M5.6 HUD pipeline and the 117HD renderer.
- The item list scroll, paragraph wrapping and outlined text came across with
  the stage 3 work. The cache keeps one sprite decoder: the RGBA
  `SpriteDefinition` used by the texture path now reads through
  `SpriteSheet`.
- Headless evidence: `target/stage3-whole-1920.ppm`,
  `target/stage3-whole-1366.ppm`, `target/stage3-death-1920.ppm` and
  `target/stage3-shop-1920.ppm`, written by an ignored client test.
- Stage 4 has started with the score strip, ability bar, minimap, objectives,
  unit plates, chat, kill feed, shop, announcements and death overlay:
  `--ui-live` draws the assembled interface with the clock, both teams'
  kills, the player's K/D/A and creep score, health, resource, level,
  experience and cooldowns from `PlayerHud`; the live lanes, river, unit
  dots, structures, camps and camera box on the minimap; the lane
  tower/inhibitor pips, nexus states and camp timers in the objectives
  panel; the already-projected hero, minion and structure plates over the
  battlefield; the match event log, recent kills, announcements and the
  respawn/killer death lines. Neutral camp plates stay with the old HUD. The
  feed only carries allied kills, and its gold is derived from the victim
  type because the match does not record per-kill gold. The death panel now
  shows the real damage breakdown: damage to the player in the last ten
  seconds is grouped by source (rival champion, tower, minion, camp,
  watchtower or a team fallback) and sorted largest first. The shop tooltip
  follows the selected item and the live cursor while the shop is open. The
  new shop overlay now takes clicks: a row selects its item, Buy purchases
  the selection through the same item path as the old HUD, and the close
  button closes the shop; category switching still belonged to the old HUD at
  that point. The remaining components stayed on the stage 3 sample until
  their own bridges were added; the last of them, inventory and gold, are in
  the latest entries.
  Team frames are bridged too, but the current match is one hero a side, so
  they stay empty until allied champions are spawned. Clicking a unit now
  selects it and fills the target panel with its live name, kind, health,
  description and combat figures; clicking the ground clears the selection.
  The conversions are in `crates/client/src/ui_live.rs`, with the damage log
  in `AnimatedScene`, and are unit tested.
- Verified: `cargo test --workspace` 248 tests pass, 3 ignored; the client
  and tools build; the stage 3 layout tests, live score/ability/minimap/
  objectives/plates/chat/shop/tooltip/announcement/death/team-frame/target
  tests, sim battle tests and cache decoders all run.


## 2026-11-19 — Lane readability

- Enemy tower range is drawn as a ground ring whenever the player is within
  1.35× its attack range. It brightens when the tower is actually targeting
  the player.
- Enemy minions that have the player as their attack target get a red aggro
  ring, so lane pressure is visible instead of inferred.
- Enemy minion health bars turn **white** when one auto-attack would kill
  them, giving a clear last-hit window.
- Verified capture: `/tmp/opencode/tower_danger.png` (red tower range).
  Tests 61, clippy clean, client ~54.5 fps at 1280×720.

## 2026-11-19 — Cache-scraped grass biome variants

- Added `rift-tools dump-textures`: exports all 208 index-9 textures and,
  with `--sprites`, all 6,839 readable index-8 sprites into
  `assets/cache-scrape/` with CSV metadata.
- Confirmed the current ground grass is cache texture **129**.
- Added six grass variants: classic 129, lush 203, dry 18, steppe 197,
  dire 195, straw 205. `map.json` biome entries now carry a `grass` field;
  dire pockets use dire/dry/steppe, radiant pockets use lush, and default
  lanes stay classic.
- Grass is loaded into a dedicated **512px mirror-tiled array** (pattern
  repeats every four tiles with no mirror seams), sampled with nearest/LOD0
  so the scraped blades stay crisp. Classic ground borrows lush as a sparse
  detail pass.
- The post grade was eased and the terrain sampler switched to nearest so
  scraped OSRS greens survive the tonemap.
- Verified captures: `/tmp/opencode/grass_hires.png`,
  `/tmp/opencode/grass_hires_lush.png`, `/tmp/opencode/biome_goblin.png`,
  `/tmp/opencode/biome_elven.png`, `/tmp/opencode/grass_compare.png`.

## 2026-11-19 — Core-loop feedback: deaths and rewards

- Last-hit gold is now visible: every player kill pushes a `+NG` float above
  the victim (minion, camp, tower, inhibitor, champion, nexus), on top of the
  existing damage numbers and kill feed.
- Added a proper death state to the HUD: dimmed screen, `YOU HAVE BEEN SLAIN`,
  a live `RESPAWNING IN N` countdown fed from the player's respawn timer, and
  `THE FOUNTAIN AWAITS`.
- While dead, the player's ability bar, inventory, shop/deliver buttons, health
  orbs and hero profile card are hidden; the duplicate centre announcement is
  suppressed so the death card owns the screen.
- `--demo-death` kills the player after one second so the overlay can be
  inspected without relying on a random fight.
- Verified captures: `/tmp/opencode/death_overlay2.png`,
  `/tmp/opencode/gold_camp.png`; client benchmark back to **p50 16.7 ms**
  (~60 fps), tests 61, clippy clean.

## 2026-11-19 — Focus correction: main-game spectator

- Removed the experimental client sim overlay and standalone 3D sim arena
  (`--sim-overlay`, `--sim-3d`, `--sim-control`). They were a detour from the
  core game loop. The headless sim crate and `rift-tools match` remain the
  server-spine work, but the client is back on the real match.
- Spectator mode is now a camera swap over the main game's champions:
  - `V` toggles spectator mode; `Z`/`X` cycle cameras;
  - `--spectate` starts attached to the local champion;
  - `--spectate-index N` starts on a specific champion slot (verification).
- While spectating: camera follows the chosen champion, the HUD shows name,
  level, team, HP and mana, the player's ability/inventory/health cluster is
  hidden, click-to-move and QWER input are disabled, and the hero profile card
  is suppressed so the view is clean.
- Restored the per-frame `animated.refresh_visibility(dt, collision)` call
  that was lost during the cleanup; the client is back to **p50 17.4 ms**
  (~58 fps) instead of the accidental 26 ms regression.
- Verified capture: `/tmp/opencode/spectate_clean.png` (rival champion camera,
  spectator bar, clean HUD). Tests 61, clippy clean.

## 2026-11-19 — M7 slice 4: 3D sim arena bridge

- New `crates/client/src/sim3d.rs`: a standalone 3D spectator arena that
  steps the authoritative `rift_sim::Match` and renders it with cache models.
- Arena: a generated 64x64 platform with three lane strips, placed at world
  origin so it cannot be confused with the live map. Camera is a dedicated
  top-down spectator view; the prototype unit mesh and HUD are suppressed.
- Actors: King Awowogei (blue hero), Kree'arra Jr. (red hero), OSRS imp
  minions, and tower/inhibitor/nexus cache models, with team-coloured ground
  rings. Missing models fall back to coloured boxes.
- `--sim-3d --sim-seed N` runs the arena at fixed 30 Hz with the same
  deterministic `Match` as the CLI. Minimal HUD shows tick, seed, state hash,
  hero counts, wave timer and the winner.
- This is a dev bridge, not the final renderer: the prototype map still owns
  live gameplay. The next slice replaces that with this actor layer, animated
  poses, health plates and local movement prediction.
- Verified capture: `/tmp/opencode/sim3d7.png` (tick 795, three lanes, team
  rings, structures, hash `fe0273656458b7f7`). Workspace tests 61, clippy
  clean.

## 2026-11-19 — M7 slice 3: client consumes the sim

- `rift-tools match --trace-every N` prints `tick hash` lines, so a run can
  be compared externally without dumping all entities. Seed 7 traces are
  identical across runs (1000/2000/3000… ticks).
- Client gained a **SIM SPECTATOR** dev overlay (`--sim-overlay`,
  `--sim-seed N`): it runs the real `rift_sim::Match` on a fixed-step
  accumulator and draws the three lanes, structures, minions, heroes, nexus
  health, wave timer, live tick and `last_tick_hash`.
- The overlay is the first client consumer of the authoritative sim. It does
  not replace the 3D actor state yet; that is the next slice (map sim entities
  onto cache models, keep local movement prediction).
- Verified: overlay at tick 1540 shows blue/red structures, 8 minions, hero
  health bars and hash `3214a38435b7f6bc`; trace runs and the CLI summary agree
  with the sim tests. Workspace tests stay at 61 and clippy is clean.

## 2026-11-19 — M7 slice 2: lanes, waves, structures, economy

- The headless match is now a real three-lane game: top/mid/bottom waypoint
  lanes, a scaled arena with jungle blocks, and lane assignments for both
  hero teams.
- Minion waves spawn every 25 s (first at 10 s), three per lane per team,
  capped at 90 live minions per side.
- Structures: one nexus, three towers and three inhibitors per team. Towers
  and inhibitors shoot; the nexus is shielded until at least one enemy
  inhibitor falls, then becomes destructible.
- Minions advance their lane, stop to fight anything within 3 tiles, and deal
  double damage to structures. Heroes have an 8-tile aggro radius, otherwise
  push their lane and fall back to the enemy nexus at the end.
- Hero economy: gold and XP on minion/structure/hero kills, level-ups increase
  HP and damage. Heroes respawn at base after 15 s.
- Match ends on nexus destruction; a 20-minute integer-only timeout compares
  nexus health fractions if neither side closes.
- `last_tick_hash` is written after every step, so per-tick determinism can be
  asserted (the 600-tick test compares every tick).
- `rift-tools match --seed 7` finishes at tick **6986**, team 0 wins, hash
  `bc1f32e1b0f15ace`, 9 waves. Default is 12000 ticks and `--verbose` prints
  every entity. Seed 1 also finishes before the timeout.
- Tests: 14 in `rift-sim` (waves interval, guaranteed nexus ending, shield
  rule, per-tick hash equality, commands, integer roots). Workspace total 61.
- Remaining M7 work: move the client's lanes/combat/economy onto this `Match`,
  add a state-hash trace mode and make the client a renderer over the sim.

## 2026-11-19 — M7 slice 1: headless match spine (superseded by slice 2)

- New `crates/sim/src/battle.rs`: a 5v5 bot match running at exactly **30 Hz**
  (`TICK_HZ`, `TICK_MS`) with no wall-clock time, no OS RNG and no floats in
  movement. Positions are fixed-point integers (1024 units per tile) and
  distances use an integer square root.
- Seeded xorshift RNG drives damage variance and 10% crits; the RNG state is
  part of the hash.
- Input commands: `MoveTo`, `Attack`, `Hold`, queued at tick boundaries.
- Bot AI picks the nearest enemy, chases with the existing A* grid (central
  wall + pillars force real pathing), and auto-attacks in range.
- Stable FNV-1a `state_hash()` covers tick, winner, stats, RNG state, every
  entity and its order/path.
- `rift-tools match` runs headless without opening the cache; slice 2 replaced
  the duel arena with the three-lane map below.
- Tests at that slice: 12 in `rift-sim`. Workspace total was 59.

## 2026-11-19 — M6.4 HUD skin pass

- Added a shared three-step HUD type scale (`HUD_TITLE_SCALE`,
  `HUD_BODY_SCALE`, `HUD_SMALL_SCALE`) instead of one-off font sizes.
- Card titles now use the gold palette and a thin carved rule: objectives,
  enemy team, chat, debug stats and the scoreboard underside.
- Abilities and inventory slots share one beveled `slot_frame`: dark inset,
  warm outer edge and top-left highlight. The old full colour block behind
  ability icons is gone; ready slots now carry a class-coloured underline.
- Shop and Deliver buttons use detached shadows, hover highlights, gold
  outlines and disabled colours. The shop button flips to `CLOSE [B]` while
  the window is open.
- Confirmed the Fountain Shop already shares the same panel sprite, close
  control, tabs and item-slot language; it now matches the in-game HUD.
- Verified captures: `/tmp/opencode/skin_hud.png`, `/tmp/opencode/skin_shop.png`.

## 2026-11-19 — M6.3 ability VFX pass

- Added a deterministic cosmetic particle system (capped at 320 live sparks)
  with gravity, drag, ground bounce and team-vision filtering.
- Projectiles now leave trails; dart hits leave a green burst and a short
  poison stain, bolt hits leave an orange burst.
- Bananarang leaves a gold trail and emits an impact burst on each hit.
- Royal Dash emits cyan bursts at both ends and a second arrival ring.
- King's Decree emits rising green sparks and a healing ground stain.
- Trial by Combat bursts around the new arena.
- R meteor is now telegraphed: a pulsing danger circle plus a visible falling
  rock precedes a 0.9 s delayed impact, which then spawns a shockwave ring,
  40 sparks and a scorch decal. Damage is applied by the impact, not on cast.
- Pending impacts drive a warm point light that intensifies as the rock falls.
- Verified visually: `--champion jad --demo-kit` at frames 185/200/250 shows
  the telegraph, fall and impact; `--demo-combat` shows projectile trails.
  `cargo test --workspace` and workspace clippy stay clean.

## 2026-11-19 — M6.2 unit frames + hit feedback

- Screen-space unit plates projected from the world: champion name, level,
  HP and mana; structures and camps get name + HP. Plates de-overlap by
  nudging lower-priority frames upward.
- World-space HP bars remain only on lane minions and jungle camps, so
  champions and structures no longer double-draw.
- Damage numbers carry a per-hit scale (`0.9..1.8`) and a one-pixel shadow;
  large hits read at a glance.
- Committed player attacks show a `CAST` bar above the bottom cluster.
- Ability cooldowns use a clockwise radial wedge instead of a vertical fill.
- Taking damage triggers a red screen-edge vignette and a short camera shake;
  both decay over ~0.4 s.
- Fixed the unused `id` warning in `rift-tools`; workspace clippy is clean.

## 2026-11-19 — presentation & terrain pass (M6.1)

- Camera default moved to the reference framing: 3200 units at 0.95 rad
  (~54°), so heroes read large and the map reads as a place.
- Scoreboard strip now shows only blue kills, match clock, red kills, personal
  K/D/A and CS. Personal kill/death/assist and creep-score counters were added
  to the match state and only credit damage attributed to the player.
- Right edge is a five-slot enemy team card (portrait chip, level, HP, MP);
  left edge is a single objectives card with team comparison, nexus bars and
  wave/buff readout. The engineer stat sheet is behind `--debug-hud`.
- Minimap carries a level/XP/gold strip along its bottom edge.
- Terrain conditioning: `relax_heights` removes single-tile rock spikes,
  `add_rolling_relief` adds two octaves of sub-tile rolling relief so flat
  fields are not tabletops, and `flatten_bases` levels each courtyard to its
  centre height with a soft skirt. This fixes the "rocky or too flat" ground.
- Grass shader: micro-relief normal perturbation, directional blade streaks,
  stronger fine speckle and slightly higher saturation so the field reads as
  grass at the new camera distance instead of a flat wash.
- Verified: `cargo test --workspace` green (14+33+7); debug benchmark at
  1280×720 immediate p50 16.4 ms / p95 17.1 ms (~60 fps). Screenshots captured
  for field and base views.
- Note: an experimental flagstone courtyard overlay was tried and reverted; it
  fought an unresolved depth artifact. The levelled grass base with the stone
  fountain ring is the current base look.

## Working

- Rust workspace builds; `cargo test --workspace` and `cargo clippy
  --workspace --all-targets` are clean.
- Cache reader: all 25 indices, reference tables, XTEA, gzip/bzip2/LZMA,
  archive splitting with real file IDs.
- Terrain/locations/object/NPC/underlay/overlay configs.
- Model formats 2 and 3 with flat-shading normals, vertex groups, texture refs.
- Cache texture pipeline: index 9 texture definitions resolve to index 8 sprite
  archives; 208 sprites decode to a mipmapped 256-layer texture array.
- Sequence/frame/framemap decoding and interpolated CPU vertex animation.
- `crates/sim`: deterministic 192×192 tile grid, A* with octile heuristic,
  no corner cutting, nearest-walkable fallback, path simplification, LOS.
- Scene: 5×5 Karamja block (radius 2), 614k terrain verts, ~1500 scenery
  placements, 1200 cache-native ground decorations (flowers, ferns, grass,
  mushrooms, rocks, twigs, roots and stumps), 8 ambient animated NPCs, three
  dirt lane ribbons, distant backdrop plane.
- Biome pockets: eight real OSRS regions transplanted into the map (Wilderness
  dead trees, Morytania swamp, ogre ruins, goblin camp on the dire side;
  Varrock, human keep, Tirannwn elven woods, gnome stronghold on the radiant
  side), each with its own ground tint and scenery.
- River: a diagonal waterway from the upper left third through the centre to
  the lower right third, carved into the terrain, blocked for movement except
  at the centre bridge.
- Lane tree lines: two rows of OSRS tree models flank every lane, tinted by
  the side of the river they stand on.
- Jungle: eight neutral camps with proximity aggro, chase, leash reset and
  respawn; Might (+25% damage) or Swift (+20% speed) team buffs for 60 s.
  Camps are revealed by proximity (1500 units) regardless of brush or tree
  line of sight, so they never ambush the player from a hidden tile. The eight
  scattered critters are weak camps too (220 HP, 14 damage, 420 aggro). The two
  camps that accidentally used a table prop model now use creature models
  (General Graardor Jr. 27660 and Zilyana Jr. 27989).
- Ground textures: lush green-boosted grass, stable world-space planar UVs,
  contact darkening instead of a projected cliff overlay, and two noise scales
  for natural break-up. Lane roads use cache soil textures and are suppressed
  under the river. Dead and snowy trees use bark on every face.
- Presentation: HDR `Rgba16Float` scene target with a real bloom chain
  (bright pass, separable Gaussian, ACES tonemap), cache-native ground
  texture array with world-space UVs, two-sided lighting with a camera fill,
  and a fog of war that shadows unexplored ground instead of tinting it.
- Fog of war: a shader-side horizontal distance circle around the hero
  (1500 units, 0.80R..1.30R soft edge, 42% brightness outside). No fog mesh,
  so it never fights with terrain z levels and the boundary is a consistent
  lateral radius at every height.
- HUD: OSRS palette text at 1.32x, Dota scoreboard strip (kills, deaths,
  clock), cursor tooltip for units, minimap viewport box, biome label.
- Minimap: a baked 256x256 terrain map (biomes, river and lanes) uploaded to
  the UI atlas, team hero markers with a hero profile card (portrait, name,
  level, HP/MP), structure dots for both teams, and a camera viewport box.
  Enemy heroes are never shown; enemy mobs only while inside team LOS.
- Lanes: baked into the terrain as a mask in the vertex alpha; the shader
  blends the OSRS dirt/path sprites over the ground, so there is no ribbon
  overlay, no z-fighting and the road edges wobble naturally.
- Watchtowers: six OSRS "Battle tower" structures (three per team) with a
  620 unit acquire radius and a 1200 unit lock range, firing slow 280 u/s
  bolts at heroes, minions and jungle camps until the target dies or leaves
  the lock range.
- Castle wall ruins: broken OSRS walls and ruin clusters line the outside of
  every lane.
- Scattering: trees, ruins and ground decor are placed by a fixed-seed
  xorshift RNG with variable spacing, offsets, scales, species and clearings,
  so the world is varied but identical every match. Ground decorations are
  selected from real OSRS object locations and sorted by lane proximity so
  the detail follows the play area. Dire species are dead and dark, radiant
  species stay green. Pocket props are kept 6.5 tiles clear of every lane.
- Visibility: structures (towers, inhibitors, nexus, watchtowers) are always
  visible; projectiles are only drawn inside the team's line of sight.
- Movement: click-to-move A* path following; hold-to-steer with collision
  sliding; camera follow/pan; selection and click rings; minimap click-to-pan.
- Collision: only rendered props block movement (base scenery, pocket props,
  lane trees and ruins), so there are no invisible walls; the pathfinder snaps
  start/goal up to 20 tiles so a hero pushed onto a blocked tile can escape.
- Selection/camp/structure rings sample the height grid per vertex, so the
  bands follow slopes instead of clipping through them. Scenery props keep six
  tiles clear of every camp and structure.
- Shop: purchases work anywhere (the fountain still heals and respawns).
  Remote purchases go to the storage chest beside the team spawn; a HUD button
  dispatches the flying Armadylean guard, which delivers the stored items to
  the hero and returns to its post. The shop marks waiting items `IN TRANSIT`.
- Delivery: chest model 1249 with a gold ring while loaded; guard model 28019
  (Kree'arra Jr., the closest Armadylean model in this cache) at 1100 u/s.
  `--demo-delivery` verifies buy-away-from-base -> dispatch -> handover.
- Lane: three-minion waves per lane every 25 s, now composed of cache-native
  imps (melee) and fairies (ranged), spawned in a lane-relative wedge with
  parallel lane tracks and staggered spawn delays. Small team rings and
  wider health bars read the two sides at a glance.
- Inhibitors: destroyed inhibitors respawn after 120 s. While an enemy
  inhibitor is down, that lane's waves include a super minion (2.5x health,
  2x damage, 1.6x scale, gold minimap marker). Respawn re-shields the nexus.
- **Castle-boxed corner map**: the playable area is a 282×282 tile square
  enclosed by a white crenellated castle wall with towers and gatehouses.
  Team 0's base sits in the bottom-left corner and team 1's in the top-right;
  both are 28-tile courtyards with gates, corner towers, a blue nexus dais,
  and white statues. Three lanes run diagonally between the corners and cross
  the diagonal river at three bridge points. The player spawns inside the
  bottom-left courtyard.
- Champion roster: four data-driven configs in `data/champions/`:
  - **Blowpipe Ranger** — ranged marksman, 420 HP / 200 mana, 99 ranged, OSRS
    torso (314) with the hood (221), arms (26619), legs (258), feet (185)
    and a blowpipe (176) merged in. Q **Toxic Dart** (30
    mana, 5 s) fires a fast dart that poisons; W **Dart Volley** (50 mana, 12 s)
    fans five poisoned darts; E **Ranger's Footwork** (25 mana, 14 s) hops
    backwards and grants +40% attack speed for 5 s; R **Deadshot** (80 mana,
    75 s) is a piercing shot that deals +35% against targets under 25% HP.
  - **Monkey** — balanced, 500 HP / 200 mana.
  - **JalRek-Jad** — tank/mage, 700 HP / 260 mana, higher damage.
  - **Kree'arra Jr.** — ranged skirmisher, 420 HP / 240 mana, ranged auto-attacks.
  Load with `--champion monkey|jad|kree`. A normal launch opens the hero-select
  screen; `--champion` skips it for demos and direct matches.
- **Hero select (M5.19)**: full-screen OSRS-style champion picker with baked
  cache-model portraits (CPU rasteriser, idle pose), role/tagline cards, stat
  bars, click/keyboard selection, and a gold lock-in button. Selecting a hero
  rebuilds a fresh match scene so skills, stats and cooldowns start clean.
- **Nexus/inhibitor assets**: the painted blue crystal is replaced by real
  cache models — Portal Nexus (36053) for both nexuses, Crystal of Power
  (12953) for radiant inhibitors, and Crystal Outcrop (9721) for dire
  inhibitors, with team tinting.
- Kit: Q projectile, W blink, E heal+shield, R meteor; mana, cooldowns, shield
  absorption, expanding rings; ability power from items scales Q/R/E.
  King Awowogei overrides Q and R:
  - **Bananarang (Q)**: a fixed rounded-trapezoid skillshot that leaves his
    left hand 30 degrees left of facing, runs across a long top face, and
    closes back into his right hand. A centripetal Catmull-Rom spline keeps the
    heading changing in gentle arcs, and the return leg eases toward the king's
    current right hand if he moves after throwing it. Each enemy is hit once,
    with 60% physical / 40% pure damage, a rank scaling mini-crit chance, and a
    short crit stun. Forward depth is 1,300 world units (furthest top corner
    ~1,516, about 11.8 tiles). Cache model 2387 at 6.25x scale flies the path in
    about 3.33 s (30% of the original speed).
  - **Trial by Combat (R)**: a champion-only unit-target ult that forms a
    barrier where both fighters stand, locks both inside, blocks tower fire
    against them, and spawns eight monkey subjects who throw banana peels every
    0.9 s. Peels stack -2 armor (max -16) for 4 s. Killing the target within the
    10 s trial grants +4 permanent base melee damage per win; the barrier ends
    early if either fighter dies. Cast range is 75 tiles.
  Jad and Kree keep the shared Q bolt and R meteor.
- **Match start**: decoded models and animation clips are memoised per id
  (`UnitAssets`) and shared behind an `Arc`. A scene builds ~160 units out of
  a handful of distinct assets, and re-decoding one clip costs about 100 ms,
  so the repeats used to stall a match start for ~30 s. The same scene now
  loads in about 2.5 s.
- **Combat feedback**: floating damage numbers over the unit that was hit, a
  yellow centre-screen kill shout ("YOU HAVE SLAIN A MINION!"), a kill feed
  under the roster panel, a red attack-range ring and a yellow target ring on
  the ground, and a red minimap ping on the last enemy hit. `--demo-fx` pops all
  of them for a quick visual check.
- **5v5 lobby**: after locking in, the reference lobby shows both lineups as
  two rows of five hero cards (portrait, name, class, HP/MP/RNG), a RADIANT and
  DIRE tab down each side, the player's pick highlighted in gold, and the red
  "LOADING - PLEASE WAIT..." button that drops the teams into the match.
- **Hero select**: the reference layout - filter tabs (ALL / MELEE / RANGED /
  MAGIC), a roster grid of hero cards with baked portraits, hotkeys and class
  tags, a "Your heroes" list on the left, a "Rival heroes" list on the right,
  and a detail panel with the hero's stats plus its Q/W/E/R names and
  descriptions, and a gold LOCK IN button. Champion classes and ability
  tooltips are data-driven in `data/champions/*.json`.
- **OSRS client HUD**: an OSRS-style stat panel (Attack, Strength, Ranged,
  Prayer, Hitpoints plus Run-Attack, Ranged-Bonus, Melee-Def and Ranged-Def),
  a two-team roster panel with live health bars, an OSRS-style chat box with
  coloured lines, and a prayer orb beside the status bars.
- **Ranged damage**: champions with a `ranged` level scale ranged damage by
  `1 + ranged / 100`; venom, piercing shots, executes and attack-speed buffs
  are new unit states.
- Armor: every champion has a real armor stat (Monkey 30, Jad 40, Kree 18,
  Ranger 16).
  Physical damage is scaled by `100 / (100 + armor)`; negative armor amplifies
  damage. Pure damage bypasses armor. The HUD team panel shows ARM and DMG.
- Demos: `--demo-duel` spawns a rival champion beside the hero for manual
  Trial/Q testing. `--demo-trial` spawns a wounded rival, casts R then Q, and
  logs the duel result, win count, bonus, player HP and target HP.
- Progression: levels 1–11, XP, gold; level-ups raise health, mana, damage.
- Economy/base: passive gold 8/s; fountain heals 6% max HP/MP/s within 900
  units of spawn; two persistent rings mark the fountain.
- Item shop: 89 data-driven OSRS items across melee, ranged, magic, armour,
  rings, amulets and boots; six inventory slots, fountain-range purchase,
  gold spending, stat/ability-power application, hover tooltips, and
  cache-baked inventory icons.
- **Terrain smoothing/detail (M5.3a)**: the per-tile color field is box-blurred
  twice and assigned to shared grid-corner vertices; `scene.wgsl` adds
  procedural ground detail for terrain. Tile outlines are gone.
- **Post pass (M5.4a)**: scene renders to an offscreen sRGB target; `post.wgsl`
  applies FXAA, cheap bloom, gamma-space color grading, and dithering. HUD
  renders after post, so text/icons stay crisp.
- **OSRS-style UI**: centered Fountain Shop window with category tabs, item-card
  grid, detail pane, Buy/Sell buttons, inventory row, gold, close button, and
  real item icons/tab sprites from the gitignored `assets/osrs-ui` tree. HUD
  resource bar, team panel, objectives panel, event log, compact health/mana
  bars, bottom-right inventory, and shop button. Textured `ui.wgsl` pipeline
  with a 1024×1024 atlas; procedural fallback if assets are missing.
- HUD font: built-in 5x7 uppercase bitmap font; HP/MP/XP/gold, nexus values,
  wave timer, ability labels/cooldowns, shop names/costs/tooltips, result.
- **117HD Core renderer**:
  - `SceneVertex` carries position, normal, colour, UV, AO, and material.
  - sRGB→linear conversion on the CPU; sRGB target handles final encode.
  - Smooth terrain normals; flat per-face model and unit normals.
  - Sun + hemispheric ambient lighting, optional 4-band stepping.
  - Sky gradient, procedural clouds, sun disk/glow, horizon haze.
  - Exponential height fog and ground haze.
  - Static scenery blob shadows + per-unit blob shadows, plus a static
    sun-shadow caster mesh for trees, ruins and pocket buildings.
  - Up to 16 dynamic point lights: fountain, Q muzzle/impact, E/R effects,
    projectiles.
  - **Sun shadow map**: 1024px depth map, 60x60-tile orthographic frustum, PCF
    via comparison sampler, dynamic player/structure/camp casters, alternating
    frame updates. `--shadows off|blob|map`.
  - **Contact AO**: baked terrain cavity AO and model/unit base AO.
  - `--graphics 117hd|classic`,
    `--debug-view albedo|normal|lighting|fog|lights|shadow|ao`, `--pitch`.
- Fog/brush: 192×192 visibility grid; hidden enemies/neutrals; dark fog mesh;
  minimap draws 8×8-tile fog blocks and hides unseen units.
- HUD: health/mana/XP/shield/buff bars, four ability boxes, six inventory slots,
  shop panel, nexus bars, wave timer, crosshair, fogged live minimap.
- Dev demos: `--demo-target`, `--demo-combat`, `--demo-camp`, `--demo-kit`,
  `--demo-shop`, `--demo-base`, `--demo-inhibitor`, `--champion`, `--timescale`.

## Verified

- `cargo test --workspace`: 54 tests pass (14 cache, 33 client, 7 sim).
- `cargo clippy --workspace --all-targets`: no warnings.
- 117HD Core runs at **60–68 fps at 1280×720** on the dev machine at timescale 1;
  shadow modes measured **67 fps (off)**, **68 fps (blob)**, **66 fps (map)**.
- Shadow debug view confirmed dark cast shadows under structures/units; AO
  debug view confirmed baked terrain/model contact darkening.
- A/B captures: `--graphics classic` flat/unlit baseline vs `--graphics 117hd`
  lit terrain/models, sky/clouds, fog, blob shadows, dynamic lights.
- Debug views confirmed albedo, normals, lighting, fog, and dynamic-light
  passes; Q impact shows an orange light pool next to the green fountain pool.
- Shop demo: 6/6 items, `dmg 45`, `HP 730/730`, `MP 260/260`, `spd 460`,
  `AP 45%`, `gold 300`.
- Base demo: 200/500 → 410/500 HP; passive gold 8 → 64.
- Inhibitor demo: 2/3 → respawn 3/3; nexus re-shielded; super minions spawned.
- Champion runs: Jad 700 HP / 260 mana; Kree 420 HP / 240 mana; W/E/R true.
- Model textures: 208 cache texture sprites uploaded to a 256-layer array;
  textured faces sample native leaf/wood/stone/flower art instead of flat
  fallbacks, with wood tree bases below the leaf canopy.
- Water: 9,712-triangle river surface with two animated ripple layers,
  fresnel sky reflection, sun specular and fog. Water is walkable and no longer
  an invisible collision wall.
- MSAA: the HDR scene target is 4x multisampled and resolves into the post/bloom
  chain; HUD remains single-sample and crisp.
- Static shadows: trees, ruins and pocket buildings cast into the 1024px sun
  shadow map alongside units.
- Combat presentation: jungle camp health slashed, critter camps are 150 HP /
  9 damage, camp bars are red and wider, and unit bars are pulled toward the
  camera to stop depth flicker while moving.
- Lane road: dirt-dominant, noise-warped 310-unit UVs remove the repeating
  cobble border while keeping the cache path sprite as a low-weight accent.
- Chunk culling now defaults to 24-tile chunks: 196 terrain + 112 model + 27
  water chunks (down from 400/228/49 at 16 tiles). Release 1280×720 immediate
  benchmark measured p50 11.98 ms / p95 13.62 ms with shadows + 4× MSAA at
  24 tiles, versus p50 12.49 ms / p95 14.17 ms at 16. The `--chunk-tiles`
  flag makes the trade-off directly measurable.
- Fog run: visible enemies 7–13; dark fog mesh over unseen tiles.
- Three-lane 20× run: victory states, towers/inhibitors/nexus live.
- Weighty champion movement: champion files carry `weight` (plus optional
  `turn_time`, `accel_time`, `stop_time`, `stride` and `bounce`). The player
  turns at a finite rate, accelerates and coasts to a stop, moves while
  turning, and waits until facing its target before auto-attacking. The walk
  cycle advances from distance covered instead of wall-clock time, so feet no
  longer slide while speeding up or slowing down. This was ported from the
  `MobaScape Movement System Branch` reference checkout.
- Animation pass: the player's walk clock is now adjusted by distance
  covered (the reference branch's `walk_ms_for` logic), and idle/walk poses
  cross-fade over roughly a tenth of a second instead of snapping. The
  `--demo-anim` mode loops King Awowogei through idle, eastward walk, turn and
  westward walk, coasting stop, and attacking the training dummy.
- Monkey walk baseline: `walk_sway: 0` removes torso side rotation and
  `walk_arm: 0` removes local arm transforms from the walk clip while leaving
  global body motion, head bob and leg swing intact. `walk_arm` accepts a
  0–1 blend so arm swing can be reintroduced and tuned without touching code.
  `--walk-sway` and `--walk-arm` override the champion file for quick tuning.
- Walk animation is now distance-driven for every mobile unit, not only the
  player. A central post-update pass replaces the clock's share of animation
  time with the distance each unit actually covered, so minions, camps, cows,
  crabs and the champion all keep their feet planted while speeding up,
  turning and stopping. Minions, camps and delivery guards also turn toward
  their heading at a finite rate instead of snapping on waypoint corners.
- Death feedback: non-structure units shrink and sink over 0.45 s when killed
  instead of popping out of existence, so last hits and camp clears read
  cleanly. Bars and selection rings disappear immediately.
- Attack commitment: champion files carry `attack_windup` (0.3 s Monkey/Kree,
  0.5 s Jad). The player turns to face before starting the swing, holds the
  pose for the wind-up, then lands the hit or fires the projectile. An orange
  charge ring grows around the champion during the wind-up so the commitment
  is readable.
- Ambient scenery creatures: 44 cache-coloured butterflies, 16 monkey-riding
  pasture cows around the map edge, and 28 shore crabs beside the river. They
  wander on their own schedules, are neutral, cannot be selected or damaged,
  never attack, and never show health bars. Crab NPC 4819 is unreadable in this
  cache revision, so the equivalent beach crab NPC 1553 (same models) is used.
- Cache groups compressed with type 0 (uncompressed) are decoded correctly:
  they carry a two-byte version trailer and no decompressed-length header.
  This unblocked butterfly sequences and other uncompressed assets.
- Tree lines now use only full-canopy models with bark trunks; bare tropical
  stems, stumps and dead-stem models are skipped so they cannot become solid
  green pillars.

- Jungle demo: camp killed, buff Might 58s.

## Known issues

- 117HD Core now has cache model textures, an HD river surface, 4x MSAA and
  static sun casters. It still has **no full SSAO, anisotropic filtering, or
  draw-distance streaming**. Terrain still uses pre-baked ground sprites in
  `assets/osrs-ground` rather than cache-native ground-layer blending.
- The post bloom is a cheap 8-tap screen-space glow, not a full HDR bloom chain.
- Static meshes are split into 16-tile chunks and frustum-culled, but there is
  no distance LOD or streaming yet; the map remains a resident 5×5 block.
- The shadow map covers dynamic units/structures/camps and large static props;
  small ground decorations still use cheap blob shadows. Chunked caster/LOD
  caching is pending.
- The shadow map is 1024px and updates every other frame; fast camera movement
  can show slight shadow lag.
- The 3×3 region block edge is still visible at low camera pitch. A distant
  backdrop plane and horizon haze hide it from the default camera, but true
  117HD draw distance needs chunked streaming/LOD.
- The fog-of-war mesh is an opaque dark plane. At low pitch it can read as a
  hard wall; it needs a soft/dithered screen-space treatment.
- Lighting is tuned for Karamja afternoon sun; other maps still need their own
  atmosphere presets.
- Dynamic lights are capped at 16 and have no shadows.
- Item purchases and gold/heal/shop-range remain client-side.
- No item selling/combining/passives; champions share the QWER shape.
- The HUD font is a code-defined 5x7 bitmap, not a cache-native OSRS font.
- UI assets live in gitignored `assets/osrs-ui`; without them the shop falls back
  to procedural icons and no tab sprites. The bronze sword icon is a trimmed
  detail render because the plain inventory icon file was unavailable under the
  expected wiki name. Faint baked tab numbers/icons can show through overlays.
- Kree'arra/Jad models are large; scale tuning is ongoing.
- Cache texture alpha cut-outs are rendered opaque; leaf and flower sprites are
  wrapped and colour-lifted rather than alpha-tested.
- Fog/brush is client-side only; server-authoritative fog is required.
- No netcode, audio, or map editor.

## Next tasks

1. **M5.2 117HD Water**: water mesh, fresnel, sky reflection, specular, caustics.
2. **M5.3b ground blending**: cache-native terrain layers, per-tile texture
   IDs, anisotropic filtering, and alpha-tested foliage cut-outs.
3. **M5.4b post + draw distance**: full SSAO, anisotropic texture filtering,
   chunked streaming/LOD, and model batching.
4. Static scenery shadow caching/chunked casters.
5. Item selling/combining/passives and five champions with distinct kits.
6. Server-authoritative fog, economy, and shop transactions.
7. Deterministic 30 Hz sim tick in `crates/sim` + state hash.
8. Engine performance follow-ups: see [ENGINE_ROADMAP.md](ENGINE_ROADMAP.md).

## Verification log

- `cargo test -p rift-sim`: 7 tests (straight path, wall detour, nearest
  walkable, no corner cutting, simplification, bounds, LOS).
- `cargo test -p rift-cache`: 9 tests (smart reads, chained smart, XTEA, HSL,
  group rejection, framemap/frame parse, sprite bounds, texture reference).
- `cargo test -p rift-client`: 24 tests (5 items/layout, 2 font, 2 minimap,
  3 hero-select layout/cycling, 2 portrait raster, 2 wave formation,
  2 lane offset/distance, 4 scene/lighting, 2 shadow matrix/defaults).
- Shop demo: 6/6 items, dmg 45, HP 730/730, MP 260/260, speed 460, AP 45%,
  gold 300.
- Base demo: 200/500 -> 410/500 HP; gold 8 -> 64.
- Inhibitor demo: 2/3 -> respawn 3/3; nexus re-shielded; super minions spawned.
- 117HD debug: albedo/normal/lighting/fog views render; dynamic lights show
  green fountain and orange Q impact pools; shadow/AO debug views render.
- Shadow modes: off 67 fps, blob 68 fps, map 66 fps at 1280×720.
- UI run: real item icons/tab sprites render; shop open at 58–60 fps.
- Ground: A/B FOW-disabled capture is seamless; continuous fog mesh removes
  per-tile grid lines.
- Post: FXAA/grade/bloom at 59–60 fps; HUD crisp after post.
- Champion runs: Jad 700 HP / 260 mana; Kree 420 HP / 240 mana; W/E/R true.
- Hero select: three cache-model portraits baked; mouse/keyboard lock-in
  starts a fresh match with the chosen champion; `--champion` skips the screen.
- Scene scatter: 1511 scenery placements plus 1200 ground decorations; tree
  lines scaled 1.35–2.05× and cache props 1.35–1.5× over the old sizes.
- Wave formation: initial waves spawn in lane-relative wedges with per-unit
  parallel lane offsets and 0.35 s stagger; the test verifies at least 220
  units between base minions and the offset-lane test verifies parallel tracks.
- Model textures: 208 cache texture sprites uploaded to a 256-layer array;
  textured faces sample native leaf/wood/stone/flower art instead of flat
  fallbacks, with wood tree bases below the leaf canopy.
- Water: 9,712-triangle river surface with two animated ripple layers,
  fresnel sky reflection, sun specular and fog. Water is walkable and no longer
  an invisible collision wall.
- MSAA: the HDR scene target is 4x multisampled and resolves into the post/bloom
  chain; HUD remains single-sample and crisp.
- Static shadows: trees, ruins and pocket buildings cast into the 1024px sun
  shadow map alongside units.
- Combat presentation: jungle camp health slashed, critter camps are 150 HP /
  9 damage, camp bars are red and wider, and unit bars are pulled toward the
  camera to stop depth flicker while moving.
- Lane road: dirt-dominant, noise-warped 310-unit UVs remove the repeating
  cobble border while keeping the cache path sprite as a low-weight accent.
- Chunk culling now defaults to 24-tile chunks: 196 terrain + 112 model + 27
  water chunks (down from 400/228/49 at 16 tiles). Release 1280×720 immediate
  benchmark measured p50 11.98 ms / p95 13.62 ms with shadows + 4× MSAA at
  24 tiles, versus p50 12.49 ms / p95 14.17 ms at 16. The `--chunk-tiles`
  flag makes the trade-off directly measurable.
- Fog run: visible enemies 7–13; dark fog mesh over unseen tiles.
- Three-lane 20× run: victory states, towers/inhibitors/nexus live.
- Weighty champion movement: champion files carry `weight` (plus optional
  `turn_time`, `accel_time`, `stop_time`, `stride` and `bounce`). The player
  turns at a finite rate, accelerates and coasts to a stop, moves while
  turning, and waits until facing its target before auto-attacking. The walk
  cycle advances from distance covered instead of wall-clock time, so feet no
  longer slide while speeding up or slowing down. This was ported from the
  `MobaScape Movement System Branch` reference checkout.
- Ambient scenery creatures: 44 cache-coloured butterflies, 16 monkey-riding
  pasture cows around the map edge, and 28 shore crabs beside the river. They
  wander on their own schedules, are neutral, cannot be selected or damaged,
  never attack, and never show health bars. Crab NPC 4819 is unreadable in this
  cache revision, so the equivalent beach crab NPC 1553 (same models) is used.
- Cache groups compressed with type 0 (uncompressed) are decoded correctly:
  they carry a two-byte version trailer and no decompressed-length header.
  This unblocked butterfly sequences and other uncompressed assets.
- Tree lines now use only full-canopy models with bark trunks; bare tropical
  stems, stumps and dead-stem models are skipped so they cannot become solid
  green pillars.

- Jungle demo: camp killed, buff Might 58s.
