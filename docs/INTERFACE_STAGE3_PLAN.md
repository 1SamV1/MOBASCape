# Interface stage 3: plan for the static interface

Status: **a plan for review, third draft. Nothing in it is built, and
nothing in it is authorised to be built.** The second draft answered two reviews of the first: the layout worked
out as rectangles from the anchors and not as sums of widths, the screen
budget defined, the scale chosen by whether the interface fits, the scope
narrowed. This draft answers the reviews of the second: one clearance
figure corrected and one rule for clearance used everywhere, the three open
decisions recorded, the largest content sizes and other shapes of window
added to the arithmetic, and the layout settled in two passes. What changed
is listed in section 12. It was asked for after stages 1 and 2 of
`docs/INTERFACE.md` passed. It does not change that document; where it would,
the change is listed under "Changes this plan asks for" and waits for review.
`crates/client/src/hud.rs` is untouched until a later go-ahead.

Stage 3, as the design has it: every component in its place with made-up
values, drawn through the stage 2 drawing layer from a theme. No component
reads the match yet. That is stage 4.

## 1. The reference, and how far it can be trusted

Five screenshots of the reference game, supplied by the user, each 1920 by
1080:

| Short name | What it shows | Match clock |
|---|---|---|
| S1 select | hero selection, full screen | before the match |
| S2 early | play, mid lane, nothing selected | 0:15 |
| S3 fight | play, first kill, night colours | 1:18 |
| S4 death | the death screen, a structure selected | 7:27 |
| S5 shop | the shop open over play, a tooltip, an announcement | 11:27 |

They are kept out of the repository: they are pictures of someone else's
game and of Jagex artwork.

**They are not sharper than the recording the design was measured from.**
I said earlier that they were three times its resolution. Enlarged, every
edge in them is a soft ramp about three pixels wide: they are the same 640 by
360 recording shown at three times the size. What they add is three screens
the design had no frame of (S1, S4, S5), an announcement, a tooltip and a
target panel, and coordinates on a 1920 by 1080 grid that are easy to work
in. What they do not add is detail.

So, for every figure in this plan:

- **A position or size is good to about 4 pixels** on the 1920 by 1080 grid.
- **Nothing smaller than about 6 pixels can be measured**: border widths,
  the gaps between slots, the exact height of a line of text.
- **No small text can be read with certainty.** Where a label is given below
  it is what the blur most plausibly says, and it is marked when it is a
  guess.

Figures are written as *left to right, top to bottom* on the 1920 by 1080
grid. "Measured" means read off an enlarged, gridded crop. "From parts" means
worked out from the real sizes of the cache sprites and fonts that would make
the component; these are proposals, to be confirmed when the component is
first drawn.

## 2. Sizes, the scale, and the budget

In the reference, at 1920 by 1080, the interface is drawn at the artwork's
own size: an inventory cell is about 40 pixels, which is a 36 by 32 item
sprite and its frame; text is about 12 to 16 pixels tall, which is the
regular font at its own size. **The reference is scale 1 at 1080p**, so the
measured sizes are the interface's own sizes.

The first draft added those sizes up along rows and columns. That was wrong
in kind: the ability bar is centred, so what it needs is not its width but
twice the room on its crowded side. Everything in this section comes
instead from placing every rectangle by its anchor and margins and testing
the rectangles against each other. The arithmetic is in
`docs/interface_stage3_layout.py`, which prints every figure quoted here;
appendix B has its output. At step B of the build it is replaced by tests on
the real placing code.

Two of the sizes are no longer the reference's, because the reviews decided
the content is our own (section 5.4 and 5.7): the objectives list is about
290 tall where the reference's is 350 to 400, and the inventory is our six
slots, about 144 by 160 where the reference's eighteen cells are 232 by
224. Both are sizes worked out from parts and are proposals until drawn.

### Two checks, kept apart

**1. Do pieces clash?** Two pieces clash if they are closer than 8 of the
interface's own pixels. This is geometry and has no opinion.

**2. How much of the battlefield is covered?** The budget.

### What the budget measures

- **What is counted:** the area of the window inside the rectangles of the
  fixed components that are showing, each pixel counted once however many
  rectangles cover it.
- **In which state:** the usual one, which is every fixed component but the
  target panel, with one kill-feed entry. The target panel and further feed
  entries come and go with play; counting them would make the layout change
  when the player clicks on something, and a panel would hide itself by
  appearing. The busy state (target panel and four feed entries) is
  reported beside it and has to be free of clashes, but does not trigger
  anything.
- **What is not counted:** unit plates, which belong to the world;
  announcements and the tooltip, which are brief and belong to the moment;
  full-screen interfaces and the dimming behind them, which cover the
  battlefield on purpose.
- **Rectangles, not artwork.** A rectangle's clear corners and see-through
  panels count in full. Counting only solid pixels would be truer to the
  eye and would make the budget depend on the theme's artwork; the plan
  keeps to rectangles and says so. The figure is therefore a little high.
- **The limit stays a quarter**, as the design has it.

Because the usual state depends on nothing but the window's size, the layout
is a pure function of window size and of which conditional pieces are
showing: the same inputs always give the same layout, and growing a window
undoes exactly what shrinking it did.

### The scale is chosen by fit

The scale is the largest whole number, up to 4, at which **the whole
interface fits with nothing given up**: no clash even in the busy state at
its largest (the target panel, four feed entries, the longest objectives
list), with the ability bar centred, and the usual state within the budget.
If that is true of no scale, the scale is 1 and section 7 applies. Width
and height are tested each on its own; nothing assumes a 16:9 window.

| Window | Scale | Usual state covers | Busy state covers | What gives |
|---|---|---|---|---|
| 1280 by 720 | 1 | see section 7 | | collapse in force |
| 1366 by 768 | 1 | see section 7 | | collapse in force |
| 1600 by 900 | 1 | 23.5% | 30.5% | nothing |
| 1920 by 1080 | 1 | 16.3% | 21.2% | nothing |
| 2560 by 1440 | 1 | 9.2% | 11.9% | nothing |
| 3200 by 1800 | 2 | 23.5% | 30.5% | nothing |
| 3840 by 2160 | 2 | 16.3% | 21.2% | nothing |

For 16:9 windows the whole interface first fits at scale 1 from 1554 by 874,
at scale 2 from 3106 by 1747 and at scale 3 from 4658 by 2620. In a sweep of
5,824 windows from 960 by 384 to 3840 by 2400, with width and height varied
independently, the scale never fell as a window grew. The player may force
a scale in settings; a forced scale too big for the window gets the
collapse order, as the stage 2 test screen showed it can.

So the design's hope that everything shows in full at 1280 by 720 does not
hold, and the corrected figure is about 1554 by 874, with 1600 by 900 the
smallest common size that has it all. 1280 by 720 stays supported, with
collapse.

By clashes alone, without the budget, everything fits from 1364 wide and 776
high. Between that and 1554 by 874 nothing clashes and the budget alone
makes pieces give way.

### One rule for clearance, and the figures it gives

Two pieces are clear of each other when there are at least 8 of the
interface's own pixels between them. Every figure in this plan, in the
script and in its output uses that rule. With the ability bar centred:

