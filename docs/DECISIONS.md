# Architecture decisions

Short ADRs. Add a new entry when a decision is hard to reverse or affects
multiple crates.

## ADR-001 — Pivot from the 2006Scape Java client/server to Rust

**Status:** accepted.

The 2006Scape Java client/server was built for OSRS-style tick gameplay and a
317 cache. It is too limited for fast 5v5 MOBA pacing, and reskinning it fights
the renderer and interface code. We keep the Java work as reference only
(`docs/reference/`) and build a custom Rust engine.

Consequences: new workspace, new cache pipeline, no Java build in CI.

## ADR-002 — Use a pinned modern OSRS cache (OpenRS2 2392 / build 235)

**Status:** accepted.

Build 235 has a complete XTEA key set on OpenRS2; builds 241+ currently report
zero archived keys, which blocks map-location extraction. Modern content and
models are still available. The cache is gitignored; users fetch it locally.

Consequences: map extraction is reproducible; new content requires a deliberate
version bump and key audit.

## ADR-003 — Custom wgpu stack, not a full engine

**Status:** accepted.

We use `wgpu` + `winit` + `glam` + `hecs` (planned) + `egui` (planned) instead
of Bevy. The OSRS look depends on flat per-face shading, vertex animation, and a
bespoke terrain path; owning the renderer is simpler than fighting a PBR engine.

Consequences: we build UI/assets/audio plumbing ourselves; the renderer stays
small and controllable.

## ADR-004 — NPC/boss models as the first champion roster

**Status:** accepted for v1.

Player-kit compositing (`idk` body parts + equipment models + recolours) is a
large subsystem. Distinct boss models (Jad, Zulrah, Vorkath, etc.) give visual
identity and recognizable silhouettes immediately. Player avatars/gear become a
later cosmetic phase.

## ADR-005 — Left-click tap-to-move + hold-to-steer

**Status:** accepted (user decision).

- Tap LMB on ground: pathfind to the point.
- Hold LMB: steer continuously toward the cursor direction from the champion.
- Click an enemy: attack it.
- QWER: abilities; Shift self-cast; RMB attack-move/force-attack.

Consequences: the input layer needs both path following and direct steering;
the server validates both. Camera edge-pan/minimap remains.

## ADR-006 — Karamja carve + internal map editor

**Status:** accepted (user decision).

Start from a 3×3 Karamja region block and carve a MOBA arena, rather than
hand-authoring terrain from scratch. Build an internal editor with procedural
height/river/scatter and a cached asset library so more maps are cheap.

Consequences: terrain import, height blending, collision rebake, and an editor
UI are M1/M5 work. Map data is split into design JSON + derived bakes.

## ADR-007 — No game assets in the repository

**Status:** accepted.

The repo contains code, design data, and docs only. Cache files, extracted
models/textures/maps/sounds, and screenshots of game assets are gitignored. Users
run the extractor locally. Keep the repo private until distribution policy is
decided.

## ADR-008 — Flat shading first, textures later

**Status:** accepted.

Most OSRS scenery reads through vertex colours and per-face light. v1 renders
flat-shaded models with a green fallback for textured faces; texture sampling,
alpha-tested foliage, and animated textures come after the core match loop.

Consequences: the first visual pass is fast and recognizable; textured content
will look approximate until index 9 is decoded.

## ADR-009 — Server-authoritative 30 Hz with client prediction

**Status:** accepted.

The server owns the sim. The client predicts own movement and reconciles; remote
units are interpolated. Fog is filtered server-side. Direct-connect room codes
precede any matchmaking service.

Consequences: `crates/sim` must be deterministic and shared; netcode tests need
simulated latency/loss from day one.

## ADR-010 — A* on an integer tile grid in `crates/sim`

**Status:** accepted.

Pathfinding lives in the deterministic sim crate, not the renderer. The grid is
integer tiles; movement costs are 10/14; diagonals may not cut blocked corners.
The client uses the same code for click-to-move; the server will own the
authoritative path. Collision is currently conservative (water + object
footprints) and will be replaced by authored map collision.

## ADR-011 — Screen-space HUD via a second pipeline

**Status:** accepted.

