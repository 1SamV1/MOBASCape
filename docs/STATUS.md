# Status

Last updated: interface stage 3 step 12, 2026-11-20.

## Working

- Rust workspace builds; `cargo test --workspace` passes. Clippy still reports
  pre-existing lints in `crates/sim`, `crates/cache` tests and the frozen
  stage 2 drawing layer; they are not from the stage 3 work.
- Cache reader: all 25 indices, reference tables, XTEA, gzip/bzip2/LZMA,
  archive splitting with real file IDs.
- Terrain/locations/object/NPC/underlay/overlay configs.
- Model formats 2 and 3 with flat-shading normals, vertex groups, texture refs.
- Sequence/frame/framemap decoding and interpolated CPU vertex animation.
- `crates/sim`: deterministic 192×192 tile grid, A* with octile heuristic,
  no corner cutting, nearest-walkable fallback, path simplification, LOS.
- Scene: 3×3 Karamja block, 37,249 terrain vertices, 900 scenery placements,
  8 ambient animated NPCs, three stone lane ribbons.
- Map: top/mid/bottom lanes, two towers + one inhibitor per lane per team, one
  nexus per team, four jungle camps, four brush rectangles.
- Movement: click-to-move A* path following; hold-to-steer with collision
  sliding; camera follow/pan; selection and click rings.
- Lane: three-minion waves per lane every 25 s; minion AI, tower targeting,
  inhibitor-gated nexus, victory/defeat.
- Jungle: four neutral camps with respawn; Might (+25% damage) or Swift (+20%
  speed) team buffs for 60 s.
- Champion roster: three data-driven configs in `data/champions/`:
  - **Monkey** — balanced, 500 HP / 200 mana.
  - **JalRek-Jad** — tank/mage, 700 HP / 260 mana, higher damage.
  - **Kree'arra Jr.** — ranged skirmisher, 420 HP / 240 mana, ranged auto-attacks.
  Load with `--champion monkey|jad|kree`.
- Kit: Q projectile, W blink, E heal+shield, R meteor; mana, cooldowns, shield
  absorption, expanding rings; ability power from items scales Q/R/E.
