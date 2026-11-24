# Roadmap plan: front end, shop, minimap and map look

Written 2026-11-23 after studying a 17.5-minute gameplay video of
*Legends of Gielinor*, another OSRS-themed 5v5 MOBA
(<https://www.youtube.com/watch?v=uPArBJSbNRQ>, "Ahrim Mid Gameplay").
Frames were taken every 15 s (70 stills) plus close crops of the minimap and
action bar, and compared with MobaScape captures at the same 1280x720 size.
The stills are not committed: they are someone else's footage.

## Ground rules

- **The video is a reference, not a template.** It shows what a polished
  OSRS-asset MOBA reads like: interface conventions, information density and
  how far OSRS art can be pushed. MobaScape keeps its own name, layout and
  heroes. We match the *quality bar and genre conventions*, not its screens
  pixel for pixel.
- **Every visual element comes from the OSRS cache.** Sprites, fonts, item
  icons, models, textures, animations, spot animations and sounds come from
  `.cache/osrs-235` through `rift-cache`. Procedural geometry is allowed only
  for plain fills, outlines and gradients (as now). Nothing is drawn by hand,
  downloaded or AI-generated. Nothing from the cache is committed (AGENTS.md).
- Each item below lists **acceptance** checks so a task can be closed by
  screenshot plus test, as with earlier milestones.

## What the reference does better (findings)

| Area | Reference (timestamp) | MobaScape today | Gap |
|---|---|---|---|
| Hero select | 0:00. Beige OSRS stone panels; game mode and timer in a title bar; **lane picker** (Top/Jungle/Mid/Carry/Support with item icons); **both teams' columns** with player name, rank badge, pick-in-progress, lane and **two battle spells** each; roster grid with "Picked" overlays and recommended dots; detail panel with portrait, title, tags (role, archetype, difficulty), **passive plus Q/W/E/R with per-rank cooldowns**, popular build row; battle-spell picker; Lock in plus Elo note; Leave and music buttons | Dark custom panels, type-grouped roster, animated 3D spotlight, stats and abilities, 30 s timer | No lanes, no team columns, no battle spells, no passive or rank numbers, no "picked" state, panel style is not the OSRS stone and parchment |
| Shop | 0:15. "Grand Exchange" window: left nav (Recommended, Item sets, All items, Starters, Weapons, Shields, Head ... Rings, Components, Fighter, Marksman, Assassin ...); search box; centre **tiered sections** (Starting items, Early, Core) as **icon grid with price under each**; **recipes** shown as `A + B > C`; right detail with stats (+1 Power, +4 Health), **Builds into**, Add to set, Buy and Sell; bottom: your items to sell back, **Undo**, coins | Same three-column window, but the centre is a flat text list by weapon category, the detail has only one stat, no recipes, no sell-back, no undo | Icon grid, tiers, recipe tree, build paths, sell and undo, a recommended page per hero |
| Action bar | 0:15, 2:15. Compact bar: round portrait with level, skill-point "+" over abilities, Q/W/E/R plus D/F spell slots, two potion slots, HP and prayer bars with numbers. Inventory is an **OSRS equipment grid** (silhouette slot sprites) with gold and Shop/Teleport buttons | Large panel with square portrait, numbered 1-4 slots, a 6-slot grid with digits | Use the cache equipment-slot sprites; slimmer bar; level-up "+" |
| Minimap | 2:15 crop. Painted map: beige lanes, dark jungle with camp icons, blue river; **team-coloured squares for minions, larger for towers, base buildings**; **hero portraits in team-coloured rings**; your hero with a gold ring; **camera view drawn as a trapezoid**; vision rings around allied wards and towers | Blurry terrain bake, thick blue river bar, lane polylines, 5 px dots, a rectangle for the camera, no hero identity, no vision | Sharper bake, hero icons, trapezoid frustum, **live allied vision (fog) and ally sight lines** |
| Objectives panel | Always on screen: per-lane tower status for both gods; timers for Black dragon, TzKal-Zuk, Giant Mole, River chickens, Shooting star; wards count | Collapsed "Objectives" header only | Fill it with live tower and camp timers |
| Team frames | Top right: four ally rows with name, level, HP bar, ultimate-ready icon | One portrait sliver | Full ally rows |
| Kill feed and announcements | Top right feed with hero icons and bounty "+300"; centre text "First blood!", "A black dragon has appeared!" in OSRS yellow | Feed exists; announcer text sparse | More announcements, bounty numbers |
| Map: art | Cliffs of grey OSRS rock separate lanes and jungle; **wooden plank bridges** over the river; lanes are dirt with worn edges; jungle floor dark green with ferns, flowers and dead trees; bases are **white Saradomin / dark Zamorak castles** with team banners; towers are obelisks and **god statues**; fountain is a glowing ice-blue octagon | Bright flat green, Lumbridge-style castle walls, grey cube rocks, almost no cliffs | Rebuild the arena's set dressing from cache objects (below) |
| Map: light and colour | Dusk-like key light, **dark ambient**; strong contrast; **fog of war dims everything outside team vision to grey**, so the lit areas around allies read instantly; cast shadows from trees and walls; desaturated mid-tones with saturated team colours on top | High ambient (`ambient` 1.64 to 1.74), very light shadows, yellow-green oversaturation; darkening follows only the player's own radius (`scene.vision`) | Lower ambient, warmer key light, higher shadow strength, grade saturation down, and **team-grid fog of war** in the shader |
| End of match | 16:50. Victory banner, rank progress bar (+18), tabs (Overview, ...), per-player K/D/A, CS, gold, items, damage bars; Play again, Ladder, Choose game mode; balloons | Win/lose text only | Post-match screen |
| Meta screens | 17:20. Tier list (S to D) with hero icons and win rates | None | Later, needs server stats |

## Phases

Ordered by what the player sees most, then by dependency. Each phase ends
with a commit, a push to `sam main` and a progress matrix.

### Phase 1: Map light and colour (small, high impact)

1. **Team fog of war in the world shader.** Upload `VisibilityGrid` (one
   byte per tile, already line-of-sight tested in `update_visibility`) as an
   R8 texture every refresh (0.15 s), blurred over one tile. In
   `scene.wgsl`, darken and desaturate outside it (about 45% brightness, 35%
   saturation) in place of the single `scene.vision` circle. Allies,
   minions, towers and wards light their own areas.
   *Acceptance:* with an ally in another lane, its circle is lit on screen
   and on the minimap; walls cast sight shadows.
2. **Light retune.** Ambient from 1.64 to about 1.15; sun from 0.72 to
   about 0.95 and warmer (1.0, 0.92, 0.80); shadow floor from 0.52 to about
   0.30; post-grade desaturation from 0.95 to about 0.82 on mid-tones only,
   keeping team colours. Put the numbers in `data/` (a `lighting.json`) so
   tuning needs no rebuild, with a `--lighting` reload key.
   *Acceptance:* side-by-side capture against reference 2:15 reads within
   reason; no regression in the `--ui-live` screenshot tests.
3. **Terrain palette.** Darken the jungle floor hues and pull lane dirt
   towards the reference's brown-grey. Use the cache's own underlay and
   overlay colours (floor definitions, config archive 1 and 4), changing
   only which floor ids the map uses, not inventing colours.

### Phase 2: Minimap (the user's priority)

1. **Live ally vision on the minimap.** Draw the team `VisibilityGrid` as a
   dark veil over the map picture, so allies' line of sight shows in real
   time (fog lifts where they see, including LOS shadows behind walls).
   *(Started in this task; see below.)*
2. **Hero markers.** Allied and visible enemy heroes get a ring in the team
   colour, the player a gold ring. Then put the cache chathead or the baked
   portrait inside the ring (portraits already exist in the HUD atlas).
3. **Structures and minions.** Towers as larger team squares with a
   cache-sprite icon; minions as small squares; camps with their cache
   monster icons (map function sprites from the cache's `mapfunction`
   sprite group).
4. **Camera trapezoid.** Project the four screen corners onto the ground
   (the renderer already ray-casts for picking) instead of a rectangle.
5. **Sharper bake.** Bake at 512 px from the tile colours and overlay the
   cache *world map* styling: lane colour from the overlay, river from water
   tiles, cliffs darker. Keep the existing click-to-pan and drag.
6. Pings (alt-click) and ward vision rings.

### Phase 3: Shop

1. **Item data.** Extend `data/items/items.json` with `tier`
   (starter/early/core/late), `builds_from`, `builds_into`, and several
   stats (power, health, armour, haste ...). Reuse cache item names and
   icons only.
2. **Layout.** Left nav (Recommended, All, by slot: weapons, shields,
   head, body, legs, hands, boots, capes, necks, rings; components; by
   role). Search box. Centre icon grid in tier sections with price under
   each, recipe rows `A + B > C` drawn with icons and cache arrow sprites.
   Right detail: stats, *Builds from / Builds into* trees, Buy and Sell.
   Bottom: inventory with sell-back, **Undo** (last purchase this shop
   visit), gold.
3. **Recommended page** per hero from `data/champions/*.json`
   (`recommended: [...]`).
4. **Equipment-grid inventory** in the HUD using the cache equipment slot
   sprites (the OSRS worn-equipment silhouettes).
   *Acceptance:* buying a recipe consumes its parts; Undo refunds;
   recommended items are listed first; tests cover recipe cost maths.

### Phase 4: Hero select

Keep the animated spotlight (MobaScape's own touch) and add the
information the reference shows.

1. **OSRS stone panels.** Rebuild the screen from the `panel.stone` kit and
   the cache's parchment and scroll sprites rather than flat dark fills.
2. **Team columns.** Five rows per side: player or bot name, picked hero
   portrait, lane and battle spells; bots "pick" during the countdown with
   a short delay each, and taken heroes show *Picked* on the roster.
3. **Lane picker.** Top / Jungle / Mid / Carry / Support with cache item
   icons. The lane feeds the bots' lane assignment in match.
4. **Battle spells.** Two summoner-style spells chosen from a bar of OSRS
   spell icons (cache magic sprites): e.g. Teleport, Heal Other, Vengeance,
   Smite. Needs gameplay work (D/F slots exist on the bar already).
5. **Detail panel.** Passive, per-rank cooldown numbers, tags (role,
   archetype, difficulty), popular build row (from the shop data).
6. Leave button, music toggle, and the mode plus map name in a title bar.

### Phase 5: Arena set dressing

All objects from the cache, placed through the map format
(`docs/MAP_FORMAT.md`).

1. **Cliffs.** Ring the lanes and jungle with cache rock and cliff objects
   (the grey Trollheim and Wilderness rock walls) so the map has the
   reference's layered, walled look. Collision follows them.
2. **Bridges.** Wooden plank bridge objects over the river at the lane
   crossings.
3. **Bases.** Blue team: white Saradomin stone castle walls, banners and
   statue. Red team: dark Zamorak castle pieces. Fountain: the cache's
   glowing portal or pool effect.
4. **Towers.** Obelisk and god statue models with their cache spot
   animations for shots.
5. **Jungle flora.** Ferns, flowers, dead trees and stumps from the cache
   scattered by the existing brush and camp regions.
6. **Fog-of-war cloud layer** (the reference's grey mist) as a later
   polish item, built from cache smoke spot animations.

### Phase 6: HUD polish

1. Objectives panel: tower states per lane, camp respawn timers, wards.
2. Team frames: four ally rows with level, health and ultimate-ready icon.
3. Slimmer action bar with round portrait, skill-point "+" over abilities,
   and potion slots.
4. Announcements in OSRS yellow for objectives; bounty values in the feed.

### Phase 7: Post-match and meta

1. Victory and defeat screen: per-player K/D/A, CS, gold, items, damage
   bars; Play again and Main menu.
2. Match history and tier list need the server (M7/M8); park until then.

## Already done in this area

- Main menu with Bot, Public and Private (offline) modes, animated title and
  braziers, orbiting arena (ADR-052).
- Hero select countdown, type-grouped roster, 3D spotlight, wielded
  weapons.
- Shop window shell with categories, search line, detail and gold.

## Progress

- 2026-11-23: phase 1.1 (team fog of war in the world shader), 1.2
  (lighting retune) and the grass part of 1.3 are done.
- 2026-11-23: 1.2 finished: `data/lighting.json` with F5 reload. Phase 2.4
  (camera trapezoid) done. Open in phase 1: lane and jungle floor colours
  from cache floor definitions. Post-grade saturation is still a shader
  constant (the post pass has no uniform buffer).
- 2026-11-23: phase 3.2 layout started: tiered icon grid with prices under
  icons. Phase 2.2: player portrait in the minimap ring. Open in phase 3:
  recipes and Builds into data, selling and Undo (refund rate is a design
  call), recommended page, search.
- 2026-11-24: phase 3 selling (70%) and Undo (100%, this visit) done.
  Open in phase 3: recipes / Builds into data, recommended page, search. Bot portraits need bots to carry a
  champion id.

## Started with this plan

Phase 2.1 and 2.2 (first cut): the minimap now draws the team's live
vision as a veil, with line-of-sight shadows, and shows heroes as team rings
(the player in gold). See `docs/STATUS.md`.