The HUD uses the same vertex format as the world but a second pipeline with an
identity camera, alpha blending, and depth testing disabled. This keeps the HUD
cheap and avoids a UI framework dependency. Text is deferred; numeric values are
shown in the window title until a bitmap font atlas is added.

## ADR-012 — One data-driven mid lane first

**Status:** accepted.

The first playable MOBA loop is a single mid lane defined in
`data/maps/karamja_rift/map.json`. Scene building clears a collision corridor,
blocks structure footprints, skips scenery near the lane, and emits a stone-path
ribbon. `AnimatedScene` spawns waves, runs minion/tower AI, and ends the match
when a nexus dies. Top/bottom lanes, inhibitors, jungle camps, and brush are
deferred until the loop is stable.

## ADR-013 — Three lanes + inhibitor-gated nexus

**Status:** accepted.

The map now has top/mid/bottom lanes, two towers and one inhibitor per lane per
team, and one nexus per team. A nexus is damageable only after at least one
enemy inhibitor is destroyed. Minions spawn per lane (two melee, one ranged) and
follow that lane's waypoints. This creates a real push/defend loop without
requiring items or a full champion kit yet.

## ADR-014 — Jungle camps, team buffs, and a geometry minimap

**Status:** accepted.

Four neutral camps sit in the jungle pockets. They respawn and grant the
attacker's team a 60 s buff: Might (+25% damage) or Swift (+20% speed). Camps
are passive in v1 (no retaliation) and use single-model NPCs. The minimap is
drawn with HUD quads from live state (lanes, structures, camps, minions,
player); it is display-only until click-to-pan lands. Fog of war remains a
server-side milestone.

## ADR-015 — QWER champion kit and arrow-key camera

**Status:** accepted.

The player now has a four-ability kit: Q projectile, W blink, E heal/shield,
R meteor. Mana, cooldowns, XP, gold, levels, and shield absorption are tracked.
To free Q/W/E/R, camera panning moved to the arrow keys, rotation to Z/X, and
recenter/follow to Space. R is the ultimate, not camera reset. The HUD draws four
ability boxes with cooldown fills; the title carries the numeric values.

## ADR-016 — Client-side fog/brush prototype

**Status:** accepted for the prototype.

A 192×192 visibility grid is computed from allied units and structures. Enemy
and neutral units outside vision are hidden; brush rectangles hide units unless
the player is within 500 units; an opaque dark mesh covers unseen tiles. This is
client-side only and must move to the authoritative server before multiplayer.
The minimap hides unseen units but does not yet draw fog or support click-to-pan.

## ADR-017 — Data-driven champions, starting with three boss avatars

**Status:** accepted.

Champion data lives in `data/champions/*.json`: model/idle/walk IDs, scale,
health, mana, damage, range, speed, ranged-auto flag, and Q/R/E/W numbers. The
client loads one champion at launch with `--champion monkey|jad|kree`. The first
roster uses OSRS boss/mini-boss models (Monkey, JalRek-Jad, Kree'arra Jr.) and
shares the Q/W/E/R shape; a champion-select UI, passives, and more distinct kits
come later.

## ADR-018 — Six-slot fountain shop with flat-stat items

**Status:** accepted for the prototype.

Items live in `data/items/items.json` with an id, name, HUD colour, gold cost,
and flat damage/health/mana/speed/ability values. The player has six inventory
slots. Buying is allowed only within 900 world units of the spawn fountain, so
the shop is a base interaction rather than a global menu. `B` toggles the panel;
`1`–`6` and row clicks buy; clicking outside or `Esc` closes. Ability power is a
percentage that scales Q/R damage and E heal/shield. The client applies the
transaction locally in this prototype; the server must own gold, range,
inventory, and stats before multiplayer.

## ADR-019 — Built-in 5x7 bitmap font for HUD text

**Status:** accepted for the prototype.

The HUD needs on-screen numbers and shop labels before a cache-native font
atlas exists. `crates/client/src/font.rs` defines uppercase ASCII glyphs as
seven 5-bit row masks. `Hud::text` emits one quad per horizontal pixel run with
a one-pixel drop shadow. This is deliberately small and code-only: no Jagex
font assets, no extra dependencies, and no text layout engine. A cache-native
font atlas can replace it later behind the same `Hud::text` call sites.

## ADR-020 — Fountain economy: passive gold + heal + shop range

**Status:** accepted for the prototype.

The player earns 8 gold per second and, within 900 world units of spawn, heals
6% max HP and MP per second. The shop uses the same 900-unit radius, so the
fountain is a real base interaction instead of a global menu. Two persistent
world rings mark the area. The client owns this in the prototype; the server
must own gold, healing, and shop range before multiplayer.

## ADR-021 — Inhibitor respawn and lane super minions

**Status:** accepted.

Destroyed inhibitors respawn after 120 seconds. While an enemy inhibitor in a
lane is down, that lane's next waves include one super minion (2.5x health, 2x
damage, 1.6x scale, slower, gold minimap marker). Nexus vulnerability is still
derived from live inhibitor health, so a respawn re-shields the nexus. This
makes inhibitor pressure meaningful without adding a full super-minion roster
or inhibitor-specific art.

