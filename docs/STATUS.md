# Status

Last updated: kit runtime for every hero, levels to 25 and talents,
2026-11-25.

## 2026-11-25 — Vine Swing goes the distance; the Flesh Hook is an anchor; ability proposals

- **Vine Swing was short.** It landed on the cursor (never under 300
  units) and, when that tile was blocked, walked back toward the king
  until it found open ground: a demo cast among jungle trees covered 211
  units. Now it always swings the full rank range toward the cursor
  (1500/1600/1700/1800, up from 900-1125); the cursor only aims.
- **Speed curve.** The swing leaves the run-up at a fifth of its top speed,
  speeds up smoothly for 0.6 s and holds 1700 units/s to the landing
  (`swing_distance`, the integral of the speed curve, walked through an
  arc-length table so the speed is true along the rise and fall). It used
  to ease in and out, slowest at both ends.
- **Forward arc.** The ground track bows a tenth of its length to the right
  (was 22%), so the path reads as a curve but carries him forward.
- **Reliable landing.** A blocked landing tile tries 64/128/192 units to
  either side before falling short, never below 45% of the range, and only
  lands where the king could walk on from: a flood fill from his tile
  (`Grid::reachable_from`, `WorldCollision::walk_region`) keeps him off
  the far side of the map wall and out of sealed pockets.
- **Flesh Hook.** Pestilent Bloat's hook is now the Barrelchest anchor
  (item 10887, ground model 23133) flung flukes-first on a braided grapple
  rope tied to the Bloat. On a hit it stays in its catch while the pull
  drags it in, then reels home; a miss reels straight back. Projectile
  looks gained `model` (a plain cache model flown flat, front leading) and
  `rope`; the reel is visual only, the pull is unchanged.
- **`--fixed-dt SECONDS`** makes demo screenshots land on the same moment
  every run (frames used to follow the wall clock).
- Tests: the speed curve (slow start, rising, flat plateau, distance =
  running total), speed along the arc, full-distance and blocked landings,
  the flood fill against walls and sealed corners. Verified with fixed-dt
  captures of `--demo-awowogei` (1500 units in 1.19 s, landing held) and
  the Bloat's `--demo-abilities` (anchor in flight, latched, reeled).
- **Ability rework proposals** in docs/ABILITY_REWORK.md: what Dota 2 does
  with targeting, AoE, levels and innates; an audit of all 18 kits (seven
  near-identical shield Es, three pulls, six leaps, four names used twice,
  one way to scale); nine shared systems (`SYS-1` to `SYS-9`); and every
  hero's passive and Q/W/E/R with an approval ID. The approval log at its
  top records what the review decided.
- **Every kit hero casts its kit (SYS-9).** One `KitRuntime` per kit
  champion, bots included; bots learn ranks, attack through the kit and
  cast by plan (heal when hurt, dash at heroes, damage in reach).
- **Reworked kits for the 16 data-driven heroes** (docs/ABILITY_REWORK.md
  approval log): masteries at max rank, unit/ally targeting, recast,
  toggles, vector and charge casts, and the new statuses. Every ability
  casts in `--demo-abilities`. Every kit gap on the list is closed:
  Web Nest's walking through scenery, the Bloat's reel-in at hook speed and max-rank wall
  grapple (`--demo-abilities --demo-grapple` aims Q at the nearest wall),
  Feather Flurry's 60/70/80% shots locked on the target with a max-rank
  push, Tongue Lash on an ally (zip and shield), Icy Breath's shatter, Hop's
  third-hop big splash (and faster recharge at max rank), Plague's
  three-frog hex, Gale Shot's wall stun (every knockback now stops at
  walls), Pounce's rooted refund, YARRRRRRR!'s kill refund and the Tongue
  Lash zip. The Blowpipe Ranger now runs on the kit too: Toxic build-up
  (venom stacks to three, heal cut at three), piercing dart, seven-dart
  volley, caltrop hop and a three-stack Deadshot execute at max rank. Bots
  switch toggles off when no fight is near. King Awowogei's rework is in
  his hand-written kit: Monkey Madness (four hits on one hero in 6 s charge
  four attacks with bonus damage and a 30% heal), a ricochet on the
  Bananarang's return, two Vine Swing charges, chained peel slips, and
  Greegree slams that mini-stun, a 6/7/8 s extension on a hero kill and
  10 s at max rank. `--demo-abilities --demo-level 25` casts every max-rank
  bonus; all 72 cast.
- **Charge bar and Web Nest.** Held charge abilities (Gale Shot) show a
  gold bar over the slot on both HUDs, white when fully drawn
  (`--hold-indicator 0` holds Q for screenshots). Web Nest gives Venenatis
  +20% speed inside her own web and hides her from enemies outside it,
  for the player's view and bot targeting, while she isn't attacking.
  Inside her own webs she also walks and paths straight through trees,
  rocks and cliffs; structures and castle walls stay shut. A web that runs
  out while she stands in scenery steps her onto the nearest open tile.
  `--demo-abilities --demo-nest` lays E on the nearest scenery and walks
  her across it.
- **Levels to 25 and talents (SYS-3, SYS-7).** Abilities start unlearned;
  level 1 gives the first point. R ranks unlock at 6/11/16. Levels 10, 15,
  20 and 25 give a pick between two talents (`talents` in each champion
  file; HUD buttons above the bar, or Ctrl+1 / Ctrl+2). Points with
  nothing left to rank become stat bumps. Level growth is 32 health,
  16 mana and 1.6 damage, and kit damage grows 2% per level.
- **Small fixes.** The Flesh Hook tooltip names the anchor; SPELLS.md marks
  1574 spare and documents model projectiles; the Swing indicator's doc
  says it is always full length; `rift-tools --help` shows the match
  command's line on `match` again (it had slid onto `spotanim`).

## 2026-11-25 — Ability bar: skill-point "+" and a round portrait (roadmap 6.4, part)

- **"+" buttons.** Every ability that can take a skill point gets a gold
  "+" floating 4 px above its slot (`ability_bar::upgrade_button` /
  `upgrade_at`, drawn outside the bar's clip and pulsing with the slot's
  shimmer). Clicking it ranks the ability up; Ctrl + Q/W/E/R does the same.
- **Click rule changed.** A click on a slot used to rank it up whenever a
  point was free, so a learned ability could not be cast by click while
  points were waiting. Now a click learns only an unlearned (rank 0)
  ability and otherwise casts. The tooltip hint says which: "Click to
  learn (2 left)" or "Click + or Ctrl+W to rank up (2 left)".
- The ability tooltip anchors 22 px higher so it clears the "+" row.
- **Round portrait.** The 76 px portrait is masked to a circle with a gold
  ring (lit top, shadowed bottom), drawn as one span per interface row
  because the draw layer has no circles; the corners become a dark socket.
- Tests: "+" geometry and hit testing, the Ctrl mapping, the hint text.
  Verified with `--demo-ranks 1` screenshots (three "+" over Q, W, E with
  two points left; tooltip clear of them).
- Still open in 6.4: the slimmer bar, potion slots (need consumables) and
  the equipment-grid inventory.

## 2026-11-25 — Objectives panel: named objectives (roadmap 6.3)

- Camps take an optional `name` in map.json, set from the cache NPC that
  shares each camp's model: JalTok-Jad, Crypt spider (boss), Giant bat,
  Kree'arra Jr., Spider and Albino bat (boss). The hover tooltip shows it.
- The panel's lines below the nexuses are now six in all, as the stage 3
  plan counts them: one per region boss under its name with its respawn
  timer, then one "Jungle camps" line per buff for the plain camps
  ("Alive", or "1/3  12s": how many stand and the soonest respawn). A team
  holding a buff shows as that buff's second line ("Blue holds Might 42s").
  Before, the panel had a generic row for each of the eight camps.
- Countdowns ("37s", "1/3  12s") use the title colour; they were drawn in
  the quiet colour because only all-digit states counted as figures.
- `ui_live::camp_lines` is a pure function with a test; `state_colour` has
  one too. The stage 3 sample carries the new lines.
- Verified: `--ui-live` (all alive) and `--ui-layout` (the longest state:
  timers and four second lines).
- `tools/dress_radiant.py` now holds the radiant pocket placement script
  (see DEV_TOOLS.md).

## 2026-11-25 — Radiant pockets dressed

Same method as the dire side: pieces picked from contact sheets as bare
models (no object recolours), every placement checked against the lanes,
river, camps and structures before it is written.

- **Human City** (`radiant_human_city`): the pocket is cut by the top lane
  and the river, so the city is a market street down the west side of the
  road. Seven stalls face the lane (the white-awning base 1421 stacked with
  bakery 1445, silk 1485, gem 1524, veg 4631 and spice 1484 goods, plus the
  blue-striped 24905 and 24903), hanging lamps (1324) between them, barrels,
  stacked barrels and crates (1460, 4615, 7699, 3817) and a picnic bench
  (1453). Between the lane and the river, a well square: the roofed well
  (1410), two lamps, two benches (1496), a noticeboard (2040) and a king's
  statue (1527).
- **Human Keep** (`radiant_human_keep`): a knights' training yard. The knight
  statue (40158) between four blue banners (14664), an archery range of
  four targets (1451, 12896) down the west side, a dummy line (1286, 25177,
  32307) down the east, two weapon racks (5946 with 5944 / 5945), armour
  stands (12561, 47426, 30875), a second statue (40149) and benches.
  Transplanted Falador scenery budget 100 -> 35.
- **Elven Lands** (`radiant_elven_lands`): a crystal grove round the Fragment
  of Seren (38604) in a ring of six elven lamps (37359), with the statues of
  Glarial (38323) and Baxtorian (38322), five crystal trees (37353), arcane
  crystals (30686, 30691, 30693, kept small: at x1.3 they swallowed the
  pocket), an elven crystal chest (37905) and the ornamental fountain
  (13191); scattered crystal clusters, saplings and blue flowers. Jungle
  budget 110 -> 60.
- **Gnome Stronghold** (`radiant_gnome_stronghold`): the spirit tree (8110,
  x2.3, its canopy keeps the cache leaf texture), a glider (1323) between
  landing lights (5647), a gnomeball pitch (goals 1317, flag 10348), gnome
  statues (1302-1304), a cafe of tables, chairs and a cooker (17144, 17136,
  17137), gnome barrels and crates, swings (24742) and an obstacle net
  (1439); toadstools and flowers scattered. Budget 90 -> 50.
- Verified with `--shots` (top and low) over all four. The pocket test now
  checks the radiant set pieces as well.

## 2026-11-25 — Ogre Ruins and Goblin Camp dressed