| The bar clears | From a window this wide | Why |
|---|---|---|
| the minimap alone | 1021 | the bar ends at half the width plus 229; the minimap starts 274 from the right |
| our inventory (144 wide) | 1321 | the inventory starts 424 from the right |
| the chat at its full 440 | 1364 | the chat ends 445 from the left; the bar starts at half the width less 229 |
| the reference's inventory (232 wide) | 1497 | it would start 512 from the right |

So the least width for the whole interface, 1364, is set by the **chat on
the left**, not by the inventory. The second draft said the inventory set
it, and gave 1481 for the reference's inventory, which was a figure with no
clearance at all from an earlier run. Both are corrected here.

### The least window

The first draft said the ability bar and the minimap fit side by side from
about 740 wide. That is true only if the bar may leave the centre; centred,
it is 1021. With every step of section 7 taken, nothing clashes down to 953
wide (the chat's last line, the bar slid left, and the minimap) and 280 high
(the minimap alone, top to bottom). Below 953 by 280 the window refuses to
shrink. 1280 by 720 remains the smallest size the game supports; the figures
below it only say that nothing breaks on the way there.

## 3. How it is built

### No large file

```
crates/ui/src/
  hud/
    mod.rs            the list of components, layout, collapse; nothing drawn here
    state.rs          HudState: everything the interface shows, as plain data
    layout.rs         anchors, the placing pass, the collapse order
    sample.rs         made-up HudState values for stage 3 and for tests
    score_strip.rs
    ability_bar.rs    includes health and resource, which are drawn inside it
    minimap.rs
    objectives.rs
    team_frames.rs
    target_panel.rs
    unit_plates.rs
    inventory.rs
    chat.rs
    kill_feed.rs
    announcements.rs
    tooltip.rs
    overlays/
      shop.rs
      death.rs
```

All of it is in `crates/ui`, which depends on nothing, so every component is
tested without the game, the cache or a graphics card, the way stage 2 was.
The boundary, stated once:

- `crates/ui` turns a `HudState` and a theme into a list of quads over one
  atlas. It depends on nothing: not the client, not the cache, not the
  simulation, not the graphics card.
- The client fills `HudState` (in stage 3 from `sample.rs`; in stage 4 from
  the match), builds the theme from the cache, and hands the quads to the
  stage 2 pass. Nothing flows the other way.

### What a component is

Each component file holds three things and nothing else, as rule 2 of the
design says:

1. **Its slice of `HudState`**, a plain struct with no behaviour.
2. **Its size**: a function from that slice and the theme to a width and
   height at the interface's own size. Most are constant; objectives, chat,
   team frames and the tooltip depend on their contents.
3. **Its drawing**: a function that is given a rectangle and draws inside
   it, asking the theme for roles.

A component does not know where it is on the window, what the scale is, or
that other components exist. `layout.rs` is the only code that knows all of
them.

### The placing pass

Each frame, in this order:

1. Ask every component its size.
2. Choose the scale by fit (section 2).
3. Place each at its anchor (section 5), in whole pixels.
4. If pieces clash, or the usual state is over budget, apply section 7.
5. Draw them, lowest layer first.

Unit plates are not part of this pass at all (section 5.6): they are placed
by the world, not by the window.

Steps 1 to 4 produce a list of rectangles and nothing else, so the whole of
layout and collapse is tested by asserting on rectangles at many window
sizes, with no pictures needed.

### The match stays apart

`HudState` is the only thing a component reads, and it is plain data copied
out of the match once a frame (rule 1 of the design). In stage 3 it is
filled from `sample.rs` and nothing else. No component has a way to reach
the match, and no component sends anything: clicks are stage 4. This is
checked the way the design asks: with the interface removed, the match plays
out identically, because nothing in the interface can write to it.

### Sample values are marked

In stage 3 every value on screen is made up. In development builds every
component drawing sample values carries a small "sample" mark in its
corner, so that a made-up kill feed, gold count, ally or objective is never
taken for a working system. The mark goes when stage 4 connects that
component, one at a time.

### Layers

| Layer | What is on it |
|---|---|
| 0 | unit plates (they belong to the world, under everything else) |
| 1 | the fixed components |
| 2 | the kill feed and announcements |
| 3 | overlays that go over a match (shop, death screen) and their dimming |
| 4 | the tooltip, always on top |

## 4. The theme

Components ask for roles. The table gives each role, what kind of thing it
is, and what in the cache could fill it. "Seen" means I have looked at that
sprite in the user's dump and it is what the role needs. "Candidate" means
it is the right kind of thing by size or name and has not been chosen by eye
beside the reference. "Not found" means nothing has been looked for yet; the
role is drawn as a plain coloured rectangle until something is chosen, which
is how the reference draws several of them anyway.

### Fonts

| Role | Used for | Cache font | Confidence |
|---|---|---|---|
| `font.small` | timers, stat figures, bar figures, slot counts | 494 `p11_full` | seen |
| `font.body` | names, chat, panel text | 495 `p12_full` | seen |
| `font.bold` | panel titles, the score strip, button labels | 496 `b12_full` | seen |
| `font.title` | announcements, "Oh dear, you are dead!", screen titles | 497 `q8_full` | seen |

Which of these the reference uses where is judged from letter shapes in the
blur and is uncertain for every piece of small text.

### Panels (nine-slice)

| Role | Used by | Cache | Confidence |
|---|---|---|---|
| `panel.stone` | score strip, ability bar, inventory, minimap frame, team frames, shop, hero select | the stone frame kit, 913 to 936 and 952 to 997, composed by the theme loader into one image; or 1031 (190 by 261 stone panel) | candidate |
| `panel.dark` | objectives, target panel, tooltip, kill feed entries | none: a plain near-black rectangle at about 85% opacity, as the reference shows | seen in reference |
| `panel.parchment` | chat box, hero description in S1 | 1017 (519 by 142 parchment) | candidate |
| `panel.slot` | ability slots, inventory cells | 1077 to 1080 (41 by 40 dark slot) or 170 | candidate |
| `button.stone` | Shop, Teleport, Lock in, Leave, chat tabs | 170 to 176 | seen |
| `button.stone.red` | Teleport when unavailable, the chosen lane in S1 | 177 to 179 | seen |

The stone frame kit is separate corner and edge sprites, not one image.
Stage 2's panels are cut from one image. The theme loader will have to
compose the kit into one image when it builds the theme; that keeps the
drawing layer as it is. This is new work in the loader and is the first
step of the order in section 8.

### Sprites

| Role | Used by | Cache | Confidence |
|---|---|---|---|
| `icon.kills` (crossed swords) | score strip, kill feed | 442 | seen |
| `icon.team.blue`, `icon.team.red` | score strip, objectives | 1065 or 1066 and 1060 (26 by 26 orbs) | candidate |
| `icon.gold` | inventory | 1090 (coin bag) or the coin item sprites | candidate |
| `icon.equip.*` (ten outlines) | inventory rows 1 and 2 | 156 to 166 | seen |
| `icon.minimise` | objectives | not found | not found |
| `icon.close` | shop | not found | not found |
| `orb.ring` | the portrait's ring in the ability bar | 1059 to 1066 as a base | candidate |
| `map.frame` | minimap | 12 `mapback` is the old game's and the wrong shape; use `panel.stone` | candidate |
| `map.dot.*` | minimap | 300 `mapdots` | seen by name |
| `scrollbar` | chat, shop | 316 | seen by name |
| ability, spell and item pictures | ability bar, inventory, shop | not interface sprites: they come with champions and items, later | out of scope |
| portraits | ability bar, team frames, target panel, kill feed, S1 | drawn from each champion's model, later | out of scope |

In stage 3 every ability, item and portrait is a labelled grey square. The
plan does not choose art for systems that do not exist.

### Colours

Read off the reference; each is a role so a theme can change it.

| Role | Colour | Where |
|---|---|---|
| `text` | white | most panel text |
| `text.quiet` | grey | objectives not yet due, stat labels |
| `text.title` | orange | "Objectives", panel titles |
| `text.team.blue`, `text.team.red` | light blue, red | team names, player names by side |
| `text.good` | green | "Alive", item stat lines, hero name over a unit |
| `announcement` | yellow, dark outline | announcements, the clock, gold |
| `death` | red, dark outline | "Oh dear, you are dead!" |
| `bar.health` | green | everywhere health is shown |
| `bar.resource` | cyan in the ability bar, blue in team frames | resource |
| `bar.enemy` | red | enemy plates, the damage breakdown |
| `bar.back` | near-black | behind every bar |
| `shade` | black at about 85% | `panel.dark` |
| `dim` | black at about 50% | behind full-screen interfaces |

The text outline is drawn as the text in the dark colour offset by one
pixel, then the text over it. That is two text draws and needs nothing new
in the drawing layer.

## 5. The components

For each: where it is anchored, its measured box, what is in it, how it
ranks for the eye, what is static in stage 3, and what it will read in
stage 4. Sizes are at scale 1.

### 5.1 Score strip

- **Anchor:** top centre. Measured in S2: 692 to 1226, 10 to 48, so 534 by
  38, centred on 959, 10 from the top.
- **Contents, left to right:** blue team mark, crossed swords, red team mark
  with each side's kills beside its mark (S3 shows "1" and "1"); the match
  clock in yellow at the centre; own kills, deaths and assists as "0/0/0" in
  white; creep score as "0 CS".
- **Panel:** `panel.stone`, one line of `font.bold`.
- **Rank:** quiet. Read on demand.
- **Stage 3:** fixed sample values. **Stage 4:** clock, team kills, own
  score, creep score.
- **Uncertain:** whether team kills are shown from the start (S2 shows marks
  with no figures; S3 shows figures).

### 5.2 Ability bar, with health and resource

- **Anchor:** bottom centre. Measured in S2: 730 to 1188, 968 to 1062, so
  458 by 94, centred on 959, 18 from the bottom.
- **Contents, left to right:**
  - **Portrait:** a circle about 75 across at 740 to 815, 980 to 1055, in a
    ring that fills as experience is gained (blue, part-filled in S2). The
    level is a small box at its lower right; a small icon at its lower left
    is unread.
  - **Abilities:** four slots of about 40, from 822 to 990, 978 to 1018,
    with a thin strip under each (cooldown or rank pips; unread). S3 shows
    figures over some slots: cooldown seconds.
  - **Battle spells:** two slots of about 40 at about 1010 to 1096, the same
    row, set a little apart from the four.
  - **Four small slots** in a two by two block at 1104 to 1180, 975 to 1055:
    item actives or consumables; unread.
  - **Health bar:** 824 to 1094, 1028 to 1044, green on dark, "current /
    maximum" centred in `font.small`. A paler stretch at its end is recent
    damage (red in S3).
  - **Resource bar:** 824 to 1094, 1048 to 1064, cyan, the same.
- **Rank:** with the minimap, the strongest piece of interface.
- **From parts:** portrait 76, four slots of 40 with 2 between, a gap of 12,
  two slots of 40, a gap of 8, a block of 76, and 8 of frame each side: 462
  by 92. Within measurement of 458 by 94.
- **Stage 3:** sample cooldowns, one ability on cooldown, health at 60% with
  a damage stretch. **Stage 4:** abilities and cooldowns, spells, portrait,
  level, experience, health, shield, resource.
- **Health and resource are part of this component**, not a separate one,
  because the reference draws them inside its frame. The design lists them
  apart; see "Changes this plan asks for".

### 5.3 Minimap

- **Anchor:** bottom right corner. Measured in S2: 1646 to 1912, 800 to
  1064, so 266 by 264, 8 from the right and 16 from the bottom.
- **Contents:** the map picture in a stone frame about 6 wide; dots for
  units (blue allies, red enemies, pale for neutral camps); rings for
  structures; a lighter box for what the camera sees; a small round button
  in its top right corner (unread).
- **Rank:** with the ability bar, the strongest.
- **Stage 3:** the picture the client's old interface already draws from
  the lanes, structures and camps, inside the new frame, with sample dots.
  Stage 3 proves the frame, the dots, the scale and the place. It makes no
  new picture of the map; in particular not from the walkable map, which
  describes where a body may go and not what the battlefield looks like.
  **Stage 4:** units the viewer may see, the camera box; clicks to move the
  camera and to order a move.

### 5.4 Objectives

- **Anchor:** left edge. Measured in S2: 8 to 280, 410 to 760; 272 wide, 8
  from the left. Its top is at 410 in all three frames that show it, while
  its height changes, so it hangs from its top. 410 is 38% of 1080, and with
  only one window size in the reference that is all that is known.
- **The rule that is kept is that it clashes with nothing**: not the chat
  below it, not the target panel above it, not the window's edge. Where it
  sits within the room it has is provisional: 38% of the window's height,
  moved up if that would bring it within 8 of the chat. That placing is
  tried at 16:9, 16:10, 4:3, 5:4, 21:9, 32:9 and 9:16 (section 6); it is to
  be looked at in the game at step B and changed if it looks wrong at any
  of them, without the rule changing.
- **Layout from the reference:** a title in orange with a minimise button at
  its right; a small table with a column for each team (its name in its
  colour) and a row for each lane, each cell a pip for each tower and a dot
  for what stands behind them; a thin rule; then a line for each objective
  with an icon, its name, and at the right its timer or state, some with a
  quieter second line. `panel.dark`; row pitch about 24.
- **Content is our game's, not the reference's.** The reference lists a
  dragon, TzKal-Zuk, a mole and others. Ours:

  | Part | Lines |
  |---|---|
  | Lane table | Top, Mid, Bot; for each team two tower pips and an inhibitor dot |
  | Nexus | one line for each team: shielded, open, or its health |
  | Camps | four lines, one for each jungle camp, with what it grants (Might or Swift) and its state or timer |

  That is the title, the table and six lines: about 290 tall, from parts.
  A camp whose buff is running has a second, quieter line under it; with
  all four so, the list is about 370 tall. **370 is the largest this stage
  shows**, and it is the height every clash test uses.
- **Sample states for testing**, each plainly made up: a tower down, an
  inhibitor down, a nexus open, a camp alive, a camp on a timer, a camp
  taken with its buff running.
- **Rank:** dark and quiet until a line changes; a live objective's state is
  green.
- **Stage 3:** the sample states above. **Stage 4:** structure counts and
  camp timers, all of which our game has.

### 5.5 Team frames

- **Anchor:** right edge. Measured in S2: 1728 to 1908, 168 to 356, so 180
  by 188; 12 from the right, top at 168.
- **Contents:** four rows, one for each ally but the player, pitch 47. Each:
  a portrait about 36 square with the level in a small box at its lower
  right; the name; a green health bar and a blue resource bar, each about
  100 by 6; at the right, two battle-spell icons one over the other.
- **Panel:** `panel.stone`, thin.
- **Rank:** quiet; a low health bar is what draws the eye.
- **Stage 3:** four sample allies, one low, one dead (S4 shows a dead ally
  greyed, with its respawn time; unread detail). **Stage 4:** each ally's
  portrait, health, resource, spells, respawn.

### 5.6 Target panel and unit plates

**Target panel**

- **Anchor:** top left. Measured in S4: 8 to 340, 82 to 240, so 332 by 158;
  8 from the left, top at 82. Shown only while something is selected.
- **Contents:** a portrait box about 58 square at the left; the name in the
  team's colour; a kind tag at the right in yellow ("Altar"); a health bar
  about 240 by 14 with figures; a line of description in orange; then eight
  figures in two columns of four. I read the labels as Attack, Attack speed,
  Defence melee, Defence mage, and Power, Range, Defence ranged, Move; these
  are guesses from blurred text.
- **Panel:** `panel.dark`.
- **Stage 3:** a sample structure selected. **Stage 4:** the selected unit.

**Unit plates**

These are a different kind of thing from every other component and are
kept apart in the code.

- **Placed by the world, not the window.** A plate's place is where its
  unit's head falls on the screen, which only the client's camera knows.
  The client does that projection, decides which units' plates are shown,
  and gives `crates/ui` a list of plates already placed in window pixels.
  `crates/ui` draws a plate where it is told and never places one.
- **Visibility and depth stay the game's.** A plate is shown only for a
  unit the game shows: fog, brush and death hide it exactly as they hide
  the bar the old interface draws today. Plates are drawn before the fixed
  components, so a panel covers a plate and never the reverse. A plate is
  not hidden by scenery in front of its unit, which is how the old bars
  behave and how the reference's do.
- **Not in the layout or the budget.** They take no part in placing,
  clashing, collapse or the quarter.
- **Anchor:** above each unit, in the world. Measured over the player's
  hero in S2: 915 to 1003, 395 to 425: the name in green, then a level box
  and a health bar about 80 by 8.
- **Kinds:** heroes get name, level and bar; minions a thin bar about 28 by
  3, blue for allies and red for enemies; structures a wider bar.
- **Stage 3:** drawn at sample screen positions. **Stage 4:** each visible
  unit's name, level, health and screen position. The client's old interface
  already places bars over units; this replaces that drawing only.

### 5.7 Inventory

- **Anchor:** bottom right, left of the minimap, its right edge 6 from the
  minimap's left, 18 from the bottom.
- **What the reference has:** a panel 232 by 224 (measured in S2: 1408 to
  1640, 838 to 1062) holding ten equipment cells in two rows of five, eight
  item cells, a gold count, and Shop and Teleport buttons. Cells are about
  40, at a pitch of about 44 by 42.
- **What this plan draws: our six item slots, and nothing that implies
  more.** Equipment slots, a second kind of item and a Teleport button are
  item-system and game decisions that have not been made. Drawing eighteen
  cells would look like a promise. So: six cells in two rows of three, in
  the reference's cell style and pitch, then the gold pile and count, then a
  Shop button.
- **From parts:** three cells of 40 with 4 between and 8 of frame each side
  is 144 wide; two rows, a gold line and a button line come to about 160
  tall. A proposal until drawn.
- **Panel:** `panel.stone` with `panel.slot` cells.
- **Rank:** quiet, but the gold count is yellow and is read often.
- **Stage 3:** six labelled squares, three of them filled. **Stage 4:**
  items, gold; buy, sell, use, open shop.
- **Clearance:** at 144 wide the centred ability bar is clear of it from
  1321 wide, so it is never what limits the layout; the chat is, at 1364
  (section 2).
- **If the item system later wants the reference's eighteen**, the panel
  grows to the reference's 232 by 224, the centred bar is clear of it only
  from 1497 wide, and the inventory becomes what limits the layout.

### 5.8 Chat and event log

- **Anchor:** bottom left. Measured in S2: the parchment 5 to 445, 930 to
  1045; three tabs under it to 1070. 440 by 140 in all; 5 from the left, 10
  from the bottom.
- **Contents:** five or six lines of `font.body` on parchment, the oldest at
  the top; a scrollbar at the right; a last line that reads "Press Enter to
  Chat" when idle; tabs "All", "Game", "Public".
- **Line colours:** system lines dark; a speaker's name in their team's
  colour with their hero in brackets; match events in blue.
- **Rank:** quiet. The only parchment on the play screen, so it reads as a
  different kind of thing from the stone.
- **Width is elastic:** parchment repeats, so the box can be narrower than
  440 where the bottom row is tight. This is the first thing to give in a
  narrow window, before anything collapses.
- **Stage 3:** six sample lines. **Stage 4:** chat lines, match events;
  sending chat.

### 5.9 Kill feed

Not in the design's list. Seen in S3 and S5.

- **Anchor:** top right corner. Measured in S3: 1622 to 1908, 18 to 46, so
  about 286 by 28 an entry; 12 from the right.
- **Contents:** a bar in the killer's team colour at the left; the killer's
  portrait and name in their colour; crossed swords; the victim's portrait
  and name in theirs; the gold won in yellow ("+300"). S5 shows the same
  shape for a structure: a team name, swords, "Top outer statue".
- **Panel:** `panel.dark`. New entries push older ones down; each fades
  after a few seconds (not measurable from stills).
- **Stage 3:** two sample entries. **Stage 4:** the match's kill events.

### 5.10 Announcements and tooltip

**Announcements**

- **Anchor:** centred, upper quarter. Measured in S5: one line from about
  630 to 1285, baseline near 250.
- **Contents:** one line in `font.title`, yellow with a dark outline. The
  brightest text on the screen, and short-lived.
- **Uncertain:** the letters are about 22 tall, which is more than font 497
  at its own size (16 by 20 cell). It may be that font at scale 1 with a
  taller face than its cell suggests, or a larger font. To be settled by
  drawing the same sentence and laying it beside S5.
- **Stage 3:** one sample line. **Stage 4:** a queue of match announcements.

**Tooltip**

- **Anchor:** beside the cursor, kept inside the window. Measured in S5: 685
  to 995, 340 to 455, so 310 by 115 for that item.
- **Contents:** a title in yellow with the price at the right; stat lines in
  green; a passive's name in orange and its text in white, wrapped.
- **Panel:** `panel.dark`. The design calls for parchment; the reference's
  shop tooltip is dark. See "Changes this plan asks for".
- **Needs:** wrapping a paragraph to a width. The drawing layer draws single
  lines; breaking a paragraph into lines is a small pure function beside it,
  tested on its own.
- **Stage 3:** S5's tooltip as sample. **Stage 4:** what is under the
  cursor.

### 5.11 Overlays, and what is put off

Two kinds, kept apart:

- **Overlays over a match** dim the match behind them with `dim` and leave
  the rest of the interface on screen. The shop and the death screen.
- **Screens before a match** have no match behind them and nothing to dim.
  Hero selection. None is in this stage.

Neither kind is part of the fixed layout, the clash test or the budget.

**Shop** (S5), its own acceptance step

- **Anchor:** centred. Measured: 568 to 1350, 250 to 830, so 782 by 580.
- **Contents:** a title bar ("Grand Exchange") with a close button; at the
  left a list of categories; a search line; in the middle the items of the
  chosen category as rows of pictures with prices, arrows between an item
  and what it builds into; at the right the chosen item's name, price,
  stats, what it builds into, and a buy button; along the bottom "Your
  items" with a hint and the gold count.
- **Our content:** our six items in one category. The reference's long list
  of categories is its item system, not ours; stage 3 shows the frame, one
  category, the grid, the detail panel and the foot line.
- **Stage 3:** that layout with labelled squares. **Stage 4:** the item
  list, gold; buy.

**Death screen** (S4), its own acceptance step

- **The match in grey:** in the reference the whole scene is drawn grey and
  dark. That is a change to how the scene is drawn, not an interface piece;
  in stage 3 it is a `dim` rectangle only, and the grey is noted as a later
  rendering task.
- **Contents, centred:** "Oh dear, you are dead!" in red, about 808 to 1108
  at 372 to 400; "Respawning in 3s" in yellow under it; a dark panel 795 to
  1125, 440 to 520 with who dealt the killing blow and a bar for each source
  of damage in the last seconds, its total at the right.

**Hero selection** (S1): put off

A whole screen for a lobby that does not exist, whose way of working has
not been decided. It is not built in stage 3. Its measurements are kept in
appendix A for when a lobby is designed.

**Scoreboard:** put off

There is no reference frame of one and no design. It is not in this stage
and has no file in the tree until it has one or the other.

## 6. Behaviour at each scale and window size

- **Scale** is one whole number for the whole interface, chosen by fit
  (section 2). Every size in section 5 is multiplied by it; every position
  is a whole pixel.
- **Anchors hold.** Corner components keep their margins from their corner;
  centred ones stay centred while they fit; the team frames and target panel
  keep their distance from the top; the objectives hang from 38% of the
  height. Extra window goes to the battlefield.
- **Overlays** are centred at their own size; the shop scrolls inside itself
  when the window is too small for it, and is never scaled down.

Every rectangle at 1920 by 1080, scale 1, from the anchors, beside what was
measured:

| Piece | From the anchors | Measured in the reference |
|---|---|---|
| Score strip | 693 to 1227, 10 to 48 | 692 to 1226, 10 to 48 |
| Kill feed (four entries) | 1622 to 1908, 18 to 136 | one entry 1622 to 1908, 18 to 46 |
| Team frames | 1728 to 1908, 168 to 356 | the same |
| Target panel | 8 to 340, 82 to 240 | the same |
| Objectives (ours, 290 to 370 tall) | 8 to 280, 410 to 700 or 780 | 8 to 280, 410 to 760 or 810 |
| Chat | 5 to 445, 930 to 1070 | the same |
| Ability bar | 731 to 1189, 968 to 1062 | 730 to 1188, 968 to 1062 |
| Inventory (ours, 144 by 160) | 1496 to 1640, 902 to 1062 | 1408 to 1640, 838 to 1062 |
| Minimap | 1646 to 1912, 800 to 1064 | the same |

The anchors reproduce the reference to a pixel wherever the content is the
same. That checks the anchors at the one size the reference was recorded
at, and nothing more: it does not show the layout is good at any other
shape of window. For that:

| Window | Shape | Scale | What gives | Usual covers | Objectives, at their longest | Chat from |
|---|---|---|---|---|---|---|
| 2560 by 1080 | 21:9 | 1 | nothing | 12.3% | 410 to 780 | 930 |
| 3440 by 1440 | 21:9 | 1 | nothing | 6.8% | 547 to 917 | 1290 |
| 5120 by 1440 | 32:9 | 1 | nothing | 4.6% | 547 to 917 | 1290 |
| 1920 by 1200 | 16:10 | 1 | nothing | 14.7% | 456 to 826 | 1050 |
| 1600 by 1200 | 4:3 | 1 | nothing | 17.7% | 456 to 826 | 1050 |
| 1280 by 1024 | 5:4 | 1 | chat to 376; objectives to title, for the budget | 19.7% | title only | 874 |
| 1440 by 2560 | 9:16 | 1 | nothing | 9.2% | 973 to 1343 | 2410 |
| 1080 by 1920 | 9:16 | 1 | chat to its last line; inventory hidden; kill feed under the score strip | 12.5% | 730 to 1100 | 1886 |
| 1024 by 768 | 4:3, under the smallest supported | 1 | as the last, and objectives to title | 23.7% | title only | 734 |

Nothing clashes at any of them. Two things these show that 16:9 sizes do
not: a window can be wide enough for a higher scale and not tall enough
(5120 by 1440 stays at scale 1, because the longest objectives list does
not fit at scale 2 in 1440 of height); and a narrow window needs a rule for
the top row, where the score strip and the kill feed meet (section 7).

These are arithmetic. Whether the objectives look right two fifths of the
way down a very tall window is for an eye at step B.

## 7. When the whole interface does not fit

Below the size at which everything fits (section 2), pieces give way.
Worked through with real rectangles, a single list applied from the top hid
things that were in nobody's way, so the order is **by what is wrong**, and
it is done in **two passes**.

### Pass 1: the layout, from the window alone

Decided for the usual state, so it depends on the window's size and nothing
else.

**If pieces clash along the bottom:**

1. The chat narrows, from 440 down to no less than 280.
2. The ability bar slides left from the centre (see below).
3. The chat shrinks to its last line, which may narrow further, to 200.
4. The inventory is hidden.

**If pieces clash down the left, or the objectives would leave the
window:** the objectives shrink to their title line.

**If the score strip and the kill feed meet along the top:** the kill feed
moves down, under the score strip.

**If pieces clash down the right:** the team frames shrink to portraits and
health bars; then, in a window too short even for that, they are hidden.

**Then, if the usual state covers more than a quarter**, in the design's
order, each only until the budget is met: the objectives to their title;
the team frames to portraits; the chat to its last line; the kill feed
hidden; the score strip hidden; the inventory hidden.

### Pass 2: what comes and goes with play

The layout from pass 1 is not moved. Each of these is then shown where it
fits and not where it does not:

- the objectives' longer list: if it would clash, the title only;
- the target panel: if it would clash, it is not shown;
- further kill-feed entries: as many as fit above the team frames.

The second draft took these in one pass, and at 1366 by 768 that hid the
target panel to make room for an objectives list the budget then shrank
anyway. In two passes the target panel shows at every supported size.

The ability bar and the minimap are never hidden.

### The ability bar sliding (approved, with conditions)

- It is **at the centre whenever it is clear there**: from 1364 wide with
  the chat at full width, and from 1321 wide once the chat has narrowed.
- It moves **only when it would otherwise be within 8 of what is on its
  right**, and then to exactly 8 from it: its left edge is the lesser of
  the centred place and that. It is never placed anywhere else.
- The chat's width and the bar's place are **worked out together**: the
  chat narrows until it is 8 clear of where the bar will be.
- It is a function of the window's width alone, so it is the same every
  time and goes back to the centre the moment there is room.

### What that gives at the two small sizes

| Window | What gives | Usual state then covers |
|---|---|---|
| 1366 by 768 | nothing clashes; for the budget, the objectives go to their title and the team frames to portraits | 23.3% |
| 1280 by 720 | the chat narrows to 376 and the bar slides left 21; for the budget, the objectives go to their title, the team frames to portraits and the chat to its last line | 20.8% |

The inventory and the target panel are on screen at both. Held strictly at
the centre, 1280 by 720 would lose the inventory: the centred bar ends at
869 and the inventory starts at 856.

### Deterministic and reversible

Pass 1 is decided from the window's size and nothing else; pass 2 from that
and which conditional pieces are showing. Neither looks at what the layout
was a frame ago. So the same window and the same pieces always give the
same layout, a growing window brings pieces back in exactly the reverse
order, and a window held at a threshold cannot flicker between two layouts.
The tests at step B assert all three.

## 8. Order of building

Each step is one component or one piece of groundwork, with its own tests
and its own acceptance, and each leaves the game running with the old
interface still drawn and still the default.

| Step | What | Depends on | How it is checked |
|---|---|---|---|
| A | The theme: the stage 3 roles, the frame kit composed into `panel.stone`, sprites chosen by eye beside the reference | stages 1 and 2 | a theme sheet in the game (below) |
| B | `HudState`, `sample.rs`, the scale by fit, the placing pass and collapse, with every component an empty labelled rectangle | A | tests on rectangles (below); the labelled rectangles over the game beside S2 |
| 1 | Score strip | B | beside S2 and S3 |
| 2 | Ability bar with health and resource | B | beside S2 and S3 |
| 3 | Minimap | B | beside S2 |
| 4 | Objectives | B | beside S2 for layout; each sample state shown |
| 5 | Team frames | B | beside S2 and S4 |
| 6 | Target panel | B | beside S4 |
| 7 | Unit plates | B, the client's projection | beside S2; hidden where the old bars are hidden |
| 8 | Inventory | B | beside S2 for cell style |
| 9 | Chat | B, paragraph wrapping | beside S2 |
| 10 | Kill feed | B | beside S3 and S5 |
| 11 | Announcements | B, outlined text | beside S5 |
| 12 | Tooltip | B, paragraph wrapping | beside S5 |
| 13 | Shop overlay | B, scrolling inside a panel | beside S5 |
| 14 | Death overlay | B, outlined text | beside S4 |
| 15 | The whole, together | all of the above | the final comparison (below) |

**The theme sheet (step A)** shows every role by name: the assembled stone
frame, the dark and parchment panels, buttons and slots, all four fonts,
the icons, and every colour. It also shows them **together**, because a
font that reads well alone can be cramped in a panel: a line of each font
inside each kind of panel; a button with its label; a slot with a count
over it; a bar with figures on it; yellow outlined text over light ground
and over dark. The stone frame is judged by eye beside the reference there;
no time is spent proving the blurred original is the same sprite.

**The rectangle tests (step B)** replace this plan's script with tests on
the real code:

- every rectangle at the seven 16:9 sizes of section 2 and the nine other
  shapes of section 6;
- every component at its largest: the objectives at 370, four allies, four
  kill-feed entries, the target panel open, the chat full;
- width and height varied independently over every window from 960 by 384
  to 3840 by 2400, with no clash left at any, the usual state within a
  quarter, and the scale never falling as either grows;
- the same layout whether a size is reached by growing or by shrinking;
- the ability bar at the centre at every width from 1321, and exactly 8
  from the inventory at every width below; the chat at its full 440 from
  1364 and 8 clear of the bar below that.

**Everything at once.** Components are also checked together in the states
that strain a layout: a long chat line that must wrap, the largest tooltip,
every objective line in a changed state, an ally dead, the target panel
open and four kill-feed entries, all in one screenshot, at 1920 by 1080 and
at 1366 by 768.

**The final comparison (step 15)** is the assembled interface at 1920 by
1080 beside S2, S3, S4 and S5: anchor by anchor, size by size, and what the
eye goes to first, second and third. Differences are listed, not waved
through. The old interface stays the default until the reviewer accepts
the new one.

**The drawing layer needs three small additions**, each pure and tested on
its own: breaking a paragraph into lines; text with an outline; a clipped
area that scrolls. None changes what stage 2 froze.

**Not in this stage, whatever else is approved:** reading the match,
buying or selling, sending chat, moving the camera, selecting a unit, or any
other command. Those are stage 4.

## 9. What is static first, and what becomes live

| Component | Stage 3 shows | Stage 4 reads | Our game has it today |
|---|---|---|---|
| Score strip | sample figures | clock, kills, own score, creep score | clock no; kills no |
| Ability bar | sample cooldowns | abilities, cooldowns, level, experience | yes |
| Health and resource | sample | health, resource | yes |
| Minimap | sample dots | visible units, camera | yes |
| Objectives | sample states of our own structures and camps | towers, inhibitors, nexus, camp timers | yes |
| Team frames | four sample allies | allies | no: one player |
| Target panel | a sample structure | the selected unit | no selection yet |
| Unit plates | sample positions | visible units | yes |
| Inventory | six squares, three filled | items, gold | yes: six slots |
| Chat | sample lines | chat and events | no |
| Kill feed | two samples | kill events | no |
| Announcements | one sample | announcements | no |
| Tooltip | S5's | what is under the cursor | no |
| Shop | S5's frame with our six items | items, gold | six items, one category |
| Death screen | sample | killer, damage, respawn | respawn yes; damage record no |

Most of the interface is ahead of the game. That is expected, and it is why
the design marks a placeholder on screen in development builds: the static
interface must not be mistaken for working systems.

## 10. Changes this plan asks for

Each is a change to the frozen `docs/INTERFACE.md` and needs review.

1. **The reference** becomes the recording and the five screenshots, with
   the note that the screenshots are the recording enlarged.
2. **The scale** is chosen by whether the whole interface fits, not by the
   bottom row and tallest column fitting.
3. **The size at which everything shows in full** is about 1554 by 874, not
   1280 by 720; 1280 by 720 stays supported, with collapse.
4. **The quarter** is defined: the area inside the rectangles of the fixed
   components showing in the usual state, each pixel once.
5. **The collapse order** is by what is wrong, with the chat narrowing and
   the kill feed added, and (if approved) the ability bar sliding.
6. **Health and resource bars** are part of the ability bar.
7. **The kill feed** is a component.
8. **The tooltip** is a dark panel, not parchment, as S5 shows it.
9. **The scoreboard and hero selection** leave stage 3.
10. **The native size table** in the design is filled from section 5 once
    each component is drawn.

## 11. Decisions made in review

| Question | Decision |
|---|---|
| May the ability bar slide left in a narrow window | Yes: only when it would otherwise clash, only as far as 8 clear, worked out together with the chat's width, the same for the same window (section 7) |
| Six slots as two rows of three | Yes; revisited with real item sizes if the inventory ever grows |
| The objectives at 38% of the height | Provisional. What is fixed is that they clash with nothing; where they sit is tried at seven shapes of window and looked at in the game at step B |

Nothing is left open in this plan. What it waits for is its figures being
checked and a separate go-ahead for stage 3.

## 12. What changed

### From the first draft to the second

| Review point | What was done |
|---|---|
| The 740 minimum width was wrong | Corrected: the bar and minimap need 1021 centred with 8 clear; with every step of section 7 taken the least window is 953 wide (section 2) |
| Inventory clashes with the bar at 1280 | Confirmed by rectangles; answered by sliding the bar, for the reviewer to decide (section 7) |
| Objectives and chat overlapped at 1600 by 900 | They did with a fixed top of 410; the top is now 38% of the height, and the rectangles are clear at every size tested (sections 5.4 and 6) |
| Layout from rectangles, not sums | Done; script and output included (appendix B) |
| Define the quarter | Done: unique area of the fixed components' rectangles in the usual state (section 2) |
| Does the quarter force collapse at 1600 by 900 | No: 23.5% (section 2) |
| A bigger screen must not show less | Scale is now chosen by fit; 1440p is scale 1, 1800p and 2160p are scale 2 (section 2) |
| Our own objectives | Done (section 5.4) |
| Do not imply eighteen inventory cells | Six slots drawn; the reference's panel kept only as a measurement (section 5.7) |
| Hero selection optional | Put off (section 5.11) |
| Scoreboard | Put off, and out of the file tree |
| Pre-match screens apart from overlays | Done (section 5.11) |
| Unit plates are placed by the world | Done, with visibility and depth (section 5.6) |
| Shop, death and hero select as separate steps | Shop and death are separate steps; hero select is out |
| The boundary of `crates/ui` | Stated (section 3) |
| The minimap is not made from the walkable map | Done: the old interface's picture is reused (section 5.3) |
| Theme sheet shows combinations | Done (section 8) |
| Collapse reversible and deterministic | Stated, with why it holds and how it is tested (section 7) |
| Strain several components at once | Added (section 8) |
| Sample values visibly marked | Added (section 3) |

### From the second draft to the third

| Review point | What was done |
|---|---|
| "The centred bar clears the inventory from 1364" was wrong | Corrected: 1321. 1364 is the chat's doing, and is the layout's least width (sections 2 and 5.7) |
| The 1481 for the reference's inventory | It had no clearance in it. With 8 clear it is 1497 (the review's 1498, less one for where a half pixel falls) |
| One rule for clearance everywhere | 8, in the prose, the script and the output (section 2) |
| The bar slides only as far as needed, with the chat | Written as conditions; the script works the two out together (section 7) |
| Objectives at 38% provisionally, tested at other shapes | Done at 16:10, 4:3, 5:4, 21:9, 32:9 and 9:16 (section 6) |
| The objectives' position must not be frozen as a universal rule | The rule is "clashes with nothing"; 38% is a placing within it (section 5.4) |
| Largest content sizes in the tests | The busy state now has the objectives at 370; it is what the scale and every clash test use (sections 2 and 5.4) |
| Width and height independently | A sweep of 5,824 windows; nothing clashes, and the scale never falls (section 2, appendix B) |
| The 1920 by 1080 match proves only the anchors | Said so (section 6) |
| Exclusions from the budget stay explicit | Unchanged (section 2) |

