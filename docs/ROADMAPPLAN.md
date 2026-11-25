# Roadmap plan: front end, shop, minimap and map look

A living plan for bringing MobaScape's screens and arena up to the bar set
by *Legends of Gielinor*, another OSRS-themed 5v5 MOBA, as shown in a
17.5-minute gameplay video (<https://www.youtube.com/watch?v=uPArBJSbNRQ>,
"Ahrim Mid Gameplay").

- **First study, 2026-11-23:** stills every 15 s at 1280x720, crops of the
  minimap and action bar.
- **Second study, 2026-11-24:** the full video again at 1920x1080, contact
  sheets every 10 s, full-size stills of the hero select (0:03), shop
  (11:44), lane fights (2:10, 7:30, 9:20, 11:40) and minimap. The stills
  were measured for hue, saturation and value and compared with MobaScape
  captures at the same size (`--screenshot`, `--shots`).

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
  for plain fills, outlines and gradients. Nothing is drawn by hand,
  downloaded or AI-generated. Nothing from the cache is committed (AGENTS.md).
- Each task lists **acceptance** checks so it can be closed by screenshot
  plus test.

## The OSRS interface kit (cache index 8)

Both interfaces now share one set of cache pieces. Use these before reaching
for anything else.

| Piece | Sprites | Where it is built |
|---|---|---|
| Window frame (bank / Grand Exchange) | tile 297, edges 954-957, riveted corners 958-961 | `stone::OUTER`, theme `panel.frame`, legacy `frame_outer` |
| Recessed inner panel | tile 297 at 62%, edges 962/967, corners 972-975 | `stone::INNER`, theme `panel.inset`, legacy `frame_inner` |
| Notched buttons | grey 293, red 294, thin 295/296 | theme `button.osrs`, `button.osrs.red`; `Hud::osrs_button` |
| Parchment (chatbox scroll) | 1017 | `Hud::parchment` (stretched, not tiled) |
| Item slot | 1077 | `Hud::item_slot`, theme `panel.slot` |
| Skill icons | 197-204 | hero tags and stats |
| Fonts | p11 494, p12 495, b12 496, q8 497 | both atlases |

Text colours: orange `#FF981F` for titles, cream on stone, dark ink on
parchment (`hud/kit.rs`).

## Where we stand against the reference