Same method as the Vampire Swamp: pieces picked from contact sheets,
checked again as bare models (props are placed by model id, so an
object's recolours would be lost; none of these depend on one).

- **Ogre Ruins** (`dire_ogre`): a war camp south of the jungle camp. The
  bone-ring ogre fire pit (6794, x1.8) with the spit-roast (3453), two
  ogre drums (6792), a bench (723), four skull standards (6800), an ogre
  coffin (6786) and chest (3450), a totem pole (1759) and a serpent totem
  (39439), and a row of five spiked skull barricades (6788) facing the
  lane. Scattered boulders (15626, 6880, 685), skulls (1733, 13474, 2171),
  wrecked campfires and a ruined pillar (20055, 871).
- **Goblin Camp** (`dire_goblin`): the Bandos altar (27752, x1.5) in a
  ring of seven shabby and little tents (26482, 7738), four red goblin
  standards (17039), a campfire (20040), a spit roast (4328) and crates
  (26908); scattered skulls, crates, wrecked campfires and rocks.
- Verified with `--shots` (top and low) over both. The pocket test now
  checks all four dire set pieces.
- Not yet: the four radiant pockets (Human City, Human Keep, Elven Lands,
  Gnome Stronghold).

## 2026-11-25 — Ground mist removed; the Vampire Swamp and Dark Magic dressed

- **Mist removed** at the user's request (revert of 48df811). Roadmap 5.7
  is open again; the fog-of-war darkening and height fog are unchanged.
- **Biome dressing.** `props.scatter` takes `biome: "<pocket id>"` to drop
  only inside that pocket (rect optional), and `place` takes `plaza` (the
  clear radius other props keep, default 6) so railings and lamps can sit
  close. MAP_FORMAT.md documents both.
- **Vampire Swamp** (`dire_vampire`, Mort Myre region 54,52), every piece
  a cache object picked from contact sheets:
  - a plaza with the Vampyre statue (object 18045, model 17643, x2.6) ringed
    by ten Darkmeyer spiked railings with red gems (40294 / 40295), open
    to the lane side, and four Swampy lamps (32611);
  - the Pool of blood (35712), two Vampyre tombs (6350, 6357), a Raised
    coffin (39808), the Blood Stinktree (33670), a Darkmeyer spire (17763)
    and two red-glass gothic windows (40438);
  - scattered through the pocket: swamp-bubble pools (2259), Mort Myre
    reeds (3549-3553) and swamp grass (3605-3607), huge mushrooms
    (4092-4094), rotting logs, branches and roots (3595, 3592, 2172),
    rotting trees (3597, 3602), graves (46700, 1363, 36358, 30519) and three
    blood crystals (30685);
  - dry grass layer and a greyer tint on the pocket floor.
- Verified with `--shots` over the plaza (top, low and wide). Tests: the
  new schema fields and the shipped pocket's plaza and dressing; every
  model id decodes from the pinned cache.
- **Dark Magic** (`dire_dark_magic`, Draynor Manor region 48,52): a ritual
  ground with the Arceuus Dark Altar (30837, x0.95) in a ring of eight
  candle stands (22936), two Symbol of Zamorak banners (12580), the chaos
  altar (1216), the spiked Zamorak altar (27778), an Altar of Zamorak
  (35298), the crystal-ball table (1190), the great cauldron (46675) and
  the ceremonial brazier ring (16677); scattered skull piles (2171,
  13474), burning bones and a bone arch (3880, 6899), candles (14204,
  17536), spooky and witch cauldrons (46789, 2241) and bone and ancient
  braziers (16698, 41097). The transplanted Draynor scenery budget went
  from 110 to 55 so its dead-tree canopy frames the ritual instead of
  burying it. At x1.8 the altar dwarfed the pocket.
- Not yet: the other six pockets (Ogre Ruins, Goblin Camp, Human City and
  Keep, Elven Lands, Gnome Stronghold).

## 2026-11-25 — Phase 5: fountains and god-coloured castles

- **Fountain.** The cache "Large fountain" (object 6750, model 13149:
  white stone and blue water, the reference's ice-blue fountain), picked
  from a `model-sheet` of 24 cache fountains. One per team in
  `props.place` at scale 1.7, plain texturing, 2x2 blocking, mirrored
  through the map centre: blue (2713, 3165), red (2977, 2925). Each sits
  inside its courtyard, within the 900-unit heal range, off the spawn
  point and the gate path.
- **God colours.** Castle pieces now carry the team whose courtyard they
  enclose (`CastleSpec::team_at`: the base rectangle plus one tile, so the
  inner walls, gates, corner towers, the perimeter run that closes the
  courtyard, the dais and its statues). `CastlePalette` colours them:
  Saradomin keeps white stone with a blue walkway band and gold merlons;
  Zamorak gets dark stone, a blood-red band and a red dais pool. The open
  perimeter stays white. The castle layout test checks the assignment.
- Verified with `--shots` at both spawns, both courtyards and the north
  perimeter.
- Still open in Phase 5: ground mist (5.7).

## 2026-11-25 — Phase 5: tower shot graphics

- Tower orbs (spot animations 1526 / 1528, the watchtower's 1527: one
  cache model in three recolours) are now checked on a contact sheet and
  listed in `kit::spot::VERIFIED`, with all twelve Surge graphics
  (1455-1466).
- Every tower shot lands in the OSRS Surge splash of its colour: water
  (1460) for blue, fire (1466) for red, earth (1463) for the watchtower.
  A new `ProjectileLook::hit_scale` draws the splash at unit size (1.3)
  while the orb still flies at 3.2-3.6x. A test keeps tower looks on
  verified ids.
- `--demo-tower` moves the hero 4 tiles from the nearest red tower one
  second in. Verified: the red orb flies and the fire splash covers
  Kolodion on the hit; the watchtower demo shows the earth splash, though
  the tower's own model hides part of it from the default camera.
- Still open in Phase 5: god-coloured castle pieces, the fountain and
  ground mist.

## 2026-11-25 — Phase 5: arena set dressing

- New `props` block in map.json (`scene/props.rs`, MAP_FORMAT v1.8):
  single `place`ments, `lines` (a polyline, or both sides of every lane)
  and seeded `scatter` rectangles. All are cache models. They can block
  movement and sight, and keep off lanes, structures, camps, courtyards,
  biome pockets and the river.
- Shipped arena:
  - Saradomin statues and banners at the blue gates; Zamorak statues and
    banners at the red gates.
  - About 410 grey cliff rocks along the lanes, with gaps.
  - About 1,950 ferns and stumps.
  - Cache plank bridges (piece 699) over the three river crossings. They
    replace the procedural boardwalk and sit just above the local water.
- Scene models can ask for `plain` texturing, which keeps the cache face
  colours instead of a stone or leaf texture pool. Statues stay white or
  dark, ferns stay green.
- `rift-tools model-sheet` renders models or objects in a contact sheet
  for picking props.
- Still open in Phase 5: god-coloured castle pieces, the fountain, tower
  shot graphics and ground mist.

## 2026-11-25 — Thrall scrolls

- Three 300-gold consumables in a new SCROLLS shop tab: ghostly (magic,
  npc 10880), skeletal (ranged, 10883) and zombified (melee, 10886)
  thrall scrolls, with cache icons Ectophial, Bones and an ensouled goblin
  head. An item's new `thrall` block (`items::ThrallSpec`) describes the
  creature; any hero can use one.
- Number keys 1-6 read the scroll in that bag slot while the shop is closed
  (they still buy while it is open). The scroll is used up and leaves the
  Undo history. The thrall rises in the Arceuus swirl (1290) and fights for
  40 s; a new one replaces the old (`animated/thrall.rs`).
- Summons now attack their hero's target first, camps included, as OSRS
  thralls do; kit summons gain this too. NPC loading for summons was split
  out of the kit runtime (`kit_runtime::load_npcs`) so thralls preload
  for every hero.
- Shop and bag tooltips say what a scroll does (`ItemConfig::use_lines`).
- Dev flags: `--items a,b` (start with items in the bag) and
  `--demo-thrall`. Verified: all three rise beside Kolodion, and the ghost
  hits the camp he attacks; the SCROLLS tab fits in both HUDs.

## 2026-11-25 — Pestilent Bloat; NPC opcode 99 fixed

- **NPC decoder:** opcode 99 (render priority) is a flag with no payload,
  but the decoder read a byte for it and stopped on a bogus opcode (the
  "opcode 6 / 44 / 65 / 84" failures). Pestilent Bloat, Ahrim, the
  Alchemical Hydra, the Nightmare, Skotizo and the Barrows brothers now
  decode. Regression test
  from the Bloat's opening bytes.
- **Pestilent Bloat** (npc 8359, `data/champions/pestilent_bloat.json`), a
  MELEE "flesh hooker" with a kit after Dota 2's Pudge: models 35404 +
  50070, idle 8080, walk 8081, stomp 8085 as its attack.
  - Passive **Flesh heap**: every enemy dying within 450 adds 2 maximum
    health for good, 25 for a hero (new `Passive::FleshHeap`).
  - Q **Flesh Hook**: a thrown limb (1574, blood splat 1576) that pulls the
    first enemy hit back to the Bloat (`pull` already stops 110 short).
  - W **Swarm**: its flies (1568/1569) ride on it for 5 s, biting and
    slowing everything within 260 and costing it 4 health a tick. New zone
    fields `follow` and `self_damage`.
  - E **Bloated Hide**: 140 shield and 20% damage reduction for 4 s (1579).
  - R **Devour**: a 2.4 s channel that stuns and chews what is in front
    of it every 0.4 s, healing 15 per bite.
- Icons 228 / 298 / 141 / 248. Balance report regenerated; after a trim
  (Q 80, swarm 9 a tick, Devour 25 a bite) nothing is flagged.
- Verified with `--demo-abilities --champion pestilent_bloat` (hook drags
  the camp monster in, swarm ticks, shield 136 after damage) and hero select
  at 1280x720. Bots cast it from the rank tables (ADR-055), so a bot Bloat's
  hook hits but does not pull.

## 2026-11-25 — Magic: spell graphics, teleport scrolls, battle spells

Eight commits that follow the plan in docs/SPELLS.md:

- **Spell graphics.** 46 cache spot animations checked by eye and listed in
  `kit::spot::VERIFIED` (a test keeps unlisted ids out). `--demo-gfx`
  plays any set in a labelled grid. Every kit action can name `cast`,
  `land`, `hit` and `hold` graphics. Kolodion's claws and pillar of fire,
  the Wyvern's ice and the Wise Old Man's teleport now use the OSRS ones.
  Stun holds the stars, a freeze the ice block, and a shielded hit splashes
  (85). OSRS overhead chat (yellow text) is in both HUDs.
- **Teleport scroll** for every hero on T: a 3 s channel to an allied
  structure picked in the world or on either minimap, broken by hard crowd
  control. The arrival is visible to both teams with a ping. Bots read
  scrolls back to lane.
- **Battle spells** for MAGIC heroes on D and F: Ancient Teleport, Heal
  Group, Vengeance ("Taste vengeance!"), Ice Barrage, Entangle and Tele
  Block (`data/spells/battle_spells.json`, new `NoMobility` status). They
  run for any caster in `animated/battle.rs`. Camera follow moved to Y.
- **Hero select**: two spell slots under the spotlight open a spellbook
  picker; the lane fills a default pair; the team rows show spell icons or
  the scroll. `--spells a,b` and `--spell-picker` are the dev flags.
- **Bots** pick spells by lane and cast them by rule (`spells::bot_aim`).
- **Chaos Elemental** (entry below).
- ADR-056 records the magic-only rule and the scroll for everyone.

## 2026-11-25 — Chaos Elemental joins the roster

- New MAGIC hero from npc 2054 (`data/champions/chaos_elemental.json`):
  models 11216 + 57329, idle 3144, walk 3145, attack 3146, 520 health,
  600 range. Its basic attack is the new `chaos` look (Discord swirl 557,
  ring 558); Confusion and Madness have their own looks (551/552, 554/555).
- Kit: Pure chaos (every third ability hit lands a random slow, venom or
  mana drain), Q Discord (skillshot + mana drain), W Confusion (burst that
  throws each enemy 350 in a random direction), E Madness (skillshot that
  disarms for 2 s), R Chaos Storm (1.2 s channel of three-bolt fans, each
  bolt a random drain, throw or disarm).
- New kit schema: statuses `disarm`, `scramble` and `one_of`, and
  `Passive::Chaos`. A disarmed unit starts no basic attacks and loses the
  one it was winding up; it holds the red spike crown (559). A throw plays
  the green Confusion splash (552) at both ends.
- Icons 586 / 16 / 120 / 578. Magic, so it picks battle spells on hero
  select. Balance report regenerated; nothing flagged after Q and E went up
  by 10 and 5.
- Verified with `--demo-abilities --champion chaos_elemental` screenshots
  and hero select at 1280x720. 462 tests pass.

## 2026-11-24 — Hero select: passive row, popular build, short windows

- The detail parchment now reads like the reference: "Popular build" with
  the hero's Recommended list key and five cache item icons (best weapon,
  armour, boots and jewellery, then the next dearest), the tagline, then a
  "Passive" row over Q/W/E/R. The twelve kit passives are named in their
  champion files (`passive_name`); the sentence comes from the numbers
  (`Passive::describe`, `Status::describe`).
- Short windows: rows give way in order (tagline, weapon, descriptions,
  build) instead of overflowing; icons move beside the key chip when a row
  is too short to stack them; cut text ends in "..." and the full text is
  in an OSRS hover box (pale yellow, black edge), drawn last so nothing
  covers it. At 1024x600 the roster drops the name line for two rows of
  bigger portraits (the cursor names them), the left column widens to fit
  the tags, and the stat cells show icon and number with the label on
  hover. Before this, 1024x600 ran the roster past its panel, the tags off
  the parchment and the R row off the bottom.
- Dev: `--cursor X Y` starts with the cursor at a window pixel, for
  screenshots of hover states (a real mouse over the window overrides it).

## 2026-11-24 — Hero select: lane calls, locked-in countdown

- "Your lane:" tabs (Top / Jungle / Mid / Carry / Support, cache skill
  icons 199 / 220 / 215 / 208 / 205) on the bar over the roster; the class
  filters became compact icon tabs beside them. Every team row shows its
  call. A call is a team indicator, not a rule: optional, withdrawn by
  clicking it again, kept for the next draft, never checked in the match.
  Blue's bots call the lanes the player leaves open and spawn in them.
- After Lock in (or at zero) the screen reads "All heroes locked in /
  Game starting in 5" like the reference: roster dimmed around your pick,
  a grey inert *Locked in*, final picks in every row; lane calls can still
  change. Enter skips the count. The separate 5v5 card lobby is removed.
- Dev: `--lane <lane>` and `--lock-in` (with `--start-select`).

## 2026-11-24 — Bots dash and leap

- Two more bot spell shapes (ADR-055): Dash (damaging charges and leaps:
  Callisto's Charge, Graardor's Bandos' Wrath, K'ril's Demonic Charge,
  Venenatis's Pounce, Zilyana's Blessed Dash) hurls the bot at its target
  and hits everything where it lands; Leap (Jad's Teleport, Kree's Leap,
  Vine Swing, Ranger's Footwork) chases a hero out of reach or carries a
  retreating bot home. Both stop at the first wall on the way.
- These used to fire as ranged bolts (or, for leaps, as heals).
- 6-minute sim: 47 bolts, 43 strikes, 1 nova, 1 dash (Callisto's charge,
  two kills), 19 escapes (Jad 10, Kree 9).


## 2026-11-24 — Bots cast abilities

- Bots now use their Q/W/E/R (`animated::bot_cast`, ADR-055), read from
  each champion's rank tables: heals and shields below 60% health,
  novas, short strikes with splash, and homing bolts, plus stuns, roots
  and slows taken from the descriptions. Ranks unlock with level, damage
  scales with ability power, and mana is spent and regenerated.
- Bots were spawned with no mana pool; they now get the champion's mana
  and +20 per level, like the player.
- In a 6-minute sim, 110 casts across all nine bots; towers wore down on
  both sides, inhibitors and nexuses held.
- Approximation: summons and zones read as bolts or strikes (dashes: see above).


## 2026-11-24 — Bot heroes play the draft; the nexus is open

- The other nine draft picks now play the match (`animated::bots`, ADR-054).
  Blue bots: top, bottom, bottom, jungle (mid is yours); red: top, mid,
  bottom, bottom, jungle. Each bot lanes, jungles its own side, takes its
  side's region boss with a teammate from level 5, ganks in pairs (never
  into a base), and walks home under 30% health or when it can buy its
  next recommended item. Bots level and earn gold like the player; basic
  attacks only at first (abilities: see above). `--bots` fills both teams at boot for testing.
- Region bosses: `inner_bot_west` (blue side) and `top_east` (red side) (`"boss": true` in the
  map). Camps now aggro on, and turn on, any hero, not only the player.
- The nexus is always attackable (it was shielded until an inhibitor fell).
- Two lane bugs found on the way, both older than the bots:
  - The wave cap (36 per team) counted dead minions, and units are never
    removed, so every lane stopped spawning for good about two minutes
    in. It counts the living now, and new minions reuse long-dead slots.
  - The scene keeps towers 5 tiles off the lane centre, and minions
    measured aggro to a tower's centre, so waves walked past live towers
    into the inhibitors. Minions now see structures from their edge plus
    400. In a 7.5-minute sim, towers wear down and fall in lane order;
    inhibitors and both nexuses stayed full.
- Shared experience: heroes within 1400 of an enemy kill they did not
  land get 60% of its experience (no gold), player and bots alike.
- Hitsplats are half size: the 24x23 cache splat at 1x with the bold
  cache font at 1x (was 2x splat, 3x font).
- Bananarang chips enemy towers, inhibitors and the nexus with its pure
  share (40%) of each hit, without the stun.
- Minimap and enemy roster show the bots by name; allies blue, seen
  enemies red.


## 2026-11-24 — Skeleton binding: no more floating limbs

- Multi-part heroes no longer come apart while they animate. Before this,
  every attachment was frozen in the rest pose (`Model::merge` dropped its
  labels): Graardor's arms, kit heroes' hands and heads, Kree'arra's bird,
  Zilyana's sword, the Ranger's legs.
- `rift_cache::skeleton::assemble` binds each part native / follow / rigid
  by measuring drift and stretch over idle, walk and attack (ADR-053).
  Followers never move a pivot (`Model::follow_groups`), so the old
  blowpipe pivot bug stays fixed.
- Limb identification (`identify_limbs`) names each label head / torso /
  arm / leg / tail and limits a following vertex to its own limb region.
- In the pinned cache, frozen parts drifted 0.02–0.76 of the hero's height;
  bound, every part stays under 0.03, and that limit is a test.
- `rift-tools skeleton <hero|all> [--sheet out.png]` shows bindings, the
  limb map and old-vs-new poses. `--demo-anim` now honours `--champion`.
- Footsteps (sounds on a unit's walk loop, e.g. Graardor's stomp 3834
  twice a stride) play at their own `footstep` volume (0.2, was the 0.6
  frame-sound volume) with per-step pitch ±8% and volume 75–100%.
- Evergreens lost the dark band under their top tier: the bark override
  for broadleaf trunks no longer repaints faces the cache textures itself.


## 2026-11-24 — Hero select: team columns and bot picks

- Five rows per team down the edges of the spotlight stage (your team
  left, enemy right): portrait once locked, a pulsing "..." while
  choosing; your row shows your highlighted hero, pulsing until you lock.
- Bots lock in on `hero_select::PICK_SCHEDULE` (red 2.5 s, blue 4.5 s,
  alternating, done by 22 s). A bot never takes your highlighted hero,
  prefers a type its team lacks, and all ten heroes are different. Locking
  in early fills the remaining picks at once.
- Taken heroes are dimmed and stamped PICKED on the roster; clicks ignore
  them and arrow keys step over them.
- The 5v5 lobby shows these lineups (it used a fixed rotation before).
- Still cosmetic in match: bots do not yet play the hero they picked.

## 2026-11-24 — Recommended tab

- `data/items/recommended.json`: lists keyed by champion id, a role word
  (`TANK`) or class (`MELEE`, `RANGED`, `MAGIC`), tried in that order
  (`ItemShop::recommend_for`). Unknown item ids fail at load; `_` keys are
  comments. Shipped: one list per class plus TANK, 12–13 items each across
  all four tiers.
- New first category, RECOMMENDED; the shop opens on it when the hero has
  a list (else ALL). Categories now filter by label, not index.
- Fixed: the detail pane, tooltip and Buy/Sell could act on a selection
  the current tab or search hid (the demo showed Staff of air while Buy
  pointed at the bronze sword). `ItemShop::shown()` is now the one item
  all of them use.
- Verified: Kolodion (BATTLE MAGE, MAGIC) gets the 13-item MAGIC list.

## 2026-11-24 — Shop search

- Click the search box to type; every word must appear in the item's
  name, id or category, or name a stat it gives (`damage`, `health`,
  `mana`, `speed`, `ability`): "staff fire", "mana", "boots".
  `items::matches_query`; filters on top of the category rows.
- While typing, keys go to the box only (no casts, no B, no digits);
  Backspace deletes, Enter or any click ends typing, Esc clears it.
  Closing the shop forgets the search. Selection stays when it still
  matches, otherwise moves to the first match; the grid scrolls to the top.
- Gold frame and caret while focused; "No items match the search." when
  empty. Verified with `--demo-shop` ("sword" shows 8 items).
- Also fixed (2026-11-24): the lobby's team cards stayed over the arena in
  `--ui-live` matches (the legacy HUD mesh was never cleared).

## 2026-11-24 — Upgrade recipes

- **Data**: `builds_from` on items in `data/items/items.json`. Shipped
  ladders follow OSRS: bronze → iron → steel → mithril → adamant → rune
  swords and platebodies; shortbow → oak → willow → maple → yew → magic
  shortbow; staff of fire → mystic fire staff; Bandos boots → guardian
  boots; ranger boots → pegasian boots; dragon boots → primordial boots.
  Load fails if a recipe names an unknown or not-cheaper item.
- **Rule**: owning the parts makes an item cost its price less theirs
  (`ItemShop::upgrade`); buying it removes the parts and their stats, so
  the slot is freed (a full bag can still upgrade). Undo refunds what was
  paid and puts the parts back. Selling an upgraded item pays 70% of its
  full cost.
- **Live shop**: upgrade prices are green under the icons; the detail pane
  shows "Upgrade N gold (full M)", "Builds from" and "Builds into"; the
  tooltip shows the price you would pay now.
- **Verified** with `--demo-shop`: iron sword upgraded for 250g of 550g,
  steel for 300g of 850g, undo returned 300g and the iron sword.

## 2026-11-24 — Sell at 70%, Undo at 100%

- **Rates** (user-approved): selling refunds 70% of cost, rounded down
  (`items::SELL_REFUND_PERCENT`, `sell_price`); it was 50% in the old HUD.
  Undo refunds 100%.
- **Undo** takes back this shop visit's purchases newest first: an item in
  the bag loses its stats and is removed; an item still waiting in the
  delivery chest leaves the delivery list (`AnimatedScene::cancel_stored`).
  An item the guard has since handed over becomes a bag purchase for Undo.
  Selling an item drops its Undo entry so it cannot be refunded twice.
  Closing the shop ends the visit (`ItemShop::sync_visit`).
- **Live shop**: Buy and Sell share the detail pane's bottom row (Sell only
  answers when the item is in the bag); "In your bag: N   Sells for X"
  above them; an Undo button on the foot line.
