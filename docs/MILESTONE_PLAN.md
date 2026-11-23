# Milestone plan — from vertical slice to 5v5

Derived from the 2026-11-19 audit and revised on 2026-11-22 against the
reference video. Order is dependency-driven: bounded world/HUD art first
because it makes every later playtest useful, then the simulation spine, then
the network, then content and shipping. Each milestone ends with a playable
artifact and a verification note in `docs/STATUS.md`.

The full reference comparison, scorecard and gap matrix are in
[`REFERENCE_AUDIT_2026-11-22.md`](REFERENCE_AUDIT_2026-11-22.md).

## Revised order (2026-11-22)

1. **M6.5 — World Art Foundation (current).** Cache-native ground/grass,
   sculpted terrain, ground cover, slopes and lighting. Keep this slice
   bounded; do not let it become an open-ended art project.
2. **M6.6 — Live interface parity.** Cache ability/spell icons, portraits,
   populated team frames, minimap/ability routing, stone/parchment panels,
   results/settings. Make `--ui-live` the default once parity is reached.
3. **M7 — Simulation authority.** Move the real match into deterministic
   `rift-sim`; client becomes renderer/input/interpolation.
4. **M8 — Networked 5v5.**
5. **M9 — Content and audio.**
6. **M10 — Ship.**

Simulation authority remains the largest long-term gap. After M6.6, freeze
visual scope except for regressions until M7 lands.

## M6.5 — World Art Foundation (current)

Goal: the base, lane, jungle and river read as the same kind of world as the
reference, using only cache-scraped terrain, textures and models.

- **Done:** cache-native ground and grass sprites resolved from cache index 9
  → index 8; no local asset folder required.
- **Done:** sculpted broad/medium/ridge/fine terrain with a castle settlement
  mask, stronger lane grading and deterministic tests.
- **Done:** mip-correct 384-unit grass, per-sample tone, triplanar soil/rock
  at 128 units, slope-driven grass/soil/rock and a steeper cliff override.
- **Done:** deterministic cache-only ground cover (1,839 plants, 244 slope
  boulders) and a less muted grade/sun balance.
- **Done:** per-tile cache overlay textures packed into the terrain vertex
  ID and blended over the classifier.
- **Done:** 64-unit mask-driven river surface with soft shoreline opacity.
- **Done:** three cache-wood bridge decks with low rails, now flush with the
  natural ford.
- **Done:** shallow-water depth bias and a small lift eliminate river/swamp
  flicker; low-ground colours, dark grass, overlays, biome edges and bank
  reeds were lifted for readability.
- **Tried and reverted:** alpha-tested foliage cutouts. This cache's canopy
  alphas are not cutout masks; preserving them punched sky holes through the
  canopies. Keep the opaque average-colour fill.
- **Done (2026-11-23):** noise continuity fix, calm floor, terraced
  plateaus with stone risers, 64-unit display terrain, per-cell overlay and
  biome edges, grass anti-tiling/palette, imp facing.
- **Done (2026-11-23):** baked grass noise + adaptive terrain mesh: scene
  GPU 14.3 → 12.0 ms, 73 fps p50 at 1920×1008.
- **Done (2026-11-23):** dark-slab fix: backdrop below terrain, cache sea
  reclaimed as land, wet-soil shore bands, grass variant normalisation.
- **Done (2026-11-23):** cliff dressing: riser outcrops and plateau tree
  masses; 75 fps at 1920×1008.
- **Next:** decide cliff collision (user call); water props, tree variety,
  LOD/instancing, contact AO; stray black rectangle near 2850,3040.
- **Exit:** base/lane/jungle/river captures read as the reference's world;
  1920×1008 holds 60 fps at p95; no invented visual assets.

## M6.6 — Live interface parity

Goal: the live interface carries the reference's information hierarchy and
interaction, not just its panels.

- Cache QWER/battle-spell icons, item detail icon, target/kill-feed portraits.
- Populate team frames for a real roster; objective/boss timers and ward count.
- Route ability bar, minimap, team frames and target panel clicks.
- Use stone/parchment panels for objectives/target/kill feed.
- Results, rematch, settings, keybinds and chat input.
- Then make `--ui-live` the running default.

Exit: five live screenshots (base, lane, teamfight, shop, death/results) match
the reference hierarchy; the legacy HUD is no longer the default.

## M6 — Reference presentation

Goal: a first-time viewer cannot tell the build apart from the reference at a
glance, minus the networked players.