## ADR-022 — Fogged minimap and click-to-pan

**Status:** accepted for the prototype.

The minimap uses the same 192x192 visibility grid as the world fog. Entirely
unseen 8x8-tile blocks are covered by a dark overlay; the block size keeps the
HUD mesh cheap. `minimap_world_at` is a pure screen-to-world conversion, unit
tested for corners/center/outside clicks. A click inside the minimap pans the
camera and disables follow. Fog remains client-side until the authoritative
server owns vision.

## ADR-023 — 117HD Core lighting pipeline

**Status:** accepted.

The scene now uses `SceneVertex` (position, normal, colour, UV, AO, material)
instead of the old position+colour vertex. Terrain uses smooth heightfield
normals; models and animated units use flat per-face normals to preserve the
OSRS faceted look. `SceneGlobals` carries the camera, sun, hemispheric ambient,
height fog, sky colours, sun screen position, tuning, and a fixed 16-light
array. `scene.wgsl` applies flat-shaded sun/ambient lighting, optional four-band
stepping, dynamic point lights, height fog, and debug views. Colours are
converted sRGB→linear on the CPU because the surface is an sRGB target.

The sky is a full-screen gradient with procedural clouds and a sun disk/glow.
Blob shadows are rendered with a separate alpha-blended pipeline for static
scenery and units. Dynamic lights cover the fountain, Q muzzle/impact, E/R
effects, and projectiles. `--graphics classic` keeps a flat unlit baseline for
A/B comparison; `--debug-view albedo|normal|lighting|fog|lights` exposes the
intermediate buffers.

This milestone deliberately does not include textures, HD water, directional
shadow maps, bloom, MSAA, or draw-distance streaming. Those are M5.1–M5.4.

## ADR-024 — Sun shadow map + contact AO

**Status:** accepted for the prototype.

The 117HD Core scene now renders a 1024x1024 Depth32Float sun shadow map over a
60x60-tile orthographic frustum around the camera. Player, tower, inhibitor,
nexus, and camp models are dynamic casters; static scenery keeps blob shadows
until chunked static caster caching lands. PCF is done with a comparison
sampler. The shadow pass runs every other frame to keep the renderer near 60 fps
on the dev machine. `--shadows off|blob|map` controls the mode.

Terrain and model vertices carry baked contact AO (height-cavity for terrain,
base-to-top gradient for models/units). This is AO v0; full screen-space SSAO
will arrive with the offscreen post-processing pass in M5.4.


## ADR-025 — External OSRS UI atlas, no committed game assets

**Status:** accepted.

The shop/HUD now uses real OSRS item icons and tab/button sprites. Assets live
in the gitignored `assets/osrs-ui` tree and are loaded by
`crates/client/src/ui_assets.rs`, which trims/fits item icons into a 1024×1024
atlas and packs the UI sprites. `ui.wgsl` samples the atlas with nearest
filtering; untextured HUD quads sample a white texel so one textured pipeline
handles all HUD geometry. Missing assets fall back to procedural icons, so the
game still runs from a clean checkout. No Jagex-derived assets are committed.


## ADR-026 — Terrain color smoothing + procedural ground detail

**Status:** accepted for M5.3a.