Running the other shapes found three things the 16:9 sizes had hidden, all
now in section 7: the score strip and kill feed meet in a narrow window; a
window can be too short for the team frames above the minimap; and taking
the conditional pieces in the same pass as the layout hid the target panel
needlessly at 1366 by 768.

---

## Appendix A: measurements, screenshot by screenshot

All on the 1920 by 1080 grid, left to right then top to bottom, good to
about 4 pixels. Each screenshot has a dark border about 8 wide that is part
of the recording, not of the game.

### S2 early (0:15)

| What | Left to right | Top to bottom | Size | Notes |
|---|---|---|---|---|
| Score strip | 692 to 1226 | 10 to 48 | 534 by 38 | centre 959 |
| in it: team marks and swords | 745 to 805 | 18 to 40 | | blue, swords, red |
| in it: clock | 895 to 930 | 18 to 40 | | yellow, "0:15" |
| in it: own score | 1000 to 1075 | 18 to 40 | | "0/0/0" |
| in it: creep score | 1145 to 1185 | 18 to 40 | | "0 CS" |
| Team frames | 1728 to 1908 | 168 to 356 | 180 by 188 | four rows, pitch 47 |
| in a row: portrait | 1735 to 1770 | | about 36 | level box at 1762 to 1772 |
| in a row: bars | 1780 to 1880 | | 100 by 6 each | health over resource |
| in a row: spells | 1888 to 1905 | | about 16 | two, one over the other |
| Objectives | 8 to 280 | 410 to 760 | 272 by 350 | |
| in it: title | 20 to 100 | 415 to 435 | | minimise button 258 to 275 |
| in it: team names | 60 to 150, 170 to 250 | 445 to 465 | | blue, red |
| in it: lane rows | | 505, 528, 553 | pitch 24 | three pips and a dot each side |
| in it: rule | 20 to 275 | 573 | | |
| in it: list rows | icons 20 to 35, timers ending 275 | 592, 617, 641, 665, 690, 715, 740 | pitch 24 | the second is a sub-line |
| Hero plate | 915 to 1003 | 395 to 425 | 88 by 30 | name, level box, bar about 80 by 8 |
| Minion bars | | | about 28 by 3 | above each minion |
| Chat parchment | 5 to 445 | 930 to 1045 | 440 by 115 | scrollbar near 432 |
| Chat tabs | 20 to 185 | 1047 to 1070 | | "All", "Game", "Public" |
| Ability bar | 730 to 1188 | 968 to 1062 | 458 by 94 | centre 959 |
| in it: portrait | 740 to 815 | 980 to 1055 | 75 across | ring, level box lower right |
| in it: four abilities | 822 to 990 | 978 to 1018 | about 40 each | strip under each to 1024 |
| in it: two spells | 1010 to 1096 | 978 to 1018 | about 40 each | |
| in it: four small slots | 1104 to 1180 | 975 to 1055 | about 36 each | two by two |
| in it: health bar | 824 to 1094 | 1028 to 1044 | 270 by 16 | figures centred |
| in it: resource bar | 824 to 1094 | 1048 to 1064 | 270 by 16 | figures centred |
| Inventory | 1408 to 1640 | 838 to 1062 | 232 by 224 | |
| in it: equipment rows | 1412 to 1636 | 842 to 882, 884 to 924 | five cells each | pitch about 44 |
| in it: item row | 1412 to 1636 | 940 to 985 | five cells | |
| in it: item row and gold | 1412 to 1540, gold 1560 to 1636 | 990 to 1030 | three cells | gold pile, then count |
| in it: buttons | 1415 to 1520, 1528 to 1636 | 1036 to 1058 | | "Shop", "Teleport" |
| Minimap | 1646 to 1912 | 800 to 1064 | 266 by 264 | button near 1900, 808 |