- **Verified** with `--demo-shop` (now grants 8000 gold and finishes with a
  sell and an undo in one frame): "sell refunded 1680g (70% of 2400), undo
  refunded 1700g (of 1700)".

## 2026-11-23 — Shop icon grid in tiers; the player's face on the minimap

- **Shop** (ROADMAPPLAN phase 3, first slice, layout only): the centre of
  the Grand Exchange is now an icon grid in four sections, Starter, Early,
  Core and Late, cheapest first, with each item's price under its icon
  (gold when affordable, pale when not) and the selection outlined in gold.
  Tiers come from cost (500 / 1200 / 3000 gold) unless an item in
  `data/items/items.json` names its own `tier`. Drawing and clicks share
  one layout (`rift_ui::hud::shop::grid`), and the wheel scroll stops at
  the end of the grid (`max_scroll`). No gameplay change: selling, undo and
  recipes are still to come.
- **Minimap portrait** (phase 2.2): the player's ring holds a 16 px disc
  cut from the baked portrait (`portrait.map`). Hero rings are kept inside
  the map picture, so the one at the fountain is no longer cut in half.
  Bots have no hero identity yet, so their rings stay plain.

## 2026-11-23 — Lighting in data, minimap view trapezoid

- **`data/lighting.json`** now holds the world look: terrain and unit
  ambient, sun strength, shadow floor, the tint and saturation of unseen
  ground, and grass saturation, brightness and mottle
  (`crates/client/src/lighting.rs`, three new uniform blocks). **F5**
  reloads it together with the scene shader, so the look can be tuned in a
  running client. Unknown keys and out-of-range values are refused with an
  error and the old values stay. Verified by rendering with an extreme copy
  (frame brightness fell from 75 to 59) and with the shipped file (same
  look as before).