- **M6.1 Reference frame** *(this pass)* — close camera, scoreboard strip,
  personal K/D/A + CS, enemy team card, objectives + nexus, XP/gold strip,
  paved base courtyards, `--debug-hud` gate for telemetry.
- **M6.2 Unit frames + feedback** — name/level/HP plates for every hero,
  friendly/enemy ring colours, damage numbers with crit scale, cast bars,
  cooldown sweep, hit flash and small camera shake.
- **M6.3 Ability VFX** — one authored effect per ability, ground telegraphs,
  projectile trails, impact decals; no more flat coloured discs.
- **M6.4 HUD skin** — one nine-slice OSRS panel system, consistent type scale,
  tabs for scoreboard/shop/settings, hover and disabled states.

Exit: five screenshots (base, lane, teamfight, shop, death) match the
reference's information hierarchy; 60 fps at 1080p on the 680M.

## M7 — Simulation spine

Goal: the match is a deterministic, headless, testable state machine.

- **Slice 1 done:** fixed 30 Hz tick, integer fixed-point positions, seeded
  RNG, MoveTo/Attack/Hold commands, bot-driven 5v5 with A* pathing, stable FNV
  state hash and a cache-free headless runner.
- **Slice 2 done:** three lanes, 25 s minion waves, towers/inhibitors/nexus,
  nexus shield rule, structure combat, minion/hero structure pressure, hero
  gold/XP/levels/respawns and a decisive nexus win condition. Per-tick hashes.
  `rift-tools match --seed 7` ends at tick 6986 with hash `bc1f32e1b0f15ace`.
- **Slice 3 done:** `rift-tools match --trace-every N` external hash traces.
- **Focus correction:** the client-side sim overlay and standalone 3D arena
  were removed; they pulled attention away from the real match. Spectator
  mode is now a camera swap over the main game's champions (`V`, `Z`/`X`).
- **Core-loop feedback:** last-hit `+NG` gold floats and a proper death /
  respawn overlay (countdown, hidden ability and inventory HUD, `--demo-death`).
- **Lane readability:** enemy tower range rings, aggro rings on minions
  targeting the player, and white last-hit health bars.
- **Ground art:** cache scrape command plus six cache-scraped grass biome
  variants in a 512px mirror-tiled array.
- Remaining M7 work stays headless: keep the sim and CLI as the server spine,
  and defer client migration until the core game loop is satisfying.
- `crates/bots`: scripted dev bots that can finish a match headless.
- `rift-tools match --seed N --ticks 3600` prints the result and hash.
- Client becomes a renderer + input predictor over the sim.

Exit: two runs with the same seed and inputs produce identical hashes; a
headless 5v5 with dev bots completes without a panic.

## M8 — Networked 5v5

Goal: ten humans on one server, server-authoritative.

- `crates/proto`: versioned messages, snapshots, delta encoding, interest.
- `crates/server`: match instances, lobby codes, team/champion select, ready.
- Client prediction for own movement, reconciliation, interpolation for others.
- Fog and vision computed server-side; no hidden-entity leaks.
- Reconnect within 60 s, surrender, pause-on-disconnect policy.

Exit: 2 humans + 8 bots over LAN at <80 ms; then 10 humans on a hosted server;
packet loss/jitter soak passes.

## M9 — Content and audio

Goal: enough content for a real session.

- Eight champions with distinct kits, passives, VFX and voice barks.
- Item tree (components, upgrades, actives), consumables, recall.
- Boss objective (dragon/rift Herald analogue), buffs, soul/objective economy.
- Second map; map editor carve/place/export loop.
- Audio: SFX bus, positional hits/casts, announcer, UI clicks, music hook.

Exit: three complete matches on two maps with eight champions and no
placeholder art or silence.

## M10 — Ship

Goal: a build a stranger can install and play.

- Results screen, rematch, honour; settings (video/audio/keybinds).
- Onboarding/tutorial, tooltips, ability previews, practice mode.
- Packaging for Linux then Windows/macOS; crash reporting; update check.
- CI: fmt, clippy, tests, headless match hash, screenshot diff, release build.
- Performance budget enforced in CI (p95 frame time, memory, load time).

Exit: signed build, fresh install, complete a 5v5 with nine other humans.

## Guardrails

- The sim never depends on wall-clock time, OS RNG or platform floats.
- The client never owns damage, gold, vision or cooldowns.
- No Jagex assets in git; screenshots and caches stay local.
- Every milestone keeps a one-command demo and a written verification note.
