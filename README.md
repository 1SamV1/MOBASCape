# MobaScape

A 5v5 MOBA built on a custom Rust engine, inspired by modern Old School RuneScape
for its visual identity. Server-authoritative, data-driven, and
designed for fast-paced low-team-count matches.

> Status: **M5.6 Biome map + Dota style presentation, plus interface stage 3
> steps A, B and 1 through 15 assembled for review**. The cache reader,
> terrain/location/model/sequence decoders, animated NPCs, a 5x5 Karamja block,
> A* pathfinding with collision, hold-to-steer, a bitmap-font HUD, auto-attacks,
> a Q/W/E/R kit with mana/cooldowns/XP/gold, three data-driven champions, an
> 89-item OSRS shop with cache-baked icons, tooltips and six inventory slots,
> passive gold and
> fountain healing, three lanes with towers/inhibitors/shielded nexus,
> inhibitor respawn and super minions, minion waves, jungle camps with team
> buffs, brush/fog of war, a fogged minimap with click-to-pan, and a win
> condition are working. **117HD Core** adds sRGB-correct sun/ambient
> lighting, per-face model normals, smooth terrain lighting, a sky with
> clouds, height fog, static and unit blob shadows, and dynamic
> ability/fountain point lights. **M5.1** adds a 1024px sun shadow map with
> PCF, dynamic player/structure/camp casters, alternating shadow updates, and
> baked contact AO. The **UI pass** adds an OSRS-style Fountain Shop window
> with categories, item cards, detail/buy/sell panes, real item icons and tab
> sprites, plus HUD objectives, team, resource, event-log, inventory, and
> shop-button panels. **M5.3/M5.4** adds seamless terrain color smoothing,
> procedural ground detail, and an offscreen post pass with FXAA, color
> grading, and dithering. **M5.5** replaces the cheap glow with a full HDR
> bloom chain (Rgba16Float scene target, bright pass, separable Gaussian,
> ACES tonemap) and samples real OSRS ground sprites from a mipmapped texture
> array. **M5.6** rebuilds the map as a two-sided rift: eight regional biome
> pockets inspired from OSRS regions (dark magic, vampires, ogres, goblins on
> the dire side; human cities, elven lands, gnome stronghold on the radiant
> side), a diagonal river with three bridge crossings, a 282x282 white castle
> perimeter wall with corner bases in the bottom-left and top-right, three
> diagonal lanes, tree lines down every lane, a shadow (not tinted) fog of war,
> and a Dota style scoreboard, unit tooltip, minimap viewport and biome label.
> The interface branch adds stage 3 in `crates/ui`: a cache-backed theme,
> scale by fit, the placing/collapse pass, and every component through the
> assembled whole, accepted at PR #1. Stage 4 has started: `--ui-live` reads
> the live score strip, ability bar, minimap, objectives, unit plates, chat,
> kill feed, shop, shop tooltip, announcements, death overlay (with real
> damage breakdown bars), team frames, a click-selected target panel, the
> six-slot inventory, the live gold count from the match, cache-baked OSRS
> item icons and the cache-terrain minimap picture. Every `HudState` slice now
> reads live data. The live shop
> overlay switches categories, scrolls the expanded item list, selects and
> buys items, calls the delivery guard when purchases are stored, and the
> assembled inventory's Shop button opens it. Under
> `--ui-live` the assembled interface owns the screen and the legacy HUD is
> not built. The old HUD remains the running default and its legacy grid still
> shows six entries per
> category while the last interaction gaps and final parity are finished.
> Release on the 680M measures p50 7.7 ms / 130 fps at 1280×720 and p50
> 12.3 ms / 81 fps at 1920×1008 with `--render-scale auto` (0.91×,
> 1746×917), the live interface and the HUD at native resolution. Native 1920×1008 scene
> resolution now holds p50 14.4 ms / 69 fps and p95 15.5 ms with 4× MSAA and
> sun shadows. Timestamp queries put the auto GPU frame at 11.5 ms: scene
> 9.8 ms, bloom 0.8 ms, post 0.8 ms and the shadow map 0.05 ms. See
> [docs/STATUS.md](docs/STATUS.md),
> [docs/UI_ASSETS.md](docs/UI_ASSETS.md) and
> [docs/INTERFACE_STAGE3_REVIEW.md](docs/INTERFACE_STAGE3_REVIEW.md).

## What exists

- `crates/cache` — read-only cache reader:
  - 520-byte sector store, 6-byte index entries, XTEA, gzip/bzip2/LZMA
  - index reference tables, archive splitting with real file IDs
  - terrain (64×64×4), locations, object/NPC/underlay/overlay configs
  - model formats 2 and 3 with flat-shading normals and vertex groups
  - sequences, frames, framemaps, and interpolated vertex animation