The terrain color field is box-blurred twice with a separable radius-2 kernel
and assigned to shared grid-corner vertices. Adjacent tiles interpolate the
same corner colors, so flat square color blocks disappear. `scene.wgsl` adds
procedural high/low-frequency detail for terrain (`material == 4`). This is a
visual ground-blending step, not cache-native texture blending; real ground
texture decoding/UVs remain M5.3b.

## ADR-027 — Offscreen post pass with FXAA/grade/cheap bloom

**Status:** accepted for M5.4a.

The scene renders to an offscreen sRGB texture, then `post.wgsl` applies
console-style FXAA, an 8-tap cheap bloom, gamma-space color grading, and
dithering. The HUD renders in a separate pass after post so text and icons stay
crisp. The full-screen UV uses `p * vec2(0.5, -0.5) + 0.5` to match wgpu NDC
orientation. MSAA, full HDR bloom, full SSAO, and draw-distance streaming remain
future work.


## ADR-028 — Full HDR bloom chain and cache-native ground textures

**Status:** accepted for M5.5.

The scene renders into an `Rgba16Float` target, so highlights survive the
frame. A bright pass (`bloom_bright.wgsl`) extracts pixels above 1.0, a
separable 9-tap Gaussian (`bloom_blur.wgsl`) blurs them horizontally and
vertically at half resolution, and `post.wgsl` composites the bloom over the
scene before the ACES-style tonemap, FXAA, vignette and grade. The HUD is
drawn after the post pass so text stays crisp.

Ground detail uses real OSRS 128×128 sprites from `assets/osrs-ground`
(gitignored). They are packed into a mipmapped `texture_2d_array` with one
layer per material (grass, dirt, stone, sand, water, path) and sampled with
world-space UVs. The terrain shader classifies each fragment in display space
and blends the matching layers, which keeps the OSRS palette without visible
tiling.

## ADR-029 — Regional biome pockets, a river and lane tree lines

**Status:** accepted for M5.6.

A single contiguous OSRS region cannot express a Dota style two-sided map, so
the scene builder now loads extra regions on demand: each `biomes` entry
copies a rectangle of a real OSRS region (Wilderness, Morytania, Camdozaal,
Lands End, Varrock, Falador, Tirannwn, the gnome stronghold) into the map,
tints its ground and plants its scenery. The pockets blend into the base
terrain over a seven tile border so the seams disappear.

A `river` polyline cuts a waterway diagonally across the map, separating the
radiant half from the dire half. The bed is carved below the banks, the water
is blocked for movement except at the listed bridge tiles, and the lane
corridor is cleared after the carve so the centre crossing stays walkable.

Every lane is flanked by two rows of OSRS tree models, tinted by the side of
the river they stand on, which turns the lanes into natural barriers.

## ADR-030 — Dota style presentation: shadowed fog of war and readable HUD

**Status:** accepted for M5.6.

The fog of war used to be a blue-grey overlay, which tinted unseen ground.
It now multiplies the scene by `(1 - coverage)`, so unexplored areas read as
shadow while the area around the hero stays at full brightness and no longer
looks overexposed. Lighting flips normals towards the camera, so single sided
OSRS models are lit on the face the player can actually see, and a camera
facing fill light keeps tree fronts readable.

The HUD draws at 1.32x the authored size with the OSRS palette (white body
text, yellow labels, gold currency), a Dota style scoreboard strip with kills,
deaths and a match clock, a unit tooltip that follows the cursor, a minimap
viewport rectangle and a biome label for the pocket the hero is standing in.
The true OSRS font lives in the client jar rather than the cache, so the HUD
keeps the crisp 5x7 pixel font with a one pixel black shadow.


## ADR-031 — Lanes baked into the terrain, watchtowers with lock range

**Status:** accepted for M5.7.

The old lane ribbon was a flat mesh hovering 40 units above the terrain. It
read as a highlighter streak and z-fought with the ground textures. Lanes are
now a mask baked into the terrain: the scene computes the distance to the lane
polyline, wobbles the edge with smooth value noise, levels the road toward the
centre line, tints the corner colour toward the OSRS path colour and stores
the mask in the terrain vertex alpha. `scene.wgsl` blends the OSRS dirt and
path sprites into the ground with that mask. One mesh, no overlay, no
z-fighting.

