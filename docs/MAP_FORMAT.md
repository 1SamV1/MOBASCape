# Map format (planned)

The map is data, not Rust. A map has a display mesh, a collision/nav bake, a fog
grid, and gameplay markers. The editor writes the same files the runtime reads.

## Directory layout

```
data/maps/karamja_rift/
  map.json          # metadata, markers, lanes, camps, towers, fog zones
  terrain.bin       # baked tile heights/colours/flags (versioned)
  collision.bin     # per-tile walkable/blocked, per plane
  nav.bin           # A* graph / clearance data
  minimap.png       # baked minimap (local only; not committed if Jagex-derived)
  scenery.bin       # optional static placement list, if not in map.json
```

`terrain.bin`, `collision.bin`, `nav.bin`, and `minimap.png` are derived from
the cache and are **not committed**. `map.json` is committed because it is our
design data; it references cache IDs, not assets.

## Current `map.json` (v1.8)

The playable map is `data/maps/karamja_rift/map.json`:

- `spawn`: player spawn tile `[x, y]`; defaults to the first lane's second
  waypoint when omitted.
- `nexus`: one structure per team (model 36053, 1500 HP). The two nexuses sit
  in 28-tile castle courtyards in the bottom-left and top-right corners.
- `lanes`: top, mid, bottom. Each lane has a `waypoints` polyline, four
  `towers` (two per team), two `inhibitors`, and two `watchtowers`. The lanes
  run diagonally between the corner bases.
- `castle`: perimeter and base-castle data:
  - `bounds`: `[x0, z0, x1, z1]` map tiles enclosed by the white perimeter
    wall.
  - `wall_model`, `wall_scale`, `tower_model`, `tower_scale`, `gate_model`,
    `gate_scale`, `platform_model`, `statue_model`: cache IDs/flags for the
    castle pass. The runtime currently draws the walls, towers, gatehouses,
    platform and statues procedurally in white stone; the IDs are kept so the
    layout stays data-driven and cache models can be swapped back in.
  - `tower_spacing`: fraction of each perimeter edge between wall towers.
  - `bases`: `team`, `center`, `size`, and `corner` (`bottom_left` or
    `top_right`). Inner walls and gate gaps are generated from these.
- `wave_interval`: 25 s. Each lane spawns two melee skeletons and one skeleton
  mage per team per wave.
- `camps`: eight neutral jungle camps. Each camp has a tile, model, optional
  idle/walk sequence, health, respawn time, buff (`might` or `swift`), scale,
  and its AI numbers: `damage`, `range`, `interval`, `speed`, `aggro` and
  `leash`. Camps aggro when a hero enters `aggro` or when they take damage,
  chase the hero, leash back to `home` when pulled past `leash`, heal to full
  on arrival and respawn after `respawn` seconds. Camps grant a 60 s team buff
  when killed. `boss: true` marks a region boss, and `name` (optional) is
  the creature's cache NPC name: the objectives panel gives each boss a
  line of its own under that name, and the hover tooltip shows it.
- `biomes`: regional biome pockets. Each pocket transplants a rectangle of a
  real OSRS region into the map:
  - `region`: `[region_x, region_y]` of the source OSRS region.
  - `rect`: `[x, y, width, height]` tile rectangle inside that region.
  - `origin`: map tile where the rectangle is placed.
  - `tint`: ground colour multiplier for the pocket.
  - `grass`: grass layer variant: `0` classic (texture 129), `1` lush
    (203), `2` dry (18), `3` steppe (197), `4` dire (195), `5` straw (205).
    Omit it to use the id-based default.
  - `side`: `radiant` or `dire`, `label`: the name shown in the HUD.
  The dire half holds dark magic (Wilderness dead-tree graveyard), vampires
  (Morytania swamp), ogre ruins (ceremonial pillars) and a goblin camp; the
  radiant half holds a human city (Varrock), a human keep, the elven lands
  (Tirannwn) and the gnome stronghold. Pocket terrain, colours and scenery all
  come from the cache.
