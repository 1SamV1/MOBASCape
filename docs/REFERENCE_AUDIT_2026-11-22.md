# Reference audit and milestone order — 2026-11-22

Reference: **“I made a RuneScape MOBA…”** by 10xVisa, 11:11, 1080p
(`https://www.youtube.com/watch?v=iWeGj6ErPa0`). The build is titled
*Legends of Gielinor*.

Method: downloaded the video locally, sampled a frame every 10 seconds, and
reviewed full-resolution stops for champion select, the 5v5 loading lobby,
lane/jungle play, teamfights, the base and the shop. Reference frames live in
`/tmp/opencode/osrs_frames/` for this session only; they are Jagex-derived
game imagery and are **not** committed. The source audit also re-read the
workspace, ran the release client, and used the existing GPU timestamp
harness.

## 1. What the reference actually is

### Match flow

1. Title screen (`Legends of Gielinor`, **PLAY NOW**).
2. Champion select with two five-player rosters, role assignment (Top,
   Jungle, Mid, Carry, Support), Elo, pick state, popular builds, passives and
   ability details, battle spells and a locked-in timer.
3. A loading lobby that shows both lineups as five hero cards, a VS split,
   `Loading - please wait`, and a red progress bar.
4. In-match HUD and play.
5. The reference is a real 5v5 match with ten named players and a scoreboard;
   it is not a single-player demo.

### HUD hierarchy

- **Top strip:** blue kills, match clock, red kills, personal K/D/A, CS.
- **Right edge:** four enemy team frames with portrait, level, HP and mana.
- **Left edge:** objectives/nexus, lane status, boss timers (Blue/Red dragon,
  TzKal-Zuk, Giant Mole, River chickens, Shooting star, Wards), ward count.
- **Bottom centre:** health and prayer/resource orbs, QWER ability slots,
  battle spells, item slots, combo/resource pips.
- **Bottom right:** six-slot inventory with real OSRS item icons, gold,
  `Shop` and `Teleport`, and a square minimap with a camera trapezoid.
- **Bottom left:** OSRS chat box and system messages.
- The style is OSRS brown stone/parchment, cache fonts, cache item icons, and
  strong bevels. No flat black debug panels in the reference.

### World and readability

- Close, slightly lower camera than ours; heroes are large on screen.
- Lush, saturated grass with visible variation, flowers, ferns, reeds, bushes,
  mushrooms and ground clutter.
- Sculpted terrain: rolling ground, raised plateaus, rocky cliff faces,
  natural banks and wide dirt lanes with irregular shoulders.
- Dense tree lines and jungle pockets; ruins, bones, rocks and water props
  break up every field.
- Strong fog of war; bright glows and telegraphs are readable against the
  darker edge.
- Unit rings, wide health bars, cast bars, floating numbers and minimap pings
  make teamfights legible.

### Systems implied by the reference

- Server-authoritative 5v5, ranked roles, reconnect/queue flow.
- Real champion kits, passives, battle spells, item shop with icons.
- Boss/objective timers, wards, jungle economy, lane pressure.
- Audio and announcer feedback (not visible, but audible in the source).

## 2. Current project scorecard

| Area | Score | Evidence |
|---|---:|---|
| Cache decoding / asset pipeline | **9/10** | 25 indices, XTEA, terrain/locations, models 2/3, sequences, sprites, textures, fonts, item definitions; 208-texture model atlas |
| Renderer / performance | **8/10** | HDR scene, MSAA 4, sun shadows, bloom, post; GPU timestamps; auto 1920×1008 p50 ~12.8 ms / 79 fps, native ~14.8 ms / 69 fps |
| World art | **5/10 → 6.5/10** | Cache ground textures, sculpted relief, cache plant scatter and boulders landed this pass; cliffs, foliage cutouts, per-tile ground art and water banks remain |
| UI / HUD | **6/10** | Live interface has every major panel, cache fonts/icons/minimap/portrait; ability icons, team depth, click routing and result/settings screens are missing |
| Gameplay breadth | **5/10** | 3 lanes, waves, towers, inhibitors, camps, 89 items, 4 champions; one player hero, two distinct kits, no passives/ranks/recipes/actives |
| Match structure | **3/10** | Hero select, fake 5v5 lobby and nexus win exist; no real roster/results/rematch/surrender/reconnect |
| Simulation authority | **2/10** | A deterministic headless `rift-sim` exists, but the client owns damage/gold/cooldowns/waves/vision on variable `dt` |
| Networking | **0/10** | No `proto`, `server`, sockets, snapshots, prediction or server fog |
| Audio | **0/10** | No audio dependency, decoder, mixer or events |
| Testing / CI | **4/10** | 269 active tests and clean clippy; no CI, screenshot gate, headless client or perf budget enforcement |