- **Minimap view trapezoid** (ROADMAPPLAN phase 2.4): the four screen
  corners are cast onto the ground (`screen_footprint`) and drawn as the
  camera's true footprint, wide at the far edge, clipped at the map edge.

## 2026-11-23 — Team fog of war and a darker, deeper light

- **The world shows what the team sees.** The line-of-sight grid
  (`VisibilityGrid`, refreshed every 0.15 s) is uploaded as a 512x512 R8
  texture (`Renderer::set_vision_grid`, bindings 8 and 9) and sampled with a
  five-tap soft lookup in `scene.wgsl`. Ground no ally sees drops to about
  40% brightness and loses most of its saturation; walls and trees cast
  sight shadows. Menus and dev demos keep the old circle round the hero
  (`clear_vision_grid`).
- **Lighting retune** (ROADMAPPLAN phase 1.2): terrain ambient 1.64 to 1.30
  (units 1.74 to 1.48), sun 0.72 to 0.90, shadow floor 0.52 to 0.36,
  post-grade saturation 0.95 to 0.86.
- **Grass palette** (phase 1.3): the lime "dry" field tint is now olive and
  the grass saturation boost drops from 1.3 to 1.12, darker overall.

## 2026-11-23 — Reference study and live minimap vision

- **Roadmap plan.** [ROADMAPPLAN.md](ROADMAPPLAN.md) compares MobaScape with
  a gameplay video of another OSRS-asset MOBA (70 stills, every 15 s) and
  plans seven phases: map light and colour, minimap, shop, hero select,
  arena set dressing, HUD polish and post-match. Rule: every visual comes
  from the OSRS cache.
- **Minimap shows the team's vision live.** Ground no ally sees sits under
  a dark veil, sampled from the line-of-sight grid (`VisibilityGrid`) at
  127x126 cells, so wall shadows show too. Lit patches follow allied
  heroes, minions and towers as they move.
- **Hero rings.** Heroes are drawn as team-coloured rings (the player in
  gold) over minions and camps instead of the same dot as a minion.

## 2026-11-23 — Main menu, hero spotlight and wielded weapons

- **Main menu.** The game (and the AppImage) boots into a menu in the
  style of the early RuneScape front end: gold MOBASCAPE title between two
  pixel-flame braziers, a stone tablet with BOT MATCH, PUBLIC MATCH,
  PRIVATE MATCH and QUIT, over a slow orbit of the arena playing itself.
  Mouse, arrows/WS + Enter, 1-3 and Esc work. Public and Private are
  marked OFFLINE: there is no match server yet, so they run the bot match.
- **Hero select.** A 30 s countdown dial locks in the highlighted hero at
  zero (beeps for the last five seconds). Heroes are grouped by type
  (MELEE / RANGED / MAGIC sections, with filter tabs). The centre stage is a
  spotlight: the selected hero, rendered from the cache in real time, plays
  its cast (or attack) animation as an entrance and then idles, swaying
  under a light beam. Stats, the wielded weapon and the four abilities are
  on the right. Esc returns to the menu.
- **Wielded weapons.** Humanoid heroes hold the first weapon in their
  inventory, merged from the item's cache wield models so it moves with the
  hand. With no weapon bought they hold a cosmetic default: Kolodion a staff
  of fire, Wise Old Man a staff of air, the Ranger a toxic blowpipe. Monster
  heroes use their own skeletons and cannot wield.
- **Live portrait.** With `--ui-live`, locking in a hero rebakes the team
  frame portrait for that hero.
- **Tools.** `rift-tools item` prints wield models; `rift-tools wielders`
  lists the items that use given models. `--start-select` opens hero select
  directly for screenshots.

## 2026-11-23 — Kit abilities sound like what they do

- **Per-action sounds.** Every kit action now makes a shared sound when it
  runs (`Action::sound_name` in `crates/client/src/kit.rs`), replacing the
  single cast whoosh for the twelve kit champions. All ids are RuneLite
  `SoundEffectID`s, so buffs use the OSRS prayer chimes:
  | action | sound |
  |---|---|
  | projectile | 1930 whoosh, pitched up |
  | dash | 1930 whoosh, pitched down |
  | area (on landing, each repeat) | 3791 SMITH_ANVIL_TONK, deep |
  | zone | 2596 FIRE_WOOSH |
  | heal buff | 2678 Rapid Heal |
  | shield / damage reduction / reflect / untargetable | 2687 Steel Skin |
  | attack or move speed | 2667 Incredible Reflexes |
  | other buff | 2690 Thick Skin |
  | summon | 200 TELEPORT_VWOOP |
  | channel | 2670 Mystic Will (each tick plays its inner action) |
  | empower | 2691 Ultimate Strength |
  | transform | 200 TELEPORT_VWOOP, lower |
- **Overrides.** The names (`kit_projectile`, `kit_heal`, ...) live in
  `data/sounds.json`; a champion's `sounds` entry with the same name
  overrides them. A champion file that scores a slot itself (any `q_*` key)
  silences the shared action sounds for that slot and plays its `q_cast`
  instead. Hand-written kits (King Awowogei, the Ranger) are unchanged.
- **Tools.** `rift-tools sounds --ids 1930,2678` describes sound ids directly
  (and `--out` writes WAVs); all 17 ids in `data/sounds.json` decode in the
  pinned cache.
- Verified with debug cue traces in `--demo-kit --timescale 4`: Cerberus
  W/E/R gave 2596, 1930, 3791; Graardor 200, 2687 + 3791, 1930; Venenatis
  1930 (+3791 Pounce landing), 200, 2596; King Awowogei kept his own sounds.
  New tests: every action type used by a champion file has a table entry,
  and buffs pick their strongest effect. 361 workspace tests pass; clippy
  clean. Not yet heard by a person; volumes are a first pass.

## 2026-11-23 — Every champion makes sound: shared combat sounds and cache frame sounds

- **Cache frame sounds.** `Sequence::sounds_between` (`crates/cache/src/
  sequence.rs`) returns the opcode-14 sounds whose frame starts inside a time
  span, wrapping looping clips and playing each sound at most once per pass.
  The scene snapshots every unit's base and one-shot clip clocks at the top of
  `AnimatedScene::update` and plays the frame sounds of what each visible
  unit actually animated that tick, within each sound's cache audible range.
  Graardor's swing (3843) and stomp (3834) and K'ril's stomp (3834) now come
  straight from the cache; K'ril's claw (3865) sits a third of the way into
  its swing and is lost when movement cancels the swing early. Hidden units are silent, so sound
  never leaks through fog.
- **Shared sound table.** New `data/sounds.json` names the sounds every unit
  falls back to, all documented RuneLite `SoundEffectID`s: `melee_hit` /
  `ranged_hit` 2498 (ATTACK_HIT), `blocked_hit` 511 (ZERO_DAMAGE_SPLAT),
  `player_hurt` 510 (TAKE_DAMAGE_SPLAT), `ranged_release` / `magic_release` /
  `cast` 1930 at different pitches (NPC_TELEPORT_WOOSH), `shop_buy` 3924
  (GE_COIN_TINKLE), `shop_sell` 3928 (GE_COLLECT_BLOOP), `skill_point` 2266
  (UI_BOOP), `structure_fall` 2734 (TREE_FALLING) and `frame_sound` (volume
  only, for cache frame sounds). A champion file's `sounds` entry with the
  same name overrides one for that champion; enemy champions use the shared
  table only.