- `crates/sim` — deterministic tile grid, collision, and A* pathfinding
- `crates/tools` — `rift-tools` CLI for cache inspection and OBJ export
- `crates/ui` — interface drawing layer and stage 3 interface: sprites,
  nine-slice panels, bitmap fonts, paragraph wrapping, clipping, whole-number
  scaling, `HudState`, scale by fit, the collapse order, the score strip,
  ability bar, minimap, objectives, team frames, target panel, unit plates,
  inventory, chat, kill feed, announcements, tooltip, shop and death overlay
- `crates/client` — native wgpu client: 5x5 Karamja with regional biome
  pockets and a river, animated NPCs, three lanes, towers/inhibitors/nexus,
  minion waves, jungle camps/buffs with aggro and leash, brush/fog of war,
  item shop/inventory, bitmap-font HUD, 117HD Core lighting/atmosphere, HDR
  bloom, cache-native ground textures, fogged minimap, click-to-move +
  hold-to-steer, auto-attacks, Q, and the stage 3 interface dev screen
- `data/maps/karamja_rift/map.json` — lanes, structures, camps, biome pockets,
  river and wave timing
- `docs/INTERFACE_STAGE3_REVIEW.md` — the assembled interface's anchors and
  differences for the reviewer
- `docs/reference` — archived Java-era experiments (not built)

## Requirements

- Rust 1.85+ (developed on 1.97)
- Vulkan / Metal / DX12 capable GPU
- An OSRS cache. Development pins OpenRS2 cache **2392 / build 235** because it
  has a complete XTEA key set. The cache is **not** committed.

## Quick start

```sh
# Download and unpack the pinned cache into .cache/osrs-235/cache
cargo run -p rift-tools -- cache-info

# Inspect Karamja (region 44,47)
cargo run -p rift-tools -- terrain 44 47
cargo run -p rift-tools -- locations 44 47
cargo run -p rift-tools -- find-object tree --limit 5
cargo run -p rift-tools -- find-npc monkey --limit 5
cargo run -p rift-tools -- sequence 222
cargo run -p rift-tools -- animate 7744 222
cargo run -p rift-tools -- model 1715
cargo run -p rift-tools -- sprite 447 /tmp/wood.png
cargo run -p rift-tools -- texture 60 /tmp/leaves.png

# Render the 5x5 block and open the hero-select screen
cargo run -p rift-client

# Skip hero select with an explicit champion
cargo run -p rift-client -- --champion monkey

# Loop King Awowogei through idle, walk, turn, stop and attack animations
cargo run -p rift-client -- --demo-anim

# Tune the Monkey walk baseline live (0 = frozen arms / no torso strafe)
cargo run -p rift-client -- --demo-anim --walk-arm 0.25 --walk-sway 0.1

# Deterministic combat check: target a dummy and fire Q
cargo run -p rift-client -- --demo-combat

# Spawn a rival champion for manual Bananarang / Trial by Combat testing
cargo run -p rift-client -- --champion monkey --demo-duel

# Automated Trial by Combat check: R -> Q -> duel result and permanent bonus
cargo run -p rift-client -- --champion monkey --demo-trial --timescale 4

# Fast-forward the lane for tower/minion/nexus verification
cargo run -p rift-client -- --timescale 20

# Teleport to a jungle camp, kill it, and verify the team buff
cargo run -p rift-client -- --demo-camp --timescale 4

# Cast W/E/R and print mana/cooldown/XP state
cargo run -p rift-client -- --demo-kit --timescale 4

# Play a different champion
cargo run -p rift-client -- --champion jad

# Blowpipe Ranger: ranged ADC with venom, kiting and a piercing execute
cargo run -p rift-client -- --champion ranger
cargo run -p rift-client -- --champion kree

# Zoom the camera in on the hero (the default distance is 2750 world units)
cargo run -p rift-client -- --distance 1800

# Spawn at a specific map tile (handy for inspecting a watchtower)
cargo run -p rift-client -- --spawn 2911 3032

# Verify watchtower aggro, lock range and target reset
cargo run -p rift-client -- --demo-watchtower

# 117HD Core lighting (default) or the flat classic baseline
cargo run -p rift-client -- --graphics 117hd
cargo run -p rift-client -- --graphics classic

# Benchmark/quality overrides (auto scene scale targets 1.4M pixels)
cargo run --release -p rift-client -- --present-mode immediate --benchmark 8
cargo run --release -p rift-client -- --window-size 1920x1080 --present-mode immediate --benchmark 8
cargo run --release -p rift-client -- --benchmark 8 --gpu-timing   # + per-pass GPU ms
cargo run --release -p rift-client -- --present-mode immediate --msaa 1
cargo run --release -p rift-client -- --window-size 1920x1080 --render-scale 1.0 --present-mode immediate --benchmark 8

# Static models/water distance-cull at 16k by default; tune it
cargo run --release -p rift-client -- --draw-distance 12000 --benchmark 8

# Inspect lighting, normals, albedo, fog, shadows, or AO
cargo run -p rift-client -- --graphics 117hd --debug-view lighting
cargo run -p rift-client -- --graphics 117hd --debug-view normal
cargo run -p rift-client -- --graphics 117hd --debug-view shadow
cargo run -p rift-client -- --graphics 117hd --debug-view ao
cargo run -p rift-client -- --graphics 117hd --shadows off|blob|map
cargo run -p rift-client -- --graphics 117hd --pitch 0.22

# Optional: populate the gitignored UI asset tree (see docs/UI_ASSETS.md)
# assets/osrs-ui/items/*.png + assets/osrs-ui/ui/tab.png

# Grant gold, open the shop, and buy six items
cargo run -p rift-client -- --demo-shop --timescale 4

# Interface stage 3: the theme sheet, then the assembled interface
cargo run -p rift-client -- --ui-theme-sheet
cargo run -p rift-client -- --ui-layout

# Stage 4: the assembled interface with live match data (press B for the
# shop overlay; the mouse wheel scrolls the 89-item list)
cargo run -p rift-client -- --ui-live

# Verify fountain healing and passive gold
cargo run -p rift-client -- --demo-base --timescale 4

# Destroy an inhibitor, verify super minions and respawn
cargo run -p rift-client -- --demo-inhibitor --timescale 20
```