Watchtowers are a new structure kind. Their `aggro` radius is the short range
at which they first pick up a target; their `range` is the longer lock range
used while they already have a target. This two radius model is deliberate:
once a tower has fired at a hero, the hero has to retreat further than the
distance that pulled the tower in the first place. Bolts travel at 280 u/s and
carry the shooter's unit index so jungle camps only aggro on heroes, not on
tower fire.

The minimap bakes a real terrain image from the scene's terrain vertex
colours (which already contain the biome pockets, the river and the lanes) and
uploads it into the UI atlas. It shows the whole rift plus both teams'
structures and the team's heroes; enemy heroes are never drawn and enemy mobs
only appear while inside team line of sight. Projectiles follow the same rule:
structures are always visible, attacks are not.


## ADR-032 — Deterministic procedural scattering

**Status:** accepted for M5.8.

Hand placing every tree and ruin does not scale to a 220-tile map, but a fixed
grid looks repetitive. The scene now uses a small xorshift RNG seeded with a
constant, so the world is procedurally scattered yet identical every match.
Tree spacing, distance from the lane, scale, rotation, species and clusters
are all drawn from the RNG, with occasional gaps that read as clearings. The
dire side of the river gets dead and dark species, the radiant side keeps the
green canopy. Ruins use the same walk with wider spacing. Pocket scenery is
excluded from the lane corridor so town props never block or clutter a lane.


## ADR-033 — Fog of war as a shader radius, not a mesh

**Status:** accepted for M5.8.

The old fog of war was a terrain-following mesh drawn 60 units above the
ground with multiply blending. It fought with the terrain's z levels on slopes
and its boundary followed the tile visibility grid, which read as a bright
halo around the hero with hard, tile-shaped edges.

The fog is now a fragment shader term. `SceneGlobals.vision` carries the hero
position and a vision radius (1500 world units); the scene shader computes the
horizontal distance from the hero and blends the lit colour between
`lit * 0.42` outside and `lit` inside with a smooth 0.80R..1.30R falloff. This
gives a consistent lateral circle at every z level, cannot z-fight with
anything, and has no geometry cost. The gameplay visibility grid is unchanged:
it still decides which enemy units and projectiles are drawn and what the
minimap reveals.


## ADR-034 — Only rendered props collide; shop purchases are unrestricted

**Status:** accepted for M5.10.

The scene used to add collision for every blocking OSRS object in the loaded
5x5 block, but only the nearest 900 scenery models were rendered. That left
thousands of invisible walls on the hills and between tiles, which is what
made the hero feel stuck. Collision is now added only for objects that are
actually drawn (base scenery, pocket props, lane trees and ruins), so what the
player can see is what blocks. Pathfinding also snaps the start and goal up to
20 tiles, which keeps a hero that was pushed onto a blocked tile moving.

The fountain-only purchase rule was removed. The shop opens with `B` and any
affordable item can be bought anywhere; the fountain still heals and is still
the respawn point.


## ADR-035 — Remote purchases and the Armadylean delivery guard

**Status:** accepted for M5.11.

Buying away from the fountain no longer fails. The shop takes the gold and the
item is stored in a chest next to the team spawn instead of the bag. The chest
is a real OSRS chest model (1249) with a gold ring while anything is waiting.

A HUD button next to the shop button calls the Armadylean guard — an aviansie
using the Kree'arra Jr. model (28019), since the full Armadylean guard is not
present in this cache build. The guard flies from its post beside the chest to
the hero at 1100 u/s, hovers, hands over every stored item (applying the stats
at that moment), plays a delivery ring and flies back to its post. While it is
out the button reads `GUARD IN FLIGHT`, then `GUARD RETURNING`.

The shop shows `IN TRANSIT` on items that are waiting in the chest, and the
inventory limit counts pending items so a player cannot over-order past six
slots. Couriers and the chest are not targetable, cannot be damaged, do not
block clicks and never show health bars.


## ADR-036 — Neutral camps are visible by proximity

**Status:** accepted for M5.13.