- **When sounds play.** Like the reference client, only fights the player is
  in make hit noise (the player struck or was struck), plus visible champion
  duels; lane minion and tower trades stay quiet. A champion's basic-attack
  shot whooshes on release (magic heroes lower). Abilities get the cast
  whoosh unless the champion file scores that slot itself (`q_*`, `w_*`, ...),
  so King Awowogei keeps his own kit sounds. One copy of a sound id plays per
  frame, so an area spell striking five minions is one hit.
- Verified with `RUST_LOG=rift_client=debug` cue traces in `--demo-duel
  --timescale 4` as Graardor (3843 swings, 2498 hits, 510 hurt), K'ril (3834
  stomps) and the Blowpipe Ranger (1930 shots, 510 hurt). New tests:
  frame-sound windows and wrapping (`rift-cache`), and the shared table naming
  every sound the scene asks for. 359 workspace tests pass; clippy clean.
  Not playtested by ear; volumes are a first pass in `data/sounds.json`.

## 2026-11-23 — Cache-native legacy HUD icons and ability art

- **Only cache visuals.** `crates/client/src/ui_assets.rs` no longer reads
  `assets/osrs-ui`. The legacy atlas is built from cache sprites and cache
  model renders: fonts, hitmarks, skull, swords, champion ability icons and
  every shop item's inventory icon.
- **Item icons.** `main.rs` bakes every item in `data/items/items.json` with an
  `obj` id through `item_icons::bake` and packs the 32px renders under the item
  id. The Fountain Shop, detail pane and inventory now use OSRS item art in
  the default HUD, not coloured placeholders.
- **Ability icons.** Every champion's four cache ability sprites are packed
  under `icon.ability.<champion>.<slot>`; the legacy ability bar asks for the
  same role as the live theme. Score-strip swords (442) and skull (439) are
  also cache sprites.
- **Docs.** `docs/UI_ASSETS.md` now documents the cache-native pipeline and
  retires the external PNG tree. The procedural panels/buttons that have no
  clean cache sprite remain geometry only.
- Verified at 1280x720 with `--demo-shop --timescale 4`, `--demo-camp` and the
  normal HUD; `cargo test --workspace` and `cargo clippy --workspace
  --all-targets -- -D warnings` pass.

## 2026-11-23 — Assembled hitmarks, PK markers and max hits

- **`--ui-live` parity.** `crates/ui/src/hud/hitmarks.rs` draws the same cache
  splats and bold numbers through the assembled interface's theme atlas.
  `HudState` gained `hitmarks: Vec<HitmarkState>` and `ui_live::hitmarks`
  projects the live `PlayerHud` hitmarks into it. The theme file now names
  `icon.hitmark.damage/poison/disease/venom/heal/miss/max` and `icon.skull`
  from cache sprites 1359/1360/1361/1632/1629/1358/3571/439.
- **Camp plates.** `PlateKind::Camp` joins the assembled unit plates: a 96px
  bar, the camp's buff-coloured name, and its level chip on the right. The
  legacy and assembled paths share `hud::nudge_unit_frames`, so overlapping
  camp and hero plates are pushed apart in both.
- **PK marker.** Enemy champions get the white OSRS PK skull (sprite 439) and
  a yellow `NO SAFE` tag above a 2x name, matching the reference video's
  skull/label/health-bar stack. The skull and tag are in both HUDs.
- **Max-hit splat.** `HitsplatKind::MaxHit` uses the gold-trimmed 24x24 max
  damage splat (sprite 3571). King Awowogei's Bananarang critical now shows
  it instead of a plain red damage splat.
- **Tinted hits.** Champion hits that did not come from the local player use
  the reference client's dark cool tint (about a third brightness), while the
  local player's hits keep the cache's original red/green/teal colours.
  Splats from minions, towers and camps stay untinted.
- Verified at 1280x720: `--demo-hitsplats` (all seven kinds), `--ui-live
  --demo-duel` (skull, NO SAFE, 2x names, tinted incoming hits), `--ui-live
  --demo-camp` (camp name/level plate and hitmarks), and `--demo-duel` in the
  legacy HUD. `cargo test --workspace` and `cargo clippy --workspace
  --all-targets -- -D warnings` pass.

## 2026-11-23 — Cache-native hitmarks and overhead plates

- **Fonts.** `crates/client/src/ui_assets.rs` now decodes `p11_full` (494),
  `p12_full` (495) and `b12_full` (496) from the cache's font measurements
  (index 13) and glyph sheets (index 8), then shelves the glyphs into the UI
  atlas as white tintable quads. `p12_full` is the default interface/overhead
  font; `b12_full` is used for hitmark numbers. The built-in 5x7 font remains
  the fallback when a cache has no font archives.
- **Hitmarks.** The exact OSRS 24x23 sprites are baked from index 8: red
  damage (1359), green poison (1360), orange disease (1361), teal venom
  (1632), the purple heal cross (1629) and the blue miss (1358). They are
  drawn at 2x with the cache's bold 12px digits, a black shadow, and a 1.2 s
  life. Hits on one unit stack down the body; two-digit numbers use the full
  3x text size, longer numbers step down to stay legible.
- **Combat plumbing.** `FloatText` now carries a `FloatStyle::Hitsplat`
  kind. Damage uses the red splat, blocked/zero hits use blue, venom ticks use
  teal, and the E heal uses purple with the amount actually restored. A
  `--demo-hitsplats` flag pins one of each kind over the hero for visual
  checks; `--demo-fx` shows a three-hit combo.
- **Overhead plates.** Hero and jungle-camp plates now draw larger solid names
  in the cache's `p12_full` font: blue ally, red enemy, warm orange for Might
  camps, green for Swift camps, bone white for structures. The level chip
  moved to the right edge of the health bar (gold number on a dark slot) for
  heroes and camps. The old dark name box is gone.
- Verified at 1280x720 with `--demo-hitsplats --spawn 2770 3000`, `--demo-camp
  --timescale 4` and `--demo-duel --timescale 4`; hero select still lays out
  correctly with the cache font. `cargo test --workspace` and
  `cargo clippy -p rift-client --all-targets -- -D warnings` pass.

## 2026-11-23 — King Awowogei's new kit and the first sound effects

- **Sound.** `rift_cache::sound` ports the reference client's synthesised
  sound effects (index 4: instruments, envelopes, gating, echo and the
  moving IIR filter, with the `java.util.Random(0)` noise table) and
  renders 16-bit 22,050 Hz audio. All 11,171 sounds in the pinned cache
  decode and render. Sequences now decode their opcode-14 frame sounds
  (1,429 animations have them). `crates/client/src/audio.rs` plays queued
  cues through `rodio` with distance falloff (silent past 20 tiles) and
  stereo panning, trimming the silent lead-in some sounds have. Champion
  files name sounds in `sounds`; `--mute` turns it off.
- The OSRS server, not the cache, picks combat sounds, so no cache data
  says which sound a throw makes. The king uses documented RuneLite ids:
  teleport whoosh 1930 (throw, pitched-up whirl, swing), attack hit 2498,
  item pickup 2582 (catch), item drop 2739 (landings), magic splash boing
  227 (slip), teleport vwoop 200 and prayer-off vwoop 2663 (Greegree),
  tree chop 2735 (slam). Swap them in `data/champions/monkey.json`.
- **King Awowogei** (`animated/awowogei.rs`), approved direction "Greegree
  Trickster":
  - Q Bananarang, rebuilt: thrown at the cursor (it used to follow a fixed
    path in front of him). The same cache banana curves out on his right,
    eases into a hang at 1300, then steers home on his left with a turn
    rate and speed that grow until it is caught in his hand wherever he
    walked. Hits every enemy going out and again for 50% coming back;
    rank still raises crit chance and the crit stun. Spins with a tilt and
    leaves a trail; whirl, hit and catch sounds.
  - W Vine Swing: a long vine drops from the canopy, he runs and jumps on,
    swings along a curved arc that rises over the terrain in its path and
    lands softly at the cursor (900-1125, furthest walkable spot).
    Untargetable from the cast until 0.5 s after landing (towers, minions,
    champions and projectiles ignore him; damage is zeroed).
  - E Banana Peel: lobbed onto a ground circle (850-1000; not unit
    targeted). Landing under an enemy slips it at once; otherwise it
    waits 25 s as a trap (three at most). A slip deals 50-125, stuns 1 s
    and sends the victim sliding and spinning the way it was walking.
  - R Greegree: 8 s as the Ape Atoll gorilla (NPC 5275's models 4810/4812,
    idle 1401, walk 1399, attack 1402): +250/400/550 health, attacks hit
    1.7x as hard and 1.35x as slowly, and every attack slams enemies within
    220 for 60/95/130.
  - Trial by Combat, its arena, spectator monkeys, the tower exemptions and
    the permanent win bonus are gone (`--demo-trial` became
    `--demo-awowogei`). Royal Dash and King's Decree stay as the fallback W
    and E for kit-less champions.
- Balance report: Awowogei's burst went from 0.21x to 0.46x of the melee
  median; return hits and slams are not modelled. Not playtested; the
  ability icons are still the old Royal Dash / Decree / Trial sprites.

## 2026-11-23 — Paper balance report and first balance pass

- `rift-client --balance-report docs/BALANCE.md` (new `balance.rs`) works
  out every champion's effective health, basic and ability DPS, burst,
  shields/heals and mana per rotation at level 1 / rank 1 and level 11 /
  top ranks, with the game's formulas. It flags numbers at least 1.35x or
  at most 0.7x their group's median. It needs no cache or window.
- Approved tuning from the first report:
  - Monkey and Ranger (hand-written kits) now get +5% ability damage per
    level like every kit hero (`hand_damage_mult`; tooltip and report
    follow).
  - King Awowogei: Bananarang 38/52/66/80 -> 45/65/85/105 (`q_damage` 45).
  - Vorkath: Rapidfire channels 2 s instead of 2.5 s (8 fireballs at 35
    instead of 10 at 34); ranks 340/480/620 -> 280/380/480.
  - Kolodion: Claws of Guthix cooldown 7..5.5 -> 8..6.5 and Saradomin
    Strike 5..4.25 -> 6..5.25 (kit cooldowns match rank 1).