- `watchtowers`: per-lane structures with `kind: "watchtower"`, team, tile,
  model, health, `range` (the long lock range), `damage`, `interval`, `scale`
  and `aggro` (the short acquire radius). A watchtower acquires the first
  enemy inside `aggro`, then keeps firing slow bolts at that target until it
  dies or leaves `range`. Its bolts hit heroes, lane minions and jungle camps.
- `river`: the waterway that splits the rift. `points` is a polyline in map
  tiles running diagonally across the map, `width` is the half width in tiles,
  `depth` how far the bed is cut, `bridges` lists the three lane crossings
  that stay walkable, and `levels` sets the water height at the first and last
  point.
- `brush`: rectangles `[x0, y0, x1, y1]` in world tiles. Units inside brush are
  hidden from the enemy unless the player is within 500 units.
- `props` (v1.8, Phase 5 set dressing, `scene/props.rs`): cache models
  only, by **model** id. Three shapes:
  - `place`: `{model, at: [x, y], yaw (degrees), scale, block, tint,
    texture, plaza}`: one model on one tile. The god statues (Saradomin 9146,
    Zamorak 1400) and banners (2065, 8655) at the base gates. `plaza`
    (default 6) is the radius in tiles that lines and scatters leave open
    around it; railings and lamps set it to 0.
  - `lines`: `{models, points | lane_offset, spacing, jitter, gaps, scale:
    [lo, hi], block, tint, texture, lane_clear, seed}`: models strung along
    a polyline, or along both sides of every lane `lane_offset` tiles out.
    `gaps` leaves runs of four slots open as jungle entrances. The grey
    cliff rocks (19997, 818, 819) at 8.5 tiles.
  - `scatter`: `{models, rect: [x0, y0, x1, y1], count, scale, block, tint,
    texture, lane_clear, seed, biome}`: ferns (12932, 1707, 1653, 1655) and
    stumps (1685, 1629, 1695) across the arena. With `biome: "<pocket id>"`
    the scatter dresses that pocket instead: it drops only inside it, and
    `rect` may be left out to use the pocket's bounds. Lane, structure and
    camp clearances still apply.
  - `block` is a square footprint in tiles that blocks movement and sight
    (0 = walk-through). `texture` picks the pool for untextured faces:
    `stone` (default), `wood`, `leaves`, `flowers`, or `plain` to keep the
    cache face colours.
  - Lines and scatter are seeded and skip anything within `lane_clear`
    tiles of a lane, 7 tiles of a structure, 5 of a camp, a `place`
    landmark's `plaza`, inside a biome pocket (unless the scatter names it)
    or base courtyard, or in the river. `place` is not filtered: authored
    positions are trusted.