### S3 fight (1:18)

| What | Left to right | Top to bottom | Size | Notes |
|---|---|---|---|---|
| Kill feed entry | 1622 to 1908 | 18 to 46 | 286 by 28 | team bar, portrait, name, swords, portrait, name, "+300" |
| Score strip kills | 748 and 805 | 18 to 40 | | "1" beside each team mark |
| Ability cooldown figures | over slots at 1030 and 1075 | 990 to 1010 | | seconds, white |
| Health bar damage | 880 to 1094 | 1028 to 1044 | | red stretch after the green |
| Gold popup | 760 to 800 | 940 to 955 | | "+30" beside a small icon |
| Camp timer in the world | 130 to 160 | 250 to 300 | | an icon over "1:11"; not interface |
| Tower range ring | | | | drawn in the world; not interface |

### S4 death (7:27)

| What | Left to right | Top to bottom | Size | Notes |
|---|---|---|---|---|
| Target panel | 8 to 340 | 82 to 240 | 332 by 158 | |
| in it: portrait box | 20 to 78 | 90 to 150 | about 58 | |
| in it: name and tag | 92 to 210, 300 to 330 | 88 to 105 | | blue name, yellow tag |
| in it: health bar | 92 to 330 | 112 to 126 | 238 by 14 | figures centred |
| in it: description | 92 to 310 | 132 to 146 | | orange, one line |
| in it: figures | labels 92 and 212, values ending 200 and 328 | 162, 182, 202, 222 | pitch 20 | two columns of four |
| "Oh dear, you are dead!" | 808 to 1108 | 372 to 400 | | red |
| "Respawning in 3s" | 905 to 1012 | 410 to 428 | | yellow |
| Death panel | 795 to 1125 | 440 to 520 | 330 by 80 | |
| in it: damage bar | 805 to 1115 | 487 to 508 | | red, name left, total right |
| Objectives | 8 to 280 | 410 to about 810 | | two sub-lines more than S2 |
| Whole screen | | | | grey and dark |