- Progression: levels 1–11, XP, gold; level-ups raise health, mana, damage.
- Item shop: six data-driven items in `data/items/items.json` (bronze sword,
  iron platebody, amulet of fury, berserker ring, Ahrim's staff, dragon boots),
  six inventory slots, fountain-range purchase, gold spending, and flat
  damage/health/mana/speed/ability-power stats. `B` toggles the shop, `1`–`6`
  buys, clicking a row buys, clicking outside or `Esc` closes.
- Fog/brush: 192×192 visibility grid; hidden enemies/neutrals; dark fog mesh;
  minimap hides unseen units.
- HUD: health/mana/XP/shield/buff bars, four ability boxes, six inventory slots,
  shop panel with item-colour swatches/cost bars/owned markers, nexus bars, wave
  timer, crosshair, live minimap; numeric state in the title.
- Interface stage 2: `crates/ui` drawing layer (sprites, nine-slice panels,
  bitmap text, rectangles, clipping, whole-number scale), the client's UI pass,
  and the theme loader. `--ui-test` draws its test screen over the game.
- Interface stage 3 step A: the stage 3 theme roles in `data/ui/theme.json`
  (stone, dark and parchment panels, slot and button frames, four fonts,
  icons, map dots and 15 colours); `--ui-theme-sheet` draws every role by
  name and the pieces together. `panel.stone` uses the single stone panel
  1031: the 913–936 and 952–997 sprites are scrollbar and tab parts, not a
  panel frame kit.
- Interface stage 3 step B: `HudState`, `sample.rs`, the scale by fit, the
  placing pass and the collapse order in `crates/ui/src/hud/`. `--ui-layout`
  draws every component as a labelled empty rectangle over the game. The
  old `hud.rs` is still the default and still runs the match.
- Interface stage 3 step 1: the score strip is drawn in `--ui-layout` at its
  placed rectangle: stone panel, blue and red team marks and kills, crossed
  swords, yellow clock, own kills/deaths/assists and creep score.
- Interface stage 3 step 2: the ability bar with health and resource is
  drawn in `--ui-layout`: portrait and level box, Q/W/E/R slots with a
  cooldown overlay, D/F battle spells, four small item slots, health with a
  recent-damage stretch, and resource.
- Interface stage 3 step 3: the minimap is drawn in `--ui-layout`: stone
  frame, sample river and three lanes, base and structure rings, camp and
  unit dots, camera box and corner button.
- Interface stage 3 step 4: the objectives panel is drawn in `--ui-layout`:
  title and minimise placeholder, team names and lane tower/inhibitor pips,
  rule, two nexus lines and four camp lines with quieter second lines. Its
  height follows the nexus and camp counts; title-only collapse draws just
  the title.
- Interface stage 3 step 5: the team frames are drawn in `--ui-layout`:
  four rows with portrait squares and level boxes, names, health and
  resource bars, two spell marks, and a grey death row with respawn seconds.
- Interface stage 3 step 6: the target panel is drawn in `--ui-layout`:
  portrait square, name in the team colour, kind tag, health with figures,
  an orange description line, and eight stat figures in two columns of four.
  `TargetState` now carries the team, description and stat figures. The team
  frames' step B label is no longer drawn under them either.
- Interface stage 3 step 7: unit plates are drawn in `--ui-layout`:
  heroes carry a team-coloured name, level box and health bar, minions a thin
  blue or red bar, structures a wider bar. They are clipped to the room
  above the anchor the world gives them, sit on layer 0 under every fixed
  component, and take no part in the layout, clash test or budget. Stage 3
  places them from sample fractions of the window; the live projection and
  hiding are stage 4.
- Interface stage 3 step 8: the inventory is drawn in `--ui-layout`: six
  slots in two rows of three, three filled with item colours and initials,
  three numbered empty, the gold pile and count, and a Shop button. It is
  drawn only when the placing pass shows it, so the narrow-window collapse
  takes it away with the rest of the bottom row.
- Interface stage 3 step 9: the chat and event log is drawn in
  `--ui-layout`: parchment, six wrapped sample lines with system lines dark,
  match events blue and player names in their team colour with the hero in
  brackets, a scrollbar and the All/Game/Public tabs. When the placing pass
  cuts the chat to its last line, only that line is drawn. The drawing layer
  gains a pure paragraph wrap in `crates/ui/src/wrap.rs`.
- Interface stage 3 step 10: the kill feed is drawn in `--ui-layout`: two
  dark entries, each with a bar in the killer's team colour, portraits with
  initials, both names in their team colours, the crossed swords and the gold
  won. Entries stack at the layout's 30-pixel pitch and are clipped to the
  rectangle the placing pass gives them.
- Interface stage 3 step 11: one sample announcement is drawn in
  `--ui-layout`: the title font in yellow with a dark one-pixel outline,
  centred on the window and sitting on the measured 23% baseline. The
  drawing layer gains outlined text in `crates/ui/src/outline.rs`.
- Interface stage 3 step 12: the tooltip is drawn in `--ui-layout`: a dark
  panel beside the sample cursor, kept inside the window, with the title and
  price in yellow, green stat lines, the passive name in orange and its text
  wrapped in white. Its height follows the wrapped text and its width stays
  310; it is always on the top layer and takes no part in the budget.
- Dev demos: `--demo-target`, `--demo-combat`, `--demo-camp`, `--demo-kit`,
  `--demo-shop`, `--champion`, `--timescale`, `--ui-test`,
  `--ui-theme-sheet`, `--ui-layout`, `--ui-scale`.

## Verified

- `cargo test --workspace`: 182 tests pass, 2 ignored (the optional pictures
  of the theme sheet and the labelled layout). The 15 new layout tests check
  the 1920 by 1080 rectangles, the collapse at 1280 by 720 and 1366 by
  768, the scale edges, the 953 by 280 minimum, requested scales, 7/8/9 clearance, the objectives'
  target-panel place at a forced scale, the bar and chat widths, the 38%
  rule, a 23 by 16 window sweep with no clash, determinism and the quarter.
- Stage 3 theme sheet: loaded from the real cache, seven panels, four fonts,
  22 sprites and 15 colours; written headlessly to `target/theme-sheet.ppm`
  for the eye. The in-game GPU check is still to do.
- Stage 3 labelled layout: written headlessly to `target/layout-1920.ppm` at
  1920 by 1080, scale 1, usual coverage 16.5%, no clashes and no steps; the
  score strip, ability bar, minimap, objectives, team frames, target panel,
  unit plates, inventory, chat, kill feed, announcement and tooltip in it
  are the real components, the rest are labelled rectangles.
- Score strip tests: within its rectangle at scales 1 to 4; no pixel outside
  the rectangle; a theme with only a font still draws panel fallback and
  text.
- Ability bar tests: within its rectangle at scales 1 to 4; no pixel outside
  the rectangle; a theme with only a font still draws panel fallback, slots
  and bars.
- Minimap tests: within its rectangle at scales 1 to 4; no pixel outside the
  rectangle; a theme with no panels or sprites still draws the fallback.
- Objectives tests: full and title-only containment at scales 1 to 4; no
  pixel outside the full rectangle; a theme with only fonts still draws the
  panel fallback.
- Team frame tests: containment at scales 1 to 4; no pixel outside the
  rectangle; a theme with only fonts still draws the fallback.
- Target panel tests: containment at scales 1 to 4; no pixel outside the
  rectangle; a theme with only fonts still draws the panel fallback.
- Unit plate tests: every kind stays in the room above its anchor at scales 1
  to 4; no pixel outside the three rooms; a theme with only fonts still
  draws the fallback.
- Inventory tests: containment at scales 1 to 4; no pixel outside the
  rectangle; a theme with only fonts still draws the fallback.
- Chat tests: full and last-line containment at scales 1 to 4; no pixel
  outside the rectangle; a theme with only fonts still draws the fallback.
  The wrap tests cover words at the last fitting space, a word broken between
  characters, empty text, exactly the width and a zero width.
- Kill feed tests: two entries contained at scales 1 to 4; no pixel outside
  the rectangle; a theme with only fonts still draws the fallback.
- Announcement tests: centred on the 23% baseline at scales 1 to 4, inside
  the window, and a theme without a font draws nothing. The outline test
  checks the dark draw is one pixel down and right.
- Tooltip tests: beside the cursor and inside the window at scales 1 to 4,
  turning away from all four corners, no pixel outside its rectangle, longer
  passive text making it taller and not wider, and the font-only fallback.
- **Item shop demo**: granted 4000 gold; bought all six items; final title
  `HP 730/730`, `MP 260/260`, `gold 300`, `dmg 45 spd 460`, `items 6/6`,
  ability power 45%.
- **Partial shop capture**: after two buys, shop rows render item colours,
  owned markers, cost bars, inventory slots, and gold header; title shows
  `gold 3300`, `dmg 22`, `items 2/6`.
- **JalRek-Jad**: title `HP 700/700`, `MP 191/260`, W/E/R cast true.
- **Kree'arra Jr.**: title `HP 420/420`, `MP 170/240`, W/E/R cast true.
- Fog run: `visible_enemies=7–13` while the rest of the map is dark.
- 20× three-lane run still reaches victory states; jungle Might buff still works.

## Known issues

- Stage 3 is groundwork, not the new HUD: several components are still
  empty labelled rectangles and the old `hud.rs` remains the default. The
  theme sheet and labelled layout have only been checked as headless
  pictures, not yet in the running game on the GPU. The unit plates in
  `--ui-layout` use sample window fractions; the client's live projection,
  visibility and the replacement of the old 3D bars are stage 4. The stage 3
  inventory draws sample items and gold; its Shop button is inert until
  stage 4. The chat is sample text with no typing and no live match events
  until stage 4. The kill feed draws two sample entries; live kills and the
  fade are stage 4. The announcement uses the title font at its own size; the
  reference's letter height is still unconfirmed (plan section 5.10). The
  tooltip shows one sample item at a sample cursor position; following the
  real cursor is stage 4.
- Sprite names on the theme sheet are shortened (`icon.` and `map.dot.` are
  omitted) to fit; the full role names are in the theme file and the load log.
- Item purchases are client-side in this prototype. The authoritative server
  must own gold, shop range, inventory, and stat application before multiplayer.
- No item selling, combining, passives, or active item effects; six flat-stat
  items only.
- Shop rows and HUD have no text/font: item names and costs live in the window
  title (hovered row included), not on-screen.
- Champions share the Q/W/E/R shape; only stats/models/ability numbers differ.
  No champion-select UI or passive.
- Kree'arra/Jad models are large; scale tuning is ongoing.
- Fog/brush is client-side only; server-authoritative fog is required for
  multiplayer.
- Fog has hard tile edges and some tall trees/allied vision pockets poke through.
- No minimap fog overlay or click-to-pan.
- Camps are passive and single-model; composite NPCs not assembled.
- No inhibitor respawn/super minions or base/fountain visuals.
- Lane ribbons can clip under steep terrain in a few spots.
- Collision is approximate; textures are flat fallbacks; Animaya not applied.
- No lobby, netcode, audio, or map editor.

## Next tasks

1. Run `--ui-theme-sheet` and `--ui-layout` on the GPU, compare them with the
   reference screenshots, and fix what the eye finds.
2. Stage 3 step 13: the shop overlay, then the death screen, and the whole
   together.
3. HUD text/font atlas for the old shop names/costs, gold, HP/MP, level and
   labels until the new components replace it.
4. Item selling/combining/passives and five champions with distinct kits.
5. Server-authoritative fog + minimap click-to-pan + brush visuals.
6. Inhibitor respawn/super minions and base/fountain visuals.
7. Deterministic 30 Hz sim tick in `crates/sim` + state hash.
8. Texture sampling and better foliage/trunk colours.
9. Lobby + QUIC netcode after the sim loop is stable.

## Verification log

- `cargo test -p rift-sim`: 7 tests (straight path, wall detour, nearest
  walkable, no corner cutting, simplification, bounds, LOS).
- `cargo test -p rift-cache`: 6 tests (smart reads, chained smart, XTEA, HSL,
  group rejection, framemap/frame parse).
- `cargo test -p rift-client`: 4 item tests (row hit-testing, six-slot fill,
  affordability, owned-slot colours).
- Shop demo: 6/6 items, dmg 45, HP 730/730, MP 260/260, speed 460, AP 45%,
  gold 300.
- Champion runs: Jad 700 HP / 260 mana; Kree 420 HP / 240 mana; W/E/R true.
- Fog run: visible enemies 7–13; dark fog mesh over unseen tiles.
- Three-lane 20× run: victory states, towers/inhibitors/nexus live.
- Jungle demo: camp killed, buff Might 58s.