| Area | Reference | MobaScape now | Remaining gap |
|---|---|---|---|
| Main menu | Stone window over the arena, mode buttons | Stone tablet with the notched buttons over the orbiting arena | Mode menu entries for Practice and Spectate; a confirm dialog on Quit |
| Hero select | Stone title bar with mode and timer; both teams' columns with name, pick, lane and two battle spells; roster with *Picked* overlays; detail with passive, Q/W/E/R per-rank cooldowns and tags; Lock in; Leave | Stone windows throughout: title bar with Leave, both team columns with portraits and staggered bot picks, role tabs and filters, best-fit roster grid with *Picked*, detail with abilities and tags, 3D spotlight, timer and Confirm | **Lane picker**, **battle spells**, passive row, per-rank cooldown numbers, popular build row, music toggle |
| Shop | "Grand Exchange" window: nav, search, tier sections of icons with prices, recipe rows `A + B > C`, detail with Builds into, Buy/Sell, bag with sell-back, Undo, coins | The same: stone frame and insets, tier sections, upgrade ladders as `a > b > c` rows, two-part recipes as `a + b > c`, owned items outlined green, Builds from / into with icons, bag row with sell prices (click to select, then Sell), Undo, coins | Item sets ("Add to set"), slot-based nav (head, body, legs ...), cache arrow sprites in place of the `>` glyph, a hover tooltip with full stats |
| Minimap | Painted map: dark green jungle masses, wide beige lanes, blue river; **soft lit circles where allies see**; team squares for towers, larger for bases; hero portraits in team rings; camera trapezoid | Ground painted from the same cache textures the world uses; live team vision as a dark veil with a soft edge, following wall shadows; jungle darkened by blocked-tile density; wide lanes; team squares with a white core; player portrait in a gold ring; trapezoid | Jungle walls (needs the phase 5 cliffs); portraits for bot heroes; camp icons from the cache `mapfunction` sprites; pings; ward circles |
| World light and colour | Lit grass hue 111-114, saturation ~0.6, value p10-p90 about 0.2-0.7; dark but coloured fog of war | Hue 86-100, saturation 0.55-0.66, value p10-p90 0.19-0.49 (after the 2026-11-24 retune) | Slightly more green on the darkest jungle; brighter highlights on lit lanes (the reference's sunlit stone reaches 0.7) |
| Arena dressing | Grey rock cliffs between lanes and jungle; plank bridges; Saradomin / Zamorak castles; god statues as towers; ice-blue fountain; ground mist in unseen areas | Castle walls, scattered trees, cube-like rocks, no cliffs or bridges | Phase 5 below |
| HUD | Objectives with named camp timers; four ally frames; slim action bar with equipment grid; kill feed of hero kills only | Ally frames, objectives listing every camp, kill feed that includes minion kills, a debug line top left | Phase 6 below |
| Death, end and meta | Greyscale world while dead with a respawn count; Tab scoreboard; victory screen with K/D/A, CS, gold, items and damage; tier list | Respawn timer; win/lose text | Phase 7 below |

## Measured colour targets

From the 2026-11-24 stills (1920x1080 game view, HUD panels cropped out;
measured on the lit centre of the frame). Check a retune with the same
HSV statistics on `--shots tools/shots/terrain.txt` captures.

| | Lit grass hue | Lit saturation | Value p10 / p50 / p90 |
|---|---|---|---|
| Reference 2:10, 9:20, 11:40 | 111-114 (136 at 2:10, little grass lit) | 0.59-0.68 | 0.17-0.26 / 0.31-0.61 / 0.45-0.76 |
| MobaScape before (2026-11-23 tune) | 71-83 | 0.64-0.70 (blue clipped to zero) | 0.22-0.31 / 0.33-0.36 / 0.37-0.39 |
| MobaScape after (2026-11-24 tune) | 86-100 | 0.55-0.66 (jungle 0.45) | 0.19-0.38 / 0.27-0.45 / 0.41-0.49 |

What moved hue most: the key light and haze colours in the map's
`atmosphere` (a yellow sun and olive fog lifted red against green on every
lit surface), not the grass tint. The old post-grade's per-channel contrast
clipped blue to zero in dark linear values, which looked like saturation;
the grade now stretches luma in a square-root space and lifts saturation
explicitly.

## Phases

Ordered by what the player sees most, then by dependency. Each task ends
with a commit and a push to `sam main`.

### Phase 1: Map light and colour

1. ~~Team fog of war in the world shader.~~ Done 2026-11-23.
2. ~~Light retune with `data/lighting.json` and F5 reload.~~ Done
   2026-11-23; second retune 2026-11-24 (`lighting:` commit).
3. **Terrain palette from cache floors.** Pull lane dirt towards the
   reference's grey-brown and the jungle floor towards deep green using the
   cache's own underlay and overlay colours (config archive 1 and 4),
   changing which floor ids the map uses, not inventing colours.
   *Acceptance:* lit grass hue above 100 everywhere and jungle saturation
   above 0.5, with no change to lit lanes.
4. **Highlights.** Let sunlit stone and lane dirt reach value 0.6-0.7 (the
   reference's bright castle stone) without lifting the grass: a highlight
   shoulder in the post grade, or a per-material sun response.
5. **Post-grade uniforms.** Move saturation, contrast and split-tone
   strengths from `post.wgsl` constants into `data/lighting.json` so the F5
   reload covers the whole look.

### Phase 2: Minimap

1. ~~Live ally vision veil~~ (2026-11-23), ~~soft edge and green veil~~
   (2026-11-24).
2. ~~Hero rings~~, ~~player portrait~~, ~~camera trapezoid~~, ~~team
   building squares~~.
3. ~~Cache-texture bake.~~ Done 2026-11-24: each terrain vertex is
   coloured as the world shader paints it (grass layer, lane soil, cache
   overlay, rock on steep faces, water) from the mean colour of the same
   cache textures (grass 129/203/18/197/195/205, soil 32, stone 10, water
   1), with the grass muted to map paint. In-match minimap greens measure
   median (41, 50, 26) against the reference's (39, 48, 30). The rest of
   the difference is the arena itself: the reference jungle is walled by
   cliffs (phase 5.1).
4. **Bot hero portraits.** Bots need a champion id in the match state; then
   bake their portraits as the player's is baked.
5. **Camp icons** from the cache `mapfunction` sprites; **pings**
   (alt-click); **ward circles**.

### Phase 3: Shop

1. ~~Tiers, recipes and ladders, Builds from / into, sell-back, Undo,
   search, Recommended.~~ Done 2026-11-23 to 2026-11-24.
2. ~~Grand Exchange look: stone frame, recipe rows, icon lists, bag row.~~
   Done 2026-11-24.
3. **Arrow sprites.** Replace the `>` and `+` glyphs between recipe icons
   with cache arrow sprites (find them in the sprite dump first).
4. **Slot navigation.** Head, body, legs, hands, boots, necks, rings next to
   the role categories, from `items.json` slots.
5. **Item sets.** "Add to set" plus a saved set per hero in
   `data/` (local only until the server stores them).
6. **Hover tooltip** on grid cells with full stats and the recipe.

### Phase 4: Hero select

1. ~~OSRS stone panels, title bar, Leave~~, ~~team columns and staggered
   bot picks~~, ~~roster with Picked~~, ~~detail with abilities and tags~~.
2. **Lane picker.** Top / Jungle / Mid / Carry / Support, each with a cache
   item icon, in the player's team row. Feeds the bots' lane assignment.
3. **Battle spells.** Two spells per player from OSRS spell sprites
   (Teleport, Heal Other, Vengeance, a Smite-like jungle spell), shown in the
   team rows and cast from the D/F slots in match. Needs gameplay work.
4. **Passive row and per-rank cooldowns** in the detail panel, from the
   champion data.
5. **Popular build row** from the Recommended shop data.
6. Music toggle in the title bar.

### Phase 5: Arena set dressing

All objects from the cache, placed through the map format
(`docs/MAP_FORMAT.md`).

1. **Cliffs.** Ring the lanes and jungle with cache rock and cliff objects
   (grey Trollheim and Wilderness rock walls). Collision follows them.
2. **Bridges.** Wooden plank bridges over the river at the lane crossings.
3. **Bases.** Blue: white Saradomin castle pieces, banners and statue. Red:
   dark Zamorak pieces. Fountain: the cache's glowing portal or pool.
4. **Towers** as obelisks and **god statues**, with their cache spot
   animations for shots.
5. **Jungle flora.** Ferns, flowers, dead trees and stumps from the cache
   across the brush and camp regions.
6. ~~Tree bug.~~ Done 2026-11-24: the dire-side tree line was planted
   with `wood_only`, which put bark on every face of the full-canopy
   models (and over their own cache textures). They keep their leaves now,
   tinted darker.
7. **Ground mist** over unseen areas from cache smoke spot animations.

### Phase 6: HUD polish

1. **Remove the debug line** ("layout 1920 x 1008 scale 1 ...") from normal
   runs; keep it behind a flag.
2. **Kill feed:** hero kills, towers and objectives only; minion kills go to
   the chat at most.
3. **Objectives panel:** named objectives with respawn timers and per-lane
   tower states, not one row per camp.
4. Slimmer action bar with round portrait, skill-point "+" over abilities,
   and potion slots; equipment-grid inventory from the cache slot sprites.
5. Announcements in OSRS yellow for objectives; bounty values in the feed.

### Phase 7: Death, end of match and meta

1. **Death:** greyscale world while dead (a post-grade flag) with the
   respawn count centred.
2. **Scoreboard** on Tab: both teams' K/D/A, CS, gold and items.
3. **Victory and defeat screen:** per-player K/D/A, CS, gold, items, damage
   bars; Play again and Main menu.
4. Tier list and match history need the server (M7/M8); park until then.

## Progress

- 2026-11-23: phase 1.1 (team fog of war in the world shader), 1.2
  (lighting retune) and the grass part of 1.3 are done.
- 2026-11-23: 1.2 finished: `data/lighting.json` with F5 reload. Phase 2.4
  (camera trapezoid) done. Post-grade saturation is still a shader constant.
- 2026-11-23: phase 3.2 layout started: tiered icon grid with prices under
  icons. Phase 2.2: player portrait in the minimap ring.
- 2026-11-24: phase 3 selling (70%) and Undo (100%, this visit) done.
- 2026-11-24: phase 3 upgrade recipes done (`builds_from`, OSRS ladders,
  difference pricing, Undo restores parts).
- 2026-11-24: phase 3 search box and Recommended tab done.
- 2026-11-24: phase 4.2 team columns and staggered bot picks done; PICKED
  roster state; lobby shows the real lineups.
- 2026-11-24 (second study): front end rebuilt from the cache stone kit
  (hero select, main menu); shop restyled as the Grand Exchange with recipe
  rows, icon lists and the sell-back bag; minimap vision veil softened and
  turned green, jungle masses, team squares; lighting retuned against
  measured targets (sun, haze, grade, grass, fog of war).
- 2026-11-24: phase 2.3 done: the minimap bake follows the world shader's
  ground classes with cache texture colours instead of averaging floor
  colours (which showed the brown floors the shader paints over).
- 2026-11-24: phase 5.6 done: the dire tree line's bark cones and bark
  blobs were a `wood_only` flag on full-canopy models.