- `bridge_model`: `{model, yaw, scale, spacing, across}`: a cache bridge
  piece tiled over every river crossing, lined up with the lane, in place of
  the procedural boardwalk. The shipped map uses the plank bridge piece 699
  (object 3248), 4 across at scale 1.3. Cache bridge pieces are authored at
  plane height (699's deck is 961 units up), so the top face is dropped to
  just over the local water. Planks never touch collision.
- `atmosphere`: optional lighting/atmosphere block:
  `sun_dir`, `sun_color`, `ambient_sky`, `ambient_ground`, `fog_color`,
  `fog_density`, `fog_height`, `sky_top`, `sky_horizon`, `ground_fog`.
  Values are sRGB colours and world-space units; the renderer converts them to
  linear light.
- Towers: model 1761, 450 HP, range 640, 18 damage / 1.2 s.
- Inhibitors: radiant model 12953, dire model 9721, 700 HP, no attack. At
  least one enemy inhibitor must fall before the nexus can be damaged.
- Nexus: model 36053, 1500 HP. Destroying it ends the match.
- Castle wall: procedural white stone, with cache IDs retained in `castle`
  for future swaps. Walls block movement and sight; gate pieces do not block.

The scene clears collision in a ~4.5-tile corridor around every lane (after
biome pockets are placed, so a pocket never seals a lane), blocks footprints
under structures, skips scenery within 5 tiles of any lane, and draws a dirt
path ribbon per lane. Positions are absolute world tiles (e.g. 2800, 3040),
not region-local coordinates.

Lane sides are scattered procedurally with OSRS tree models: a deterministic
xorshift RNG (fixed seed `0x5EED9E3779B97F4A`) walks each lane with variable
spacing (170–470 units), random offsets (4.9–9.8 tiles), random scales and
occasional clusters and gaps. Species depend on the side of the river: dead
and dark trees on the dire side, lush green canopy on the radiant side.
Broken OSRS castle walls and ruin clusters (models 16918, 9233 and 9625) are
scattered the same way 10.5–15 tiles out. The seed is fixed, so every match
starts from the identical world. Pocket scenery is never placed within 6.5
tiles of a lane, so a door or a wall can never end up in the middle of the
road.

Lanes are **baked into the terrain**, not drawn as a ribbon. The scene
computes a smooth lane mask with a noise-wobbled edge, stores it in the
terrain vertex alpha and tints the ground toward the OSRS path colour.
`scene.wgsl` blends the OSRS dirt and path sprites into the terrain by that
mask, so the road is seamless with the world and can never z-fight with the
ground.

## Future schema

Top/bottom lanes, inhibitors, jungle camps, brush, fog blockers, and team
spawns will be added as the map editor lands. Positions are absolute world
tiles (e.g. 2800, 3040), not region-local coordinates.

## Terrain bake

`terrain.bin` is a header + per-plane arrays:

```
magic "RIFTTERR" | version u32 | width u32 | height u32 | planes u32
heights: u8[planes][h][w]         # cache height, world = *8
underlay: u16[planes][h][w]
overlay: u16[planes][h][w]
overlay_path: u8[planes][h][w]
overlay_rotation: u8[planes][h][w]
settings: u8[planes][h][w]
```

The display mesh is built from the same data; collision uses `settings` plus
object footprints. Bridge/plane data is kept separate from display meshes.

## Collision and nav

- Collision is per tile, per plane: blocked, bridge, water, cliff, brush.
- Object footprints come from `ObjectDefinition.size_x/size_y`, shape, and
  rotation; walls/doors use `wall_or_door` and `blocking_mask`.
- A* operates on tiles with clearance; lanes are corridors with a preferred
  width. The server owns pathing; the client predicts along the returned path.
- Validate: spawns not blocked, each lane connected, towers reachable, camps
  reachable, no path through cliffs/water, no spawn-to-spawn straight line.

## Fog

- `fog_blockers` are canopy/brush polygons; they block vision but not movement
  (unless collision says otherwise).
- `brush` polygons hide units inside them from the enemy team.
- The server computes per-team visibility at 10 Hz; the client renders fog from
  the same grid plus its local camera.

## Karamja carve plan

1. Import a 3×3 region block around 44,47 (192×192 tiles) and clamp to a
   160×160 playable area.
2. Draw top/mid/bot lane polylines. Flatten a ~10-tile corridor, blend edge
   heights, clear blocking locs, paint lane ground.
3. Carve a river crossing; add bridge objects; keep water collision.
4. Place bases (west/east), fountains, nexus, two towers per lane, inhibitors.
5. Paint canopy/brush fog blockers and jungle camp pockets.
6. Scatter procedural foliage/buildings outside lanes from the cache library;
   curate the base/river banks manually.
7. Bake collision/nav/fog/minimap and validate.

## Editor (planned)

- Same renderer as the client, free camera.
- Modes: import region, sculpt height, paint underlay/overlay, paint collision,
  paint fog, place/rotate/scale objects, lane drawing, marker placement,
  validation overlay.
- Asset library: searchable cached locs/npcs/models with thumbnails.
- Procedural generator: seeded FBM height, river, biome splat, scatter rules.
- Export writes `map.json` + bakes; hot reload lets the client see changes live.


## Fog of war

Fog is not authored in the map. The renderer receives the hero position and a
1500 world unit vision radius every frame and darkens the scene by horizontal
distance in `scene.wgsl`. The visibility grid (used for unit/projectile
culling and the minimap) is still built from team line of sight.


## Delivery

The storage chest and the Armadylean guard are runtime units spawned by the
client next to `player_spawn` (chest at +4/+13 tiles, guard at +2.6 tiles).
They are not authored in `map.json`; only their models are fixed (chest 1249,
guard 28019).