### S5 shop (11:27)

| What | Left to right | Top to bottom | Size | Notes |
|---|---|---|---|---|
| Shop window | 568 to 1350 | 250 to 830 | 782 by 580 | centre 959, 540 |
| in it: title bar | 568 to 1350 | 252 to 284 | | "Grand Exchange", close at 1320 to 1345 |
| in it: categories | 580 to 710 | 290 to 710 | | about seventeen lines, pitch 22 |
| in it: search line | 725 to 1090 | 290 to 312 | | |
| in it: item rows | 725 to 1095 | 315 to 710 | | pictures about 36, prices under |
| in it: detail panel | 1105 to 1340 | 290 to 715 | | name, price, stat, "Builds into", list, button |
| in it: your items | 580 to 1340 | 722 to 795 | | two pictures with prices |
| in it: foot line | 580 to 1340 | 798 to 822 | | hint left, gold right |
| Tooltip | 685 to 995 | 340 to 455 | 310 by 115 | title, price right, two stats, passive |
| Announcement | 630 to 1285 | 228 to 255 | | yellow, partly under the shop |
| Kill feed entry | 1622 to 1908 | 18 to 46 | | a structure: team, swords, name |
| Objectives | 8 to 280 | 410 to about 785 | | |

### S1 select

| What | Left to right | Top to bottom | Size | Notes |
|---|---|---|---|---|
| Top bar | 30 to 1890 | 22 to 100 | 1860 by 78 | mode left, title and countdown centre, sound and Leave right |
| Left team column | 30 to 395 | 118 to 1055 | 365 by 937 | name over five cards, pitch 171 |
| Right team column | 1522 to 1890 | 118 to 1055 | 368 by 937 | |
| in a card | portrait 48 to 125, text 135 to 300, spells 340 to 375 | | | name, rank and rating, pick, lane |
| Lane row | 415 to 1502 | 118 to 205 | 1087 by 87 | five buttons about 185 wide; chosen is red |
| Hero grid | 415 to 1502 | 222 to 522 | 1087 by 300 | twelve across, pitch about 89 by 100; 31 heroes |
| Detail panel | 420 to 1500 | 545 to 890 | 1080 by 345 | parchment; portrait left, text right, scrollbar |
| Spells row | 415 to 1500 | 910 to 1055 | 1085 by 145 | two chosen, seven choices, rating, Lock in at 1208 to 1482 |