The cache download is not automated yet; see
[docs/ASSET_PIPELINE.md](docs/ASSET_PIPELINE.md) for the exact URLs and layout.
Put `keys.json` next to `main_file_cache.dat2`, or pass `--keys`.

## Controls (viewer)

- **Hero select**: click a card or press **1-3**; **Enter**/click **Play** to lock in
- **LMB ground tap**: pathfind to the point
- **LMB hold**: steer directly toward the cursor with collision sliding
- **LMB on an enemy**: auto-attack it
- **Q**: fire blast projectile (5 s); King Awowogei throws **Bananarang** (8 s)
- **W**: blink/dash toward the cursor (8 s)
- **E**: heal + shield (12 s)
- **R**: meteor AoE at the cursor (45 s); King Awowogei starts **Trial by Combat** on an enemy champion (90 s)
- **B**: toggle the Fountain Shop
- **Click a category**: filter shop items
- **Click an item card**: select it; use **Buy** or **Sell** in the detail pane
- **1–6**: quick-buy the matching shop item while the shop is open
- **Click the minimap**: pan the camera to that point (disables follow)
- **Arrow keys**: pan the camera (disables follow)
- **Z / X**: rotate camera
- **Space**: recenter/follow
- `Ctrl` + wheel: zoom (plain wheel is ignored to avoid accidental zoom)
- `F`: toggle camera follow
- `Esc`: close the shop, or quit

## Design north star

- **OSRS identity, MOBA pacing.** 128-unit tiles, flat per-face shading,
  cache-native models, but 30 Hz authoritative combat and 5v5 match flow.
- **Total conversion.** The client boots into the MOBA; no normal OSRS play.
- **Humans only in production.** Bots fill dev slots and run headless soak tests.
- **Data-driven content.** Champions, abilities, items, maps, waves, and UI live
  in `data/` and hot-reload.
- **Server authority.** The client predicts movement and plays cosmetics; the
  server owns damage, vision, cooldowns, and match state.

## Documentation for agents

Start with [AGENTS.md](AGENTS.md). It defines the rules, repo map, build/test
commands, and how to contribute without breaking determinism or leaking assets.

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [docs/ASSET_PIPELINE.md](docs/ASSET_PIPELINE.md)
- [docs/ROADMAP.md](docs/ROADMAP.md)
- [docs/STATUS.md](docs/STATUS.md)
- [docs/ENGINE_ROADMAP.md](docs/ENGINE_ROADMAP.md)
- [docs/MAP_FORMAT.md](docs/MAP_FORMAT.md)
- [docs/NETCODE.md](docs/NETCODE.md)
- [docs/SPECTATING.md](docs/SPECTATING.md)
- [docs/MOVEMENT.md](docs/MOVEMENT.md)
- [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md)
- [docs/UI_ASSETS.md](docs/UI_ASSETS.md)
- [docs/DECISIONS.md](docs/DECISIONS.md)

## License

MIT. OSRS cache data and Jagex assets are not part of this repository and are
not covered by the MIT license. Do not redistribute extracted game assets.