Camps are static jungle objectives, not ambushers. Hiding them with brush or
with the tree sight grid made them invisible until the hero was almost
standing on them. Camps are now revealed whenever the hero is within 1500
world units, independent of brush and line of sight; their minimap dots follow
the same rule. Two camps had been configured with model 16238, which is a
table prop rather than a creature. Model 27114 also turned out to be a
creature carrying a cage on its back, and Graardor Jr. reads as a bench due to
its armour plates. Camps now use clean creature models only: Jad (33012),
Kree'arra Jr. (28019), model 24613 and model 18898.


## ADR-037 — Terrain-hugging selection rings and prop clearings

**Status:** accepted for M5.14.

Selection rings, camp rings, structure rings and effect rings were flat
annuli at a fixed height, so they sliced through slopes. `append_ring` now
samples the height grid per vertex (lifted 6 units) and rebuilds the ring as a
terrain-following band.

Scenery props are also kept six tiles clear of every camp and structure, in
both the base region and the biome pockets. This is what glued a pocket chair
to the Graardor camp: the dark-magic pocket (region 48,52) contains chairs,
and one landed on the camp tile.


## ADR-038 — Slope-corrected ground UVs and rock detail on cliffs

**Status:** accepted for M5.16.

Ground sprites were sampled with plain XZ world UVs, so a 45 degree hill
stretched the sprite and cliffs smeared into vertical bands. Dividing the UV by
the fragment normal's Y fixed the density but warped the sprite into ripples
wherever the normal varied, so that approach was replaced:

- Ground UVs are plain, stable `world.xz / 384` again, which keeps every
  texture pattern identifiable.
- The mesh bakes `1 - normal.y` into the terrain vertex `uv.x`. The shader
  uses that smooth per-corner slope to blend a **vertical stone projection**
  (`vec2(x + z, y) / 260`) over cliffs, which is uniform and cannot ripple.
- The lane road mask fades out on steep ground so paths never stretch up a
  cliff face.


## ADR-039 — Scattered critters are weak camps

**Status:** accepted for M5.15.

The eight decorative critters scattered around the jungle were ambient units:
they could be walked through and never reacted. They are now weak neutral
camps (220 HP, 14 damage, 420 aggro, 760 leash, no buff) using model 10413, so
approaching one pulls it exactly like the fixed jungle camps. They also show
up on the minimap through the camp proximity reveal.


## ADR-040 — King Awowogei's Bananarang and Trial by Combat

**Status:** accepted for the Monkey King kit.

King Awowogei's Q and R are now champion-specific, gated by the champion file
stem so Jad and Kree keep the shared Q bolt and R meteor.

- **Bananarang (Q, 8 s / 40 mana):** a fixed rounded-trapezoid path in front
  of the king. It starts at his left hand, leaves 30 degrees left of facing,
  runs across a long rounded top face, then closes down into his right hand.
  A centripetal Catmull-Rom spline keeps the heading changing in gentle arcs
  instead of corner-by-corner turns; the return leg smoothly blends toward the
  king's current right hand if he moves after the throw. It does 60% physical /
  40% pure damage, hits each enemy once, and has a rank-scaling mini-crit that
  briefly stuns. The path is simulated by arc length with a segment-distance
  hit test so a fast banana cannot tunnel through targets at low frame rates.
  Forward depth is 1,300 world units (the furthest top corner is ~1,516 units,
  about 11.8 tiles). The projectile uses the cache-native banana mesh (model
  2387) at 6.25x scale and flies at 30% of the original speed (3.33 s instead
  of 1 s).
- **Trial by Combat (R, 90 s / 100 mana):** a champion-only unit-target ult
  with a 75-tile cast range (five times the original 15 tiles). The barrier
  forms around the two fighters where they stand, clamps both inside, pushes
  other units out, excludes both from tower and watchtower fire, and ends
  after 10 s or when either fighter dies. Eight monkey subjects ring the
  barrier and throw banana peels that stack -2 armor (max -16) for 4 s.
  Killing the target grants +4 permanent base melee damage per win; the
  bonus is applied to actual basic attacks, not just the HUD.
- **Armor:** champions now carry an `armor` stat. Physical damage uses
  `100 / (100 + armor)` for positive armor and an amplified curve below zero;
  pure damage bypasses armor. The HUD team panel shows ARM and DMG.