- Still flagged after the pass (see docs/BALANCE.md): Awowogei's burst is
  0.2x the melee median (Trial's 3x duel damage is not modelled), Vorkath
  1.6x and Kolodion 1.7x on ability damage, Cerberus 1.8x (lava pools are
  counted at full duration), Kree'arra and Wise Old Man low. Jad's
  effective health flag is from grouping a 360-range bruiser with ranged
  heroes. Not playtested.

## 2026-11-23 — Ability ranks, skill points and ability tooltips

- **Ranks:** every ability starts at rank 1; Q/W/E go to rank 4, R to 3.
  Each level gives a skill point. While points are left, the slots that can
  take one glow gold, their hotkey letters pulse and a band of light sweeps
  across them; clicking a glowing slot spends the point instead of casting.
  R's ranks 2 and 3 need levels 6 and 9. Rank pips (gold per rank) replace
  the old placeholder strip under each slot.
- **Hand-authored numbers:** all 64 abilities have damage, range and
  cooldown per rank in their champion file (`abilities[].ranks`, plus
  `damage_label` / `damage_note`). Rank 1 equals the previous numbers, so
  nothing changes until a point is spent. Kits apply them as ratios: damage
  scales every damage, heal and shield; range scales shot length, cast range
  and dash length (indicators follow); cooldown replaces the kit's. The
  hand-written Monkey and Ranger abilities and the HUD cooldown fills follow
  the rank too. The Bananarang's crit and stun now grow with its rank
  instead of champion level. All numbers are a first pass and have not been
  playtested.
- **Tooltip:** hovering Q/W/E/R shows "Name: summary", the rank with pips,
  and a table of the current rank's damage, range and cooldown (green) and
  every rank left (changed values gold, locked R ranks marked "(Lv 9)"),
  plus "Click to spend a skill point (N left)". Damage includes current
  ability power and level.
- New `--demo-ranks SLOT` dev flag. Verified by screenshots of Big Frog Q,
  E and R and Callisto E (a no-damage ability with a note). 333 workspace
  tests pass; clippy clean.

## 2026-11-23 — AppImage packed without external tools

- Packing first needed `appimagetool`, downloaded from GitHub and run, which
  was blocked. The new `crates/appimage` replaces it: a packer
  (`rift-appimage`) that writes the squashfs with `backhand` (gzip only) and
  a runtime (`rift-appimage-runtime`) with the type-2 magic that extracts
  once to a content-keyed cache and runs `AppRun`.
- `tools/appimage/build.sh` now produces `dist/MobaScape-x86_64.AppImage`
  (6.1 MiB) entirely from this workspace.
- Verified: 4 crate tests (ELF offset, pack→extract round trip with modes and
  symlinks, refusing trailing runtime bytes, content key); clippy clean. The
  real AppImage ran from `/tmp`: extracted, rebuilt as a no-op, launched the
  client and wrote a screenshot. Warm start overhead: 23 ms.

## 2026-11-23 — Self-updating AppImage launcher

- `tools/appimage/build.sh` packs `dist/MobaScape-x86_64.AppImage`. On each
  launch it fast-forwards a clean checkout to `sam/main`, rebuilds
  incrementally and runs the new client, falling back to the last good build
  and then to a bundled snapshot. See DEV_TOOLS.md.
- Original icon (three lanes and a rift); no Jagex data inside the image.
- Verified: the launcher runs the client from any working directory
  (0.18 s no-op build). On a clone one commit behind, it fast-forwarded to
  c50f884 before launching.

## 2026-11-23 — Big Frog ability icons

- The Big Frog's bar used Vardorvis's leftover sprites. It now has its own
  green set picked from the cache sprite sheets (`rift-tools dump-sprites`):
  60 (tendrils) Tongue Lash, 364 (falling green comet) Big Leap, 205 (swamp
  herb) Swamp Croak, 332 (three falling bolts) Plague of Frogs.
- Verified by `--ui-live` screenshot: all four read clearly on the bar.
- Balance checked on paper only: 580 HP, 28 armour, 15 damage, 270 range
  sits inside the melee band. Still needs a hand playtest.

## 2026-11-23 — Attack animations, reach and projectile effects

- **Attack and cast animations:** champion files name `attack_anim`,
  `attack_hit` and `cast_anim`. The clip is time scaled so its hit frame
  lands when the wind-up ends, blends over idle/walk and is cancelled by
  walking. Wind-up now shortens with attack speed. Imps (169) and fairies
  (110) swing too, and minions, camps and enemy champions wind up a strike
  before the blow or shot lands. Clips were picked from pose sheets
  (`rift-tools skeleton-anims`), e.g. Graardor 7018, Cerberus 4491,
  Vorkath's breath 7952, Kree's wing blast 6978, the ranger's blowpipe 5061.
- **Reach:** attack range is edge to edge on the ground plane (target
  footprint `body_radius`, not the generous projectile hit box, and no
  height penalty on terraces). Melee champion ranges drop by 50 to keep
  hero-vs-hero reach about the same; towers, the nexus and big camps are
  now reachable from their edge. A committed swing tolerates 90 units of
  drift. The shared meteor is clamped to 1,100 units (it had no range).
- **Projectiles:** basic attacks, minion bolts and tower shots home and
  cannot miss; abilities stay skillshots. Shots carry a look from
  `fx::LOOKS`: an OSRS spot animation model (fire/water/earth/wind
  blasts and waves, dragonfire breaths, Jad's fireball, trident bolt,
  Armadyl crystal, darts, arrows, team-coloured tower orbs), a layered glow,
  a ribbon trail, embers, a point light, and an impact flash, shock ring,
  sparks and hit-splash model. Arrows and darts lob. Jad now attacks with
  his fireball (`ranged_auto`). Melee hits draw a bright slash.
- **Additive FX pass:** `fs_fx` in `scene.wgsl` draws glows, ribbons,
  translucent cache faces and ground indicators with additive blending after
  the opaque scene, feeding bloom. Particles and effect rings moved into it.
- **Cache:** spot animation decoder (archive 13, all 3,565 decode) and
  per-face transparency on models (`face_alphas`).
- **Ability indicators:** casting stays instant; holding right mouse shows
  the last cast ability's shape at the cursor (skillshot lanes, fans,
  target circles inside a range ring, cones, dash arrows, Bananarang's
  path), greyed while on cooldown. Every cast briefly flashes its range.
- **Big Frog replaces Vardorvis**, whose Animaya model could not animate:
  NPC 8701 (model 6947), tongue-lash attack 1793, Tongue Lash pull, Big Leap
  slam, Swamp Croak slow and shield, Plague of Frogs rain plus three frogs.
- Fixed while here: player basic-attack last hits never counted as CS, and
  allied ranged-minion kills paid out hero rewards (both passed no
  attacker).
- Verified: workspace tests and clippy pass; screenshots of homing fireballs,
  tower orbs, Vorkath's breath, the cast clip and the four indicator kinds;
  `--demo-abilities` for Big Frog; benchmark 152 fps at 1280x720.


## 2026-11-23 — Every unit faces the way it moves

- OSRS models are authored facing -Z, while a unit's `yaw` points +Z along
  its movement. Only King Awowogei (`model_yaw: π`) and the imp template
  carried a correction, so every other unit (the twelve new heroes, fairies,
  camps, ambient creatures, summons) walked backwards. The earlier imp-only
  fix treated a convention as an exception.
- The renderer now adds `CACHE_MODEL_FORWARD` (π) for every unit; the two
  special cases are removed. `model_yaw` remains for real exceptions.
- Verified from geometry (Cerberus's heads sit at model -Z; Callisto's body
  extends toward -Z) and in game: Cerberus and Callisto lead with their
  heads walking left, right, toward and away from the camera; King Awowogei
  still faces forward. Heavy champions take a moment to finish turning.

## 2026-11-23 — Jad and Kree'arra get the kits their descriptions promised

- Both used the shared bolt / blink / heal-shield / meteor despite their
  hero-select text. They now carry kits matching that text, with the same
  costs and cooldowns: Jad's Guard slows nearby enemies, Teleport is an
  untargetable blink, Miner's Endurance heals and shields, Earth Splitter
  knocks up and scatters boulders; Arcane Lance bursts on impact, Kree's
  Leap is a long dash, Flechtalon Guard heals and shields, Skyfire leaves
  burning ground. Fourteen of sixteen champions now run on kits; King
  Awowogei and the Blowpipe Ranger keep their hand-written abilities.
- `--demo-abilities`: both cast all four abilities and kill the test camp.

## 2026-11-23 — Twelve new champions on a data-driven kit system

- **NPC decoder:** opcode 249 (the parameter map) aborted nearly every
  modern boss definition. It is now decoded, unlocking the God Wars
  generals, Cerberus, the wilderness bosses, Vorkath and more. Barrows
  brothers still fail on an unknown opcode 6.
- **Kit system** (`kit.rs`, `animated/kit_runtime.rs`, `docs/KITS.md`): a
  champion file can describe its passive and Q/W/E/R as data. Actions:
  projectile, area, zone, dash (with landing actions), buff, summon,
  channel, empower, transform. Statuses: slow, root, stun, knock-up, fear,
  taunt, knockback, pull, venom, bleed, heal cut, mana drain, armour shred,
  %-current-health damage and stacking chill. Ten passive kinds. Charges,
  untargetable dashes, damage reduction, reflect and a second form are
  supported. Hooks: incoming damage, projectile damage, basic attacks,
  minion damage, XP and movement. Crowd control moves into every unit's
  `cc` (slows and roots in `move_unit_toward`/`follow_lane`, fear and forced
  movement in the AI loop, taunt in `find_target`); summons are minion units
  with a lifetime that follow the champion between fights.
- **Roster (approved by the user):** General Graardor, Cerberus, Kalphite
  Queen, K'ril Tsutsaroth, Callisto, Vardorvis (since replaced by Big
  Frog), Venenatis, Commander
  Zilyana, Ancient Wyvern (replacing Zulrah), Kolodion, Vorkath and the Wise
  Old Man, generated with cache models, sprite icons and kits. 16 champions.
- **Hero select:** portraits are 128px and wrap onto two atlas rows, so all
  16 bake (only 7 fitted before); the detail panel sits below two rows.
- **`--demo-abilities`:** all twelve cast all four abilities against a
  jungle camp: every camp hurt, eleven killed (Zilyana's R is an empower and
  needs a basic attack), summons spawn for Graardor, the Kalphite Queen,
  Venenatis and Vorkath, Kolodion's demon form takes over, statuses land.
- Known: Vardorvis's model is a skeletal (Animaya) model with no vertex
  groups, so he moves without animating until Animaya is decoded. Support
  kits heal and shield only the caster: there are no allied heroes yet.
  Ability numbers are first-pass and untested against enemy heroes.
- Verified: 296 workspace tests pass; clippy clean.

## 2026-11-23 — Live interface click routing

- Under `--ui-live`, only the shop and the inventory's Shop button took
  clicks. A click on the minimap, ability bar, objectives, chat, team frames
  or target panel fell through to the world, so the hero walked to the
  ground hidden behind the panel.
- Now:
  - **Minimap:** a click on the picture pans the camera there and turns off
    follow, like the old HUD. `ui_live::world_from_map` is the tested
    inverse of `map_point`; `minimap::picture_at` skips the stone frame.
  - **Ability bar:** clicking Q/W/E/R casts like the hotkey
    (`ability_bar::ability_at` matches the drawn slot geometry).
  - **Every other panel** swallows its clicks. A claimed press never sets
    `lmb_down`, so hold-to-steer cannot start on a panel either.
  - The old HUD's minimap rectangle no longer catches clicks under
    `--ui-live`, where it is not drawn.