## 3. Gap matrix

| # | Gap | Why it matters | Depends on | Effort |
|---:|---|---|---|---|
| G1 | Client owns the match; `rift-sim` is disconnected | Blocks real 5v5, netcode, replay, anti-cheat and deterministic balance | — | 6–10 wks |
| G2 | No `proto`/`server`/queue/reconnect | No human multiplayer at all | G1 | 8–12 wks |
| G3 | Normal match is one hero | Teamfights, roles and the reference lobby cannot be reproduced | G1, content | 3–5 wks |
| G4 | Combat depth (kits, passives, ranks, CC, item tree/actives) | Skill expression and content ceiling | G1–G3 | 6–8 wks |
| G5 | World density and natural terrain | The largest visual gap against the reference | — | iterative |
| G6 | UI parity (ability icons, team depth, objectives/boss timers, settings/results) | Readability and identity | G3, G5 | 4–6 wks |
| G7 | Map/objective depth (boss, tower rules, second map) | Session variety and strategy | G1, G4 | 4–6 wks |
| G8 | Audio/game feel | Perceived polish | G3, G4, G6 | 2–3 wks |
| G9 | Engine scale (LOD/streaming/instancing, async load, SSAO) | Larger maps and 10-hero fights | G5, G7 | 6–8 wks |
| G10 | Shipping/quality (CI, soak, packaging, onboarding) | Regressions and installability | all | 4–6 wks |

**The single largest long-term gap is still simulation authority.** The
world-art work below is a deliberately bounded user-priority slice, not a
substitute for moving the match into `rift-sim`.

## 4. Revised milestone order

1. **M6.5 — World Art Foundation (current).** Cache-native ground and grass,
   sculpted terrain, ground cover, slope/rock treatment, lighting/grade.
   Exit: base, lane, jungle and river screenshots read as the same world as
   the reference; 60 fps at 1920×1008; no invented visual assets.
2. **M6.6 — Live interface parity.** Cache ability/spell icons, target and
   kill-feed portraits, populated team frames, minimap/ability click routing,
   stone/parchment panels, results/settings. Then make `--ui-live` the
   default and retire or archive the legacy HUD.
3. **M7 — Simulation authority.** Port the real map, units, abilities, items,
   camps, waves and win condition into the deterministic 30 Hz `rift-sim`;
   client becomes renderer + input + interpolation. Fix the incomplete state
   hash. This is the highest-leverage milestone in the audit.
4. **M8 — Networked 5v5.** `proto`, `server`, lobby/queue, snapshots,
   prediction/reconciliation, server-side fog, reconnect.
5. **M9 — Content and audio.** Eight champions, kits/passives, item tree,
   boss objective, second map, audio bus and announcer.
6. **M10 — Ship.** Results/rematch, onboarding, settings, packaging, CI,
   perf/screenshot gates and a fresh-install test.

## 5. This pass: world-art first slice

Implemented:

- **Cache-native ground textures.** `GrassTextures` and `TerrainTextures` now
  resolve their sprites from cache index 9 → index 8; local
  `assets/osrs-ground` PNGs are an override/fallback, not a requirement.
  Verified by moving the local asset folder away and rendering from the cache.
- **Sculpted terrain.** Replaced the ±26/±9 rolling relief with warped broad
  hills, a ridged component, medium and fine octaves, a castle settlement
  mask, 88% lane grading, and deterministic tests. Water and base courtyard
  flattening remain authoritative.
- **Grass/soil detail.** One mip-correct grass sample at 384-unit repeat,
  per-sample luma variation, triplanar-projected dirt and stone at 128-unit
  repeat, slope-driven grass/soil/rock classification and a steeper cliff
  override. The post grade is less desaturated and the sun/ambient balance is
  slightly more directional.
- **Per-tile cache overlay textures.** Terrain vertices pack overlay texture
  ID and weight alongside the grass variant; base tiles and biome pockets
  feed real OSRS path/dirt/ground art into the shader.
- **64-unit river surface.** The water sheet now follows the carved water
  mask at half-tile resolution with soft shoreline opacity, removing the
  128-unit bank staircase.
- **Bridge decks.** The three crossings are covered by cache-textured wooden
  decks with low rails.
- **Shallow water flicker.** The water pipeline is depth-biased and the sheet
  is lifted 0.75 units, so it no longer fights the carved bed as the camera
  moves. Low-ground colours, dark grass, overlays, biome edges and bank reeds
  were lifted for the flat dark-patch problem.
- **Cache-only ground cover.** 1,839 deterministic low-poly flowers, ferns,
  grass and mushrooms, plus 244 slope boulders, all selected from cache object
  definitions. No new art assets; the model mesh rose from 674k to 835k
  triangles.