- **Demos:** `--demo-duel` spawns a rival champion for manual testing;
  `--demo-trial` spawns a wounded rival, casts R then Q, and logs the result.

## ADR-041 — Castle-boxed map with corner bases

**Status:** accepted for the castle pass.

The map is now a square arena boxed in by a 282×282 white castle wall instead
of an open jungle edge. The wall uses procedural unlit white stone geometry
rather than the dark cache wall mesh: a plinth, solid body, walkway cap and
merlons per tile, with square crenellated towers at intervals. Cache models
were tried first but their vertical faces read as black spikes under the
Karamja sun; the procedural wall keeps the reference's bright fortress look.

The two bases moved from the old left/right lane axis to opposite corners:
team 0 in the bottom-left, team 1 in the top-right. Each is a 28-tile courtyard
whose two outer sides are the perimeter wall, with inner walls and gate arches
facing the map centre, corner towers, a blue nexus dais and four white statues.
The three lanes now run diagonally between the corners and cross the diagonal
river at three bridges. Player spawn sits inside the bottom-left courtyard.

`MapFormat` additions: `spawn`, `castle.bounds`, wall/tower/gate/platform/
statue model flags, tower spacing, and per-base corner/centre/size data. The
castle layout feeds both collision and the static mesh builder from one piece
list, so walls and blocking geometry cannot drift apart.

## ADR-042 — Legends of Bielnor clone: Blowpipe Ranger and OSRS fixed-layout HUD

**Status:** accepted for the video-parity pass.

Reference: *LoL but it's Runescape - "Blowpipe Ranger" ADC Gameplay* (Legends
of Bielnor). The clip shows two things worth copying: a ranged marksman hero
built on OSRS models and a blowpipe, and the OSRS fixed-layout interface
rather than a generic action-RPG HUD.

**New champion: Blowpipe Ranger** (`data/champions/ranger.json`)

- Body model 221 (OSRS Ranger) with attachment 176 (the blowpipe), idle 2728 /
  walk 819, 420 HP, 200 prayer, 99 ranged, 16 armor, 640 attack range.
- Q **Toxic Dart** (30 mana, 5 s): a fast dart that deals ranged-scaled damage
  and applies venom.
- W **Dart Volley** (50 mana, 12 s): five darts in a fan, each poisoning.
- E **Ranger's Footwork** (25 mana, 14 s): a backwards hop that grants +40%
  attack speed for 5 s, the way an OSRS ranger kites.
- R **Deadshot** (80 mana, 75 s): a piercing shot that deals +35% against
  targets below 25% health.

**Engine work**

- `ranged` is now a real champion stat; ranged damage scales by
  `1 + ranged / 100`.
- Venom is a new unit state: a timer, a per-tick damage value and a source
  team, ticked once every two seconds.
- Projectiles gained a style (bolt or dart), piercing with a per-projectile
  hit list, a venom payload and an execute multiplier.
- Units gained an attack-speed multiplier with a timer.
- `Model::merge` no longer folds an attachment's vertex groups into the base
  model's skeleton. A held blowpipe used to share group slots with the
  ranger's body, which corrupted the root pivot and tore the body mesh apart;
  attachments now stay outside the animated groups and render in model space.

**HUD work**

- An OSRS stat panel on the left: Attack, Strength, Ranged, Prayer, Hitpoints,
  then Run-Attack, Ranged-Bonus, Melee-Def and Ranged-Def.
- A two-team roster panel on the right with live health bars, matching the
  clip's team list.
- The event log became an OSRS-style chat box with coloured lines, and a
  prayer orb sits beside the status bars.

## ADR-043 — Reference combat feedback: numbers, shouts, feeds and rings

**Status:** accepted for the video-parity pass.

The reference client (Legends of Bielnor) makes every trade legible with a
handful of cheap overlays, and the Rift client now copies them:

- **Floating damage numbers** rise from the unit that was hit. White for
  neutral damage, yellow for the player's own hits, red for damage the player
  took. They are projected from world space into the HUD pass.