- Tests: slot and picture hit geometry; minimap click round-trips a dot to
  within one map pixel. Verified: 289 workspace tests pass; clippy clean.
  Not verified with a live mouse: the only input tool on this Wayland
  desktop (`ydotool`) would drive the user's real cursor.

## 2026-11-23 — Live ability bar: cache spell icons and the champion portrait

- The `--ui-live` ability bar drew the word "HERO" and plain Q/W/E/R
  letters. It now shows the cache-baked champion portrait (`portrait.self`)
  and one cache spell/prayer sprite per ability, with the hotkey as a small
  corner badge. Letters remain the fallback when a role is missing.
- Data-driven: each champion JSON has `ability_icons` (four cache sprite
  IDs). The theme bakes every champion's set as
  `icon.ability.<name>.<slot>`, so the bar is right after hero select too.
  - King Awowogei: 57 (returning arc), 53 (swoop), 122 (heart), 168 (crossed swords)
  - JalRek-Jad: 21 (fire strike), 33 (teleport rune), 130 (heart), 131 (red pillar)
  - Kree'arra Jr.: 26 (lightning), 53 (swoop), 132 (blue pillar), 36 (fire burst)
  - Blowpipe Ranger: 45 (green orb), 58 (scatter), 61 (wind), 133 (eagle eye)
- `AbilityState` gains `portrait` and `icons`; `theme_file` accepts extra
  cache sprites by role.
- Tests: stable icon roles across punctuation; every champion ships four
  icons. Verified: 286 workspace tests pass; clippy clean.

## 2026-11-23 — Cliff dressing: riser outcrops and plateau tree masses

- Terrace risers (slope 0.8–1.6) were outside the slope boulder window
  (0.10–0.58), so cliffs were bare and plateau tops were empty lawns.
- New deterministic per-tile pass after the lane tree lines:
  - **Riser outcrops:** 191 grey cache boulders (model 1805) at 7% of tiles
    steeper than 0.7, scaled 0.6–1.15× and sunk 30×scale into the face.
    Boulders 685/1773 were dropped here: at outcrop scale they read as
    pale cream domes (a first try at 24% density and 1–2.1× scale buried
    the jungle in 1,212 of them).
  - **Plateau tree masses:** 365 trees via `place_tree` (same species, tint,
    shadow proxies and collision as the lane lines) on plateau tops at
    least 1.5 tiles back from the lip, clustered by a slow noise field so
    canopies form masses with clearings between them.
- 1920×1008, live UI, auto scale: p50 **13.2 ms / 75.6 fps**, p95 14.2 ms,
  GPU frame 12.7 ms (scene 11.45). Model mesh 979k triangles (was 860k).
- Verified: 284 workspace tests pass; clippy clean.

## 2026-11-23 — Dark green slabs: backdrop plane, cache sea, shore tint

The flat, untextured dark-green patches near the blue base and along the
river were **not sun shadows** (an earlier note blamed the shadow proxies;
that was wrong: in lighting, shadow only scales the sun term while ambient
dominates, so sun shadows are subtle). Diagnosed with a temporary
material-ID debug view: the slabs were material 0, the distant **backdrop
plane**, at a fixed y = -24. Wherever the terrain dipped below -24 (carved
banks, low ground, the old sea floor), the plane drew on top, outlined by
the -24 contour.

- **Backdrop** now sits 60 units below the lowest terrain vertex.
- **Cache sea reclaimed.** The rift's 5x5 block includes open sea off
  Karamja (underlay-free tiles with textured water overlay 133). Inside the
  castle bounds 8,114 such tiles, plus pocket ponds, become land: mean land
  colour, no overlay, height harmonically filled from the shores.
- **Shore tint.** River blue is applied only under the water sheet; the
  band beyond becomes a wet-soil bank instead of blue the shader classified
  as near-black water.
- **Grass variants** are brightness-normalised toward the classic sprite in
  linear light (30% of each variant's own tone kept): steppe/straw lifted
  1.2×, dry dimmed 0.6×.
- **Pocket scenery** skips flat ground-plane models (none in the current
  pockets, but they would render as slabs and block movement).
- New `tools/shots/shadows.txt` preset list (lit + shadow views).
- GPU at 1920×1008 (live UI, auto scale): frame **12.3 ms**, scene
  **11.05 ms**. Wall-clock p50 was noisy (16–31 ms) under a desktop load
  average of ~5.
- Known: a small black rectangle near tile 2850,3040 (a cache object) and
  a dark cross shape in the river near 2845,3065 remain.
- Verified: 284 workspace tests pass; clippy clean.

## 2026-11-23 — Terrain performance recovery: baked noise, adaptive mesh

- **Baked grass noise.** Tileable three-octave value noise is written into
  the (unused) alpha channel of every grass layer at load. The terrain
  shader reads mottle from the two rotated grass samples it already takes
  and the lush/dry macro field from a mip-5 sample at a 6400-unit repeat,
  replacing three procedural `noise2` calls (12 hashes per pixel); cliff
  strata use one `sin`. Test: the bake tiles seamlessly and spans a range.
- **Adaptive terrain.** A tile gets the 64-unit grid only where its fine
  heights leave the coarse quad's plane by more than 3 units (cliff lips,
  risers, banks) or its ground identity changes inside it; 17.9% of tiles.
  Coarse tiles that border refined ones fan around their centre through the
  shared edge midpoints, so the mesh stays watertight. 364k terrain
  triangles (was 819k uniform, 205k before the rework).
- Measured at 1920×1008, live UI, auto scale (scene GPU / p50 / p95):

  | Build | Scene GPU | p50 | p95 |
  |---|---:|---:|---:|
  | Pre-rework baseline | 11.3 ms | 13.0 ms | 13.9 ms |
  | Uniform 64-unit + procedural noise | 14.3 ms | 16.2 ms | 17.6 ms |
  | Uniform 64-unit + baked noise | 13.1 ms | 14.9 ms | 15.5 ms |
  | 128-unit + baked noise (reference) | 10.7 ms | 12.8 ms | 15.3 ms |
  | **Adaptive + baked noise (shipped)** | **12.0 ms** | **13.7 ms / 73 fps** | **14.6 ms** |

- Verified: 283 workspace tests pass; clippy clean; cliff and jungle captures
  show no cracks at refined/coarse borders.

## 2026-11-23 — Terraced terrain, 64-unit ground mesh, grass anti-tiling, imp facing

- **Noise bug fixed.** `value_noise` hashed the cell plus a corner flag
  instead of the neighbouring lattice point, so the field jumped at every
  integer line. Every noise-driven hill, lane wobble and biome edge had
  creases. A regression test checks continuity across lattice lines.
- **Calm floor.** `sculpt_terrain` no longer adds broad/medium/ridge/fine
  hills everywhere. It compresses cache tile bumps to 30% around a 10-pass
  relaxed copy and adds one very broad swell (±75) plus a ±17 medium octave.
- **Terraced plateaus.** `plateau_height` cuts a warped low-frequency field
  at two contour levels (210-unit steps) with narrow risers and rock noise
  only on the riser band. `plateau_clearance` keeps lanes (4.5–9 tiles),
  the river, camps, base courtyards and the castle wall bands on the floor.
  About 13% of the map is raised.
- **64-unit display terrain.** Heights are a Catmull-Rom upsample of the tile
  grid plus plateaus at full resolution; colours and masks are bilinear; quads
  split along the flatter diagonal. `HeightGrid` carries its sample step, so
  units, decor and structures sit on the finer surface. 819k terrain
  triangles (was 205k), chunk-culled.
- **Softer ground edges.** Overlay weights blend between same-texture tile
  centres per cell; biome grass variants are chosen per cell on a smooth
  noise contour instead of dithering whole tiles (no more square patches).
- **Grass shader.** Two grass samples at unrelated rotations and scales
  (430/1030 units) cancel the mirror-tile seam grid; detail is pulled 30%
  toward the sprite mean; a two-octave mottle and slow lush/dry patches give
  scale; palette shifted from lime/olive toward the reference's deeper green.
- **Cliffs read as stone.** Rock starts at slope 0.13–0.32, with horizontal
  strata, and the vertex-colour tint no longer greens cliff faces.
- **Imps walked backwards.** Imp model 19982 faces the opposite axis to the
  fairy (like King Awowogei). `MinionTemplate::model_yaw` is now π for imps
  and 0 for fairies; verified with timed captures of the first wave.
- Performance (live UI, 1920×1008 auto scale): p50 **16.2 ms / 62 fps**,
  p95 17.6 ms, GPU scene **14.3 ms** (was 11.3). Over budget headroom; the
  extra grass samples/noise and the 4× terrain triangles are the cost.
- Known: sun shadows from oversized tree/wall proxies blanket large areas
  near the blue base in hard-edged dark green; units can walk up cliffs
  (no cliff collision yet — a gameplay decision).
- Verified: workspace tests pass (client 69 incl. 6 new terrain tests);
  workspace clippy clean.

## 2026-11-23 — Stop shallow-water flicker and lift low-ground readability

- The water pipeline now uses a small negative depth bias and the water sheet
  sits 0.75 units above the sampled river level, so a shallow bed and the
  water plane no longer alternate depth winner as the camera moves.
- Removed the bridge-corridor terrain flattening that produced a broad flat
  dark pad; the deck is now a flush cache-wood boardwalk, the dry bridge shelf
  is narrower, and the surrounding ground keeps its natural shape.
- Low-ground readability: the darkest terrain channels are lifted toward a
  shadow floor, dark biome grass is gamma-lifted in the shader, overlay art is
  brightness-matched and weighted lower, biome edges are wider and
  noise-warped with lifted tints, and 205 cache reeds/ferns/mushrooms now
  line the banks. The water shader adds a shallow shore rim.
- Captures: `/tmp/opencode/flickerfix.png` (bridge) and
  `/tmp/opencode/flickerfix_high.png` (wide lowland view).
- GPU time remains healthy: auto 1920×1008 frame **12.57 ms** (scene 11.30),
  native frame **14.24 ms** (scene 13.01). CPU benchmark figures were noisy
  because of the desktop load.
- Verified: 272 tests pass, 4 ignored; workspace clippy clean.

## 2026-11-22 — Cache-textured bridge decks

- Levelled the three river crossings into flat bridge corridors after lane
  grading, then laid cache-textured wooden decks over them. Each deck follows
  the lane tangent and is sized to span the carved ford; low cache-wood rails
  give it a bridge read instead of a floating slab.
- Texture IDs are cache scraped: planks from `texture_0022`, rails from
  `texture_0005`. No new visual assets.