- **Performance.** Auto 1920×1008: p50 12.98 ms / 77 fps, p95 13.86 ms,
  GPU 12.57 ms (scene 11.31). Native: p50 15.27 ms / 66 fps, p95 15.99 ms,
  GPU 14.75 ms (scene 13.52). GPU timings are the reliable budget under the
  current desktop load.

Known limits and follow-ups:

1. **Terrain banks are still one tile (128 units) per vertex.** The water
   surface is now 64 units, but the land side of the bank still needs a finer
   mesh or a slope-aware skirt.
2. **Bridge decks are a first pass.** They read as wooden platforms; stone
   abutments, richer rails and a slight arch would bring them closer to the
   reference.
3. **Ground cover is still sparse at the reference density.** LOD/instancing
   should precede a large density increase.
4. **Lighting/grade still lacks strong contact/SSAO and received shadows on
   cache scenery.** The castle and trees currently skip the sun shadow lookup.
5. **No water props.** Reeds, lilies, shore rocks and driftwood are missing.
6. **Foliage cutouts do not work with this cache.** Alpha-testing canopy
   sprites punched sky holes; keep the opaque fill and pursue tree model
   variety/geometry instead.
7. **Big dirt/rock hills still read as broad smooth masses.** Add rock
   outcrops, tree variety and finer terrain mesh resolution before more
   amplitude.

## 6. Skills installed for this project

Project-level via `skills.sh` (`npx skills add … --copy`), pinned in
`skills-lock.json`:

- `sunsided/gamedev-skills`: `gamedev-wgpu-rendering`, `gamedev-wgsl-shaders`,
  `gamedev-profiling`, `gamedev-debug-ui`, `gamedev-assets`, `gamedev-ecs`,
  `gamedev-project-setup`, `gamedev-rust-workspace`, `gamedev-winit-loop`.
- `mattpocock/skills`: `codebase-design`, `diagnosing-bugs`, `research`.
- `obra/superpowers`: `writing-plans`, `verification-before-completion`,
  `systematic-debugging`.

## 7. Pass 2 (2026-11-23): terrain rework, re-audit

Reference frames re-sampled (one per 10 s plus full-resolution stops at
1:35, 4:10, 7:10 and 9:05); kept out of the repo.

### What the reference terrain actually does

- The playable floor is **flat and calm**: lanes, river banks and fights
  happen on level ground. Elevation is concentrated in **terraces**: flat
  plateau tops, steep grey rock risers, flat ground at the foot.
- Grass is a deep, saturated green (sampled ≈ rgb 26,58,26 lush,
  70,82,28 open field) with irregular blade/speck variation, not a visible
  tile. Lanes are wide dirt with soft shoulders.
- Large dark tree masses and heavy fog of war carry most of the contrast.

### What ours did wrong (before this pass)

- Noise hills everywhere (a lumpy blanket), creased by a discontinuous
  noise function; rock smeared over smooth hills; lime/olive grass with a
  mirror-tile seam grid; square overlay and biome patches at 128 units.

### Fixed this pass

See `STATUS.md` 2026-11-23: noise fix, calm floor, terraced plateaus with
stone risers, 64-unit display mesh, per-cell overlay/biome edges, grass
anti-tiling and palette, imp facing.

### Updated gaps (world art)

1. ~~Sun shadows~~ **Fixed (misdiagnosed).** The dark patches were the
   distant backdrop plane showing through terrain below y = -24, plus cache
   sea tiles; see STATUS 2026-11-23. Sun shadows themselves are subtle and
   could be strengthened toward the reference later.
2. **Performance.** Scene GPU rose 11.3 → 14.3 ms at 1920×1008 (62 fps).
   Bake the mottle/macro noise into a small tileable texture (one fetch
   instead of three procedural noises) and consider 2× subdivision only
   within plateau/riser chunks.
3. **Cliff collision.** Units walk up risers. Making risers impassable (with
   authored ramps) is a gameplay/map decision for the user.
4. **Cliff dressing.** Cache rock/boulder objects along riser feet and lips,
   tree clusters on plateau tops (the reference's dark canopy masses).
5. **Default HUD.** `--ui-live` (cache-scraped OSRS interface) is still
   opt-in; the legacy debug HUD remains the default.

Milestone order is unchanged (M6.5 world art → M6.6 live interface parity →
M7 simulation authority → M8 networked 5v5 → M9 content/audio → M10 ship).
M6.5 exit now also requires shadows fixed and 60 fps p95 restored.

### Skills added this pass

- `gamedev-skills/awesome-gamedev-agent-skills@shader-programming`
- `apollographql/skills@rust-best-practices`
