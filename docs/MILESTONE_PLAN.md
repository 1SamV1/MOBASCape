# Milestone plan — from vertical slice to 5v5

Derived from the 2026-11-19 audit. Order is dependency-driven: presentation
first because it makes every later playtest useful, then the simulation spine,
then the network, then content and shipping. Each milestone ends with a
playable artifact and a verification note in `docs/STATUS.md`.

## M6 — Reference presentation (current)

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
- **Slice 3 done:** `rift-tools match --trace-every N` external hash traces;
  client `--sim-overlay` runs the real `Match` and draws the three-lane
  spectator board with structures, minions, heroes, nexus HP and the live hash.
- Remaining: map sim entities onto cache-backed 3D actors, replace the
  prototype combat state and keep local movement prediction.
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