- Visual capture: `/tmp/opencode/bridgedeck10.png`. The deck is a first-pass
  geometry/art solution; stone abutments and richer rail detail can follow.
- Performance: auto 1920×1008 p50 **12.56 ms / 79.5 fps**, p95 13.28 ms,
  GPU **12.16 ms** (scene 10.94); native p50 **14.65 ms / 68 fps**, p95
  15.53 ms, GPU **14.25 ms** (scene 13.02).
- Added a bridge-tangent test. Verified: 272 tests pass, 4 ignored; workspace
  clippy clean.

## 2026-11-22 — 64-unit river surface

- The river was one quad per terrain tile, so every bank read as a 128-unit
  staircase. It now builds from the water mask at half-tile (64-unit)
  resolution: 111,318 vertices / 37,106 triangles, with mask-driven opacity
  at the shoreline.
- The surface covers the full carved mask again, including bends; a
  centreline ribbon was tried first but left dry wedges on the inside of
  bends, so it was discarded.
- Visual capture: `/tmp/opencode/subwater1.png`. A known remaining gap is
  the dry bridge/ford presentation (`/tmp/opencode/subwater2.png`); a proper
  cache-textured bridge deck is the next water task.
- Performance is unchanged: auto 1920×1008 p50 **12.99 ms / 77 fps**, p95
  13.67 ms, GPU **12.43 ms** (scene 11.22); native p50 **15.21 ms / 66 fps**,
  p95 16.05 ms, GPU **14.56 ms** (scene 13.34).
- Added a bilinear grid test. Verified: 271 tests pass, 4 ignored; workspace
  clippy clean.

## 2026-11-22 — Cache overlay ground textures

- Terrain vertices now carry a packed per-tile ground identity: low byte is
  the grass variant, bits 8–15 are the cache overlay texture ID, bits 16–23
  are the blend weight. The shader samples that cache texture from the model
  texture array and blends it over the classified grass/soil/rock.
- Base tiles and biome pockets both feed the packed overlay, with pocket
  edges fading the overlay weight with the existing 7-tile blend. This uses
  real OSRS path/dirt/biome ground art instead of only approximating it from
  blurred vertex colour.
- Visual result: lane dirt and the pocket cliffs now carry actual cache
  ground texture instead of smooth brown gradients. Captures:
  `/tmp/opencode/overlay_mid.png` and `/tmp/opencode/overlay_pocket.png`.
- Tried and reverted: alpha-tested foliage cutouts. The cache canopy sprites
  use sparse alpha that is not a cutout mask; preserving it punched blue sky
  holes through every canopy. The opaque average-colour fill is correct for
  this cache. No visual assets were added.
- Performance after the overlay sample: auto 1920×1008 p50 **12.98 ms /
  77 fps**, p95 13.86 ms, GPU **12.57 ms** (scene 11.31); the overlay adds
  about 0.3 ms GPU. Native remains above 60 fps.
- Added a packing test for the overlay/variant byte layout. Verified:
  270 tests pass, 4 ignored; workspace clippy clean.

## 2026-11-22 — World-art first slice: cache ground, sculpted terrain, ground cover

- **Ground sprites now come from the cache directly.** `GrassTextures` and
  `TerrainTextures` resolve cache index 9 → index 8 (`129/203/18/197/195/205`
  for the six grasses and `129/32/10/38/1/126` for the base layers); the local
  `assets/osrs-ground` folder is only an override. Verified by moving that
  folder away and rendering the cache path successfully.
- **Sculpted terrain.** Replaced the ±26/±9 rolling relief with warped broad
  hills, medium, ridged and fine octaves, a castle wall/base settlement mask,
  and 88% lane grading. Water and base flattening remain authoritative.
- **Grass and soil detail.** One mip-correct grass sample at a 384-unit repeat
  with per-sample tone, triplanar soil/rock projection at 128 units,
  slope-driven grass/soil/rock classification and a steeper cliff override.
  The post grade is less desaturated and the sun/ambient balance is slightly
  more directional.
- **Cache-only ground cover.** 1,839 deterministic low-poly flowers, ferns,
  grass and mushrooms, plus 244 slope boulders, all selected from cache object
  definitions. No new visual assets; the static model mesh is 835k triangles
  (674k before).
- **Performance.** Auto 1920×1008: p50 **12.79 ms / 79 fps**, p95 13.53 ms,
  GPU **12.21 ms** (scene 10.99). Native: p50 **14.82 ms / 69 fps**, p95
  15.59 ms, GPU **14.14 ms** (scene 12.92). The texture/sculpt/shader pass
  costs about +0.9 ms GPU against the previous flat fallback frame; the
  external AppRun/Brave desktop load still makes some CPU runs noisy.
- Captures: `/tmp/opencode/terrain_before_realtex.png` (flat fallback),
  `/tmp/opencode/terrain_v7_mid.png`, `/tmp/opencode/terrain_grade_spawn.png`
  and `/tmp/opencode/terrain_river.png`.
- Added deterministic relief/water and castle-settlement tests. Verified:
  269 tests pass, 4 ignored; workspace clippy clean.
- Full reference audit and revised milestone order:
  `docs/REFERENCE_AUDIT_2026-11-22.md`; the world-art plan is in
  `docs/MILESTONE_PLAN.md`.

## 2026-11-22 — Live team frames show the player with a cache-baked portrait

- `ui_live::allies` now returns the player first, then any other allied
  champions. With one hero a side the team panel is no longer empty.
- Under `--ui-live` the selected champion is baked once through the existing
  cache-model portrait rasteriser and added as the `portrait.self` theme
  sprite. `team_frames` stretches it into the 36px slot with the new
  `DrawList::sprite_fit`; unpainted allies and the sample theme keep the
  team-coloured initial fallback.
- No new visual assets: the portrait is rendered from the scraped cache model.
  Capture: `/tmp/opencode/team_frame_self.png`.
- Added two `sprite_fit` primitive tests, a portrait-fit test and a
  team-frame portrait pixel test. Verified: 267 tests pass, 4 ignored;
  workspace clippy clean.

## 2026-11-22 — Bloom runs at 320×180

- The three bloom passes now work at **320×180** instead of 640×360: 16.7% of
  1920×1008 and 25% of 1280×720. At the default 0.24 bloom weight the glow is
  visually unchanged, but the chain no longer costs a millisecond-scale
  fraction of the frame.
- Interleaved 1920×1008 auto A/B: bloom bright/h/v **0.36/0.19/0.19 ms →
  0.20/0.04/0.04 ms**, total **0.74 → 0.28 ms**. The GPU frame fell from
  12.34 ms to **11.59–11.77 ms**, p50 from 12.77 ms to **12.16–12.29 ms**
  (81–82 fps) and p95 from 13.62 ms to **13.16–13.24 ms**.
- Native 1920×1008: bloom **0.80 → 0.31 ms**, GPU frame **13.86 → 13.68 ms**,
  p50 14.46 ms / 69 fps and p95 15.48 ms. 1280×720: bloom **0.61 → 0.27 ms**,
  p50 7.65 → **7.39 ms / 134 fps**.
- Visual A/B: `/tmp/opencode/bloom640_demo.png` against
  `/tmp/opencode/bloom320_demo.png`, both with the live demo kit active. Same
  scraped cache assets; no visual assets were added.
- Tried and not kept: replacing the post-pass sin dither with a cheaper hash;
  it did not lower the post time, so the existing dither stayed.
- Verified: 263 tests pass, 4 ignored; workspace clippy clean.

## 2026-11-22 — Auto scale spends the new headroom on sharpness

- `--render-scale auto` now targets **1.6M scene pixels** instead of 1.4M.
  On a 1920×1008 inner window that is 0.91× (1746×917), up from 0.82×
  (1578×828), so the default world is visibly sharper while keeping a
  comfortable margin.
- Startup now derives the auto scale from the window's actual inner size,
  not the requested size. A 1920×1080 request with a 1008px client area no
  longer undershoots the pixel budget.
- Measured at 1920×1008, live interface, MSAA 4, shadows, immediate:
  p50 **12.32 ms / 81.1 fps**, p95 **13.58 ms**, p99 14.63 ms, GPU frame
  **11.51 ms** (scene 9.82, bloom 0.78, post 0.77, shadow 0.05). The 0.82×
  frame was p50 11.11 ms / 89.7 fps; the extra ~0.7 ms of scene work buys a
  22% larger scene image.
- Native 1920×1008 is unchanged at p50 14.35 ms / 69.7 fps, so `--render-scale
  1.0` remains available for maximum sharpness.
- Visual A/B: `/tmp/opencode/scene_auto16.png` (0.91×) against
  `/tmp/opencode/scene_lightgate.png` (0.82×). Both use the same scraped cache
  terrain and models; no visual assets were added.
- Added a test that the auto scale lands on the 1.6M-pixel budget. Tried and
  not kept: a one-tap castle grain projection, which did not move the scene
  time (12.14 → 12.29 ms) and was reverted. Verified: 263 tests pass, 4
  ignored; workspace clippy clean.

## 2026-11-21 — Point lights only shade fragments inside their radius

- `dynamic_lighting` now compares squared distance against the light radius
  and skips `normalize`/`dot` for every fragment outside it. Previously those
  fragments still ran the full computation and multiplied by an attenuation
  that was already zero. The fountain is the usual single light, so this was
  a large per-fragment cost across most of the screen.
- Visuals are unchanged by construction: the contribution outside the radius
  was already zero. Capture: `/tmp/opencode/scene_lightgate.png`. No new
  visual assets were introduced.
- Native 1920×1008, MSAA 4, shadows, immediate: p50 **16.23 → 14.35 ms**,
  p95 **17.17 → 15.47 ms**, GPU frame **15.90 → 13.86 ms**, scene pass
  **14.19 → 12.14 ms**. Native is now comfortably above 60 fps.
- Auto scale 1920×1008: p50 **12.61 → 11.11 ms** / **89.7 fps**, GPU frame
  **12.00 → 10.76 ms**, scene pass **10.33 → 9.11 ms**.
- Tried and not kept: a direct one-light shader fast path and gradient-based
  grass mip selection; neither moved the measured scene time, so both were
  reverted rather than kept as unmeasured complexity.
- Verified: 262 tests pass, 4 ignored; workspace clippy clean.

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
- The legacy HUD now uses the cache's `p12_full`/`b12_full` fonts and OSRS
  hitmarks, max-hit variants, tinted other-champion hits, PK skull/NO SAFE
  tags and jungle-camp plates. The assembled `--ui-live` interface reads the
  same cache-native theme fonts and sprites; its dev layout labels remain
  visible when run with `--ui-live`.
- Both HUDs are cache-native for visual assets: fonts, hitmarks, skull,
  score-strip icons, champion ability icons and item icons all come from the
  pinned cache. Panels/buttons without a clean cache interface sprite keep the
  procedural OSRS-styled geometry. The old `assets/osrs-ui` tree is retired for
  visuals.
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