- **Kill shout**: a centre-screen yellow banner ("YOU HAVE SLAIN A MINION!").
- **Kill feed**: recent kills listed under the roster panel.
- **Range rings**: a thin red ring at the champion's attack range and a small
  yellow ring on the current target.
- **Minimap ping**: a red marker on the last enemy the player hit.

## ADR-044 — Reference hero select: filters, roster grid and ability tooltips

**Status:** accepted for the video-parity pass.

The reference client picks heroes from a dense grid rather than three large
cards, and every hero carries a written ability list. The Rift picker now
matches that shape:

- Filter tabs across the top (ALL, MELEE, RANGED, MAGIC) that re-filter the
  grid; `F`, `M`, `R` and `C` are the keyboard shortcuts.
- A roster grid of hero cards with baked cache-model portraits, hotkey badges,
  names and class tags.
- A "Your heroes" list on the left and a "Rival heroes" list on the right.
- A detail panel with the hero's portrait, role, class, combat stats and the
  full Q/W/E/R list with names and descriptions.
- A gold LOCK IN button in the panel header.

Champion classes and ability tooltips are data-driven: each champion file
carries a `class` and an `abilities` array of `{key, name, description}`.

## ADR-045 — Reference 5v5 lobby between the pick and the match

**Status:** accepted for the video-parity pass.

The reference client does not drop straight from the hero picker into the
match: it shows a lobby first.

- Two rows of five hero cards, radiant on top and dire underneath, each card
  carrying the baked portrait, the hero name, its combat class and a compact
  stat line (HP / MP / RNG).
- A blue RADIANT tab down the left edge and a red DIRE tab down the right.
- The player's own pick is outlined in gold; the rest of the lineup is filled
  from the roster so the screen always shows ten cards.
- A red "LOADING - PLEASE WAIT..." button. Clicking it or pressing Enter drops
  into the match, and it also auto-starts after eight seconds so the player is
  never stuck on the lobby.

The flow is now: hero select -> LOCK IN -> lobby -> match.

## ADR-046 — Spectators are a read-only role on the one authoritative match

**Status:** proposed.

The MOBA will be watchable live, eventually from an arena inside a persistent
world. A spectator observes the same match instance as the players, through a
`Spectator` role that can never send commands. The protocol carries the role
from its first version.

Consequences: no message format may assume ten players; spectators connect to
a broadcast relay, never to the match server. See [SPECTATING.md](SPECTATING.md).

## ADR-047 — Visibility is a per-recipient filter over full state

**Status:** proposed.

The simulation holds the whole truth and computes each team's vision. Players
receive their team's permitted view; spectators receive everything, behind a
delay set per competition type (none for casual, configured for ranked and
tournaments).

Consequences: fog is never part of match state or the snapshot format; the
delay lives in the relay, not the simulation. This is where ADR-016's
client-side fog moves to.

## ADR-048 — Every match is recorded; determinism is proven by replay

**Status:** proposed.

A recording is the applied commands plus a state hash per tick. Replaying it
must reproduce every hash or report the first tick that differs. Server
authority alone does not guarantee replays.

Consequences: the match must run as a fixed-tick simulation in `crates/sim`
before this can exist; every new simulation system needs a recorded match that
replays, and all simulation state must be in the hash.

## ADR-049 — Move the match into the simulation before adding more to it

**Status:** proposed.

Match logic currently lives in `crates/client/src/animated.rs`, in floating
point, on the render loop. Spectating, replays and multiplayer all need it in
`crates/sim`. Each milestone built in the client adds to what must move.

Consequences: the simulation tick, state hash and recorder come next, and
match systems move across one at a time, each with a replay test. See the
path in SPECTATING.md.

## ADR-050 — Movement is finished first, per champion, before more is built on it

**Status:** proposed.

Champions currently move at a fixed speed and face their direction of travel
instantly. Movement is what a player does every second, so it is built to feel
right before attacks, abilities and items are layered on. The target is weight:
turn time, acceleration, stopping, attack wind-up and distance-driven walk
animation, each a value in the champion's data file.

Consequences: movement becomes the first system in the simulation tick
(ADR-049); a `--lab` mode exists for tuning; the checklist in
[MOVEMENT.md](MOVEMENT.md) must be complete, including a sign-off by play,
before combat is built on the new movement.