## Appendix B: the layout arithmetic

Output of `docs/interface_stage3_layout.py`, which places every rectangle by
its anchor and margins. "Usual" is every fixed piece but the target panel,
with one kill-feed entry and the objectives at 290. "Busy" adds the target
panel, three more entries, and the objectives at their longest, 370. Two
pieces clash if they are within 8 of each other. Sizes are the interface's
own pixels times the scale.

**The least window at scale 1 for the whole interface, bar centred:** 1364
wide and 776 high, by clashes alone.

**Clearances, bar centred, 8 clear:** the minimap alone from 1021 wide; our
inventory from 1321; the chat at 440 from 1364; the reference's inventory
from 1497.

**Where each scale first fits (16:9 windows):**

| Budget | Scale 1 from | Scale 2 from | Scale 3 from |
|---|---|---|---|
| a quarter | 1554 by 874 | 3106 by 1747 | 4658 by 2620 |
| 30% | 1418 by 797 | 2836 by 1595 | 4254 by 2392 |
| none | 1380 by 776 | 2760 by 1552 | 4140 by 2328 |

**At a quarter, by window, bar sliding.** "Sum" adds the rectangles' areas
and "unique" counts each pixel once; they came out equal at every size,
because nothing overlaps once collapse has run.

| Window | Scale | What gives (pass 1) | Usual covers | Busy covers |
|---|---|---|---|---|
| 1280 by 720 | 1 | chat to 376; objectives to title; team frames to portraits; chat to last line | 20.8% | 29.3% |
| 1366 by 768 | 1 | objectives to title; team frames to portraits | 23.3% | 30.7% |
| 1600 by 900 | 1 | nothing | 23.5% | 30.5% |
| 1920 by 1080 | 1 | nothing | 16.3% | 21.2% |
| 2560 by 1440 | 1 | nothing | 9.2% | 11.9% |
| 3200 by 1800 | 2 | nothing | 23.5% | 30.5% |
| 3840 by 2160 | 2 | nothing | 16.3% | 21.2% |

With the bar held at the centre instead, 1280 by 720 also hides the
inventory and its usual state covers 23.3%; every other row is the same.

The busy state goes over a quarter at the smaller sizes. That is by the
definition in section 2: the budget is kept by the usual state, and the
target panel and extra feed entries are allowed on top of it for as long
as they are showing.

**Other shapes:** the table in section 6.

**The sweep:** 5,824 windows from 960 by 384 to 3840 by 2400 in steps of
32, width and height independently, both states. None was left with a
clash, and the scale never fell as a window grew. Nothing clashes down to
953 wide and 280 high.

**For comparison, the same at 30%:** 1366 by 768 gives up only the
objectives' list; 1280 by 720 gives up the chat's width and the objectives'
list. The plan keeps a quarter.

**With the reference's own sizes** (inventory 232 by 224): the centred bar
clears the inventory only from 1497 wide, and the inventory, not the chat,
sets the layout's least width. That is what applies if the item system
later takes the reference's eighteen cells.
