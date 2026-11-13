# Interface stage 3: plan for the static interface

Status: **a plan for review. Nothing in it is built, and nothing in it is
authorised to be built.** It was asked for after stages 1 and 2 of
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

## 2. The one finding that changes a frozen figure

In the reference, at 1920 by 1080, the interface is drawn at the artwork's
own size: an inventory cell is about 40 pixels, which is a 36 by 32 item
sprite and its frame; text is about 12 to 16 pixels tall, which is the
regular font at its own size. **The reference is scale 1 at 1080p.**

That means the measured sizes below *are* the interface's own sizes, and
they can be added up:

| Row or column | Pieces | Sum at scale 1 |
|---|---|---|
| Bottom row, across | chat 440, ability bar 458, inventory 232, minimap 266, plus the margins and gaps seen | about 1,430 |
| Left column, down | target panel 158, objectives 350, chat 140, plus the 82 above the first | about 760 |
| Right column, down | kill feed 28, team frames 188, minimap 264, plus gaps | about 520 |

The design says: "The smallest supported window is 1280 by 720. At that
size, at scale 1, every component is shown in full. This is a target until
stage 1 gives the real sizes." **It does not hold.** At 1280 by 720 the
bottom row is about 150 too wide and the left column about 40 too tall, and
the interface would cover about 46% of the window against a budget of a
quarter. At 1920 by 1080 it covers about 21%, which is the "about a fifth"
the design measured.

What this plan proposes, for review:

- **The whole interface is shown in full from 1600 by 900.** That is the
  smallest common window the sums fit in.
- **1280 by 720 stays the smallest supported window**, and at that size the
  collapse order applies: it is the normal state there, not a failure.
- **The scale steps at whole multiples of 1280 by 720**: scale 2 from 2560
  by 1440, scale 3 from 3840 by 2160. At 1920 by 1080 the interface is scale
  1 and the same size as the reference.

The consequence the reviewer should weigh: on a 1920 by 1080 screen the
interface is small, as it is in the reference. Stepping to scale 2 earlier
(say from 1920 by 1080) would double every piece and cover over 80% of the
window, so it is not an option; the alternative is a player setting that
forces scale 2 with the collapse order in force.

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
    screens/
      shop.rs
      scoreboard.rs
      hero_select.rs
      death.rs
```

All of it is in `crates/ui`, which depends on nothing, so every component is
tested without the game, the cache or a graphics card, the way stage 2 was.
The client's part is two things: filling `HudState` (stage 4) and handing
the finished quads to the pass that already exists.

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

1. Choose the scale from the window (section 2).
2. Ask every component its size.
3. Place each at its anchor (section 5), in whole pixels.
4. If the pieces overlap, or cover more than a quarter of the window, apply
   the collapse order one step at a time until they do not (section 7).
5. Draw them, lowest layer first.

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

### Layers

| Layer | What is on it |
|---|---|
| 0 | unit plates (they belong to the world, under everything else) |
| 1 | the fixed components |
| 2 | the kill feed and announcements |
| 3 | full-screen interfaces and the dimming behind them |
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
- **Stage 3:** a flat picture of the real map made once from the walkable
  map, with sample dots. **Stage 4:** units the viewer may see, the camera
  box; clicks to move the camera and to order a move.
- **Depends on:** a picture of the map. The client already draws a minimap
  from the lanes; stage 3 may reuse that picture as it is.

### 5.4 Objectives

- **Anchor:** left edge, top fixed. Measured in S2: 8 to 280, 410 to 760, so
  272 wide; 8 from the left, top at 410.
- **Height grows with what it lists:** 350 in S2, about 375 in S5, about 400
  in S4.
- **Contents:** title "Objectives" in orange with a minimise button at its
  right; a small table with a column for each team (name in its colour over
  a quiet green word that I read as "Keeps") and a row for each lane (Top,
  Mid, Bot), each cell three tower pips and a green dot; a thin rule; then a
  line for each objective with an icon, its name, and at the right its timer
  or state. Some lines have a quieter second line under them ("Slain" with a
  count for each team under the dragon; a line of where it is under others).
- **Lines seen:** Black dragon (later Green, later Blue: it changes as the
  match goes on), TzKal-Zuk, Giant Mole, River chickens, Shooting star,
  Wards.
- **Panel:** `panel.dark`. Row pitch about 24.
- **Rank:** dark and quiet until a line changes; a live objective's state is
  green ("Alive").
- **Stage 3:** the S2 contents as sample. **Stage 4:** structure counts,
  objective timers and states. Most of those objectives do not exist in our
  game; a line whose system is not built is marked in development builds, as
  the design requires.

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

- **Anchor:** above each unit, in the world. Measured over the player's
  hero in S2: 915 to 1003, 395 to 425: the name in green, then a level box
  and a health bar about 80 by 8.
- **Kinds:** heroes get name, level and bar; minions a thin bar about 28 by
  3, blue for allies and red for enemies; structures a wider bar.
- **Stage 3:** drawn at sample screen positions. **Stage 4:** each visible
  unit's name, level, health and screen position. The client's old interface
  already places bars over units; this replaces that drawing only.

### 5.7 Inventory

- **Anchor:** bottom right, left of the minimap. Measured in S2: 1408 to
  1640, 838 to 1062, so 232 by 224; its right edge 6 from the minimap's
  left, 18 from the bottom.
- **Contents, top to bottom,** as the design already records from five
  frames, now with positions: two rows of five equipment cells (838 to 924),
  each with a faint outline; a gap; a row of five item cells (940 to 985); a
  row of three item cells and then the gold pile and gold count (990 to
  1030); two buttons, Shop and Teleport (1036 to 1058).
- **Cell pitch:** about 44 across and 42 down; a cell is about 40.
- **Panel:** `panel.stone` with `panel.slot` cells.
- **From parts:** five cells of 40 with 4 between and 8 of frame each side
  is 232 wide. Matches.
- **Rank:** quiet, but the gold count is yellow and is read often.
- **Stage 3:** S2's contents as labelled squares. **Stage 4:** items, gold;
  buy, sell, use, open shop.

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

### 5.11 Full-screen interfaces

Each dims the match behind it with `dim` and is one component.

**Shop** (S5)

- **Anchor:** centred. Measured: 568 to 1350, 250 to 830, so 782 by 580.
- **Contents:** a title bar ("Grand Exchange") with a close button; at the
  left a list of categories (Recommended, item sets, All items, then kinds
  by slot, then by role); a search line; in the middle the items of the
  chosen category as rows of pictures with prices, arrows between an item
  and what it builds into; at the right the chosen item's name, price,
  stats, what it builds into, and a buy button; along the bottom "Your
  items" with a hint and the gold count.
- **Stage 3:** S5's layout with labelled squares. **Stage 4:** the item
  list, gold; buy and sell.

**Death screen** (S4)

- **Whole screen:** the match drawn in grey and darkened. That is a change
  to how the scene is drawn, not an interface piece; in stage 3 it is a
  `dim` rectangle only, and the grey is noted as a later rendering task.
- **Contents, centred:** "Oh dear, you are dead!" in red, about 808 to 1108
  at 372 to 400; "Respawning in 3s" in yellow under it; a dark panel 795 to
  1125, 440 to 520 with who dealt the killing blow and a bar for each source
  of damage in the last seconds, its total at the right.
- The rest of the interface stays on screen, dimmed.

**Hero select** (S1)

- **Whole screen,** no match behind it.
- **Top bar:** 30 to 1890, 22 to 100. Game mode at the left; "Ranked: choose
  your hero" and a countdown at the centre; a sound button and Leave at the
  right.
- **Team columns:** left 30 to 395 and right 1522 to 1890, from 118 to 1055.
  A team name over five player cards, pitch 171: portrait, name, rank and
  rating, the hero picked or "Picking...", lane, two battle spells.
- **Centre, top to bottom:** a lane row (415 to 1502, 118 to 205) of five
  buttons, the chosen one red; the hero grid (415 to 1502, 222 to 522),
  twelve across, three rows, pitch about 89 by 100, each a portrait with a
  name under it, a green dot on some, "Picked" over taken ones; a parchment
  panel (420 to 1500, 545 to 890) with the hero's portrait, name, title,
  tags, a "Popular build" line, a description, and its passive and abilities
  in a scrolling list; a battle-spells row (415 to 1500, 910 to 1055) with
  the two chosen, the choices, rating at stake, and Lock in.
- **Stage 3:** S1's layout with our three champions and labelled squares.

**Scoreboard**

- No reference frame. Not planned in detail; it waits for one.

## 6. Behaviour at each scale and window size

- **Scale** is one whole number for the whole interface (section 2). Every
  size in section 5 is multiplied by it; every position is a whole pixel.
- **Anchors hold.** Corner components keep their margins from their corner;
  centred ones stay centred; the objectives and team frames keep their
  distance from the top. Extra window goes to the battlefield.
- **Left-edge and right-edge tops are fixed from the top**, as measured (82,
  168, 410), until the window is too short for the column, when section 7
  applies.
- **Full-screen interfaces** are centred at their own size; the shop and the
  hero grid scroll inside themselves when the window is too small for them,
  and are never scaled down.

| Window | Scale | State |
|---|---|---|
| 1280 by 720 | 1 | collapse steps 1 to 4 below in force |
| 1600 by 900 | 1 | everything in full |
| 1920 by 1080 | 1 | everything in full; the reference's size |
| 2560 by 1440 | 2 | everything in full, as at 1280 by 720 doubled: collapse steps in force |
| 3840 by 2160 | 3 | as 1280 by 720 tripled: collapse steps in force |

The last two rows are a weakness of stepping at multiples of 1280 by 720:
a 1440p screen gets the tight layout doubled. Stepping at multiples of 1600
by 900 instead gives scale 2 from 3200 by 1800 and leaves 1440p at scale 1,
small. Which is better is a judgment for someone looking at a 1440p screen,
and neither the user's machine nor mine has one. It is one constant, and the
plan is to make it a setting.

## 7. The collapse order

Applied one step at a time, only while the pieces overlap or cover more than
a quarter of the window; undone in reverse as soon as there is room. This is
the design's order, with the steps the reference's real sizes add in front.

| Step | What gives | To what |
|---|---|---|
| 0 | Chat narrows | from 440 to no less than 300 |
| 1 | Target panel | hidden |
| 2 | Objectives | its title line only |
| 3 | Team frames | portraits and health bars only |
| 4 | Chat | its last line |
| 5 | Kill feed | hidden |
| 6 | Score strip | hidden |
| 7 | Inventory | hidden |

The ability bar and the minimap are never hidden. Below the size at which
those two fit side by side at scale 1, about 740 wide and 290 high, the
window refuses to shrink.

Step 0 and step 5 are new; the rest is the design's order unchanged.

## 8. Order of building

Each step is one component or one piece of groundwork, with its own tests,
and each leaves the game running with the old interface still drawn. The
component order is the reviewer's.

| Step | What | Depends on | How it is checked |
|---|---|---|---|
| A | The theme: the stage 3 roles, the frame kit composed into `panel.stone`, sprites chosen by eye beside the reference | stages 1 and 2 | a theme sheet: every role drawn and named on one screen, in the game |
| B | `HudState`, `sample.rs`, the placing pass and collapse order, with every component an empty labelled rectangle | A | rectangles asserted at twenty window sizes; the labelled rectangles over the game beside S2 |
| 1 | Score strip | B | beside S2 and S3 |
| 2 | Ability bar with health and resource | B | beside S2 and S3 |
| 3 | Minimap | B, a map picture | beside S2 |
| 4 | Objectives | B | beside S2, S4 and S5 for its three heights |
| 5 | Team frames | B | beside S2 and S4 |
| 6 | Target panel and unit plates | B | beside S4 and S2 |
| 7 | Inventory | B | beside S2 |
| 8 | Chat and kill feed | B, paragraph wrapping | beside S2 and S3 |
| 9 | Announcements and tooltip | B, paragraph wrapping, outlined text | beside S5 |
| 10 | Shop, death screen, hero select | B, scrolling inside a panel | beside S5, S4 and S1 |

Step B is the one that matters most: when it passes, the layout, anchors,
scale and collapse are settled with nothing drawn but boxes, and every later
step is filling one box.

**Each step's check** is the design's: a screenshot at 1920 by 1080 beside
the reference frame. Anchor matches; size within what the measurements
allow; the eye goes to the same things in the same order. Differences are
listed. The old interface is hidden behind a switch for these screenshots
and stays the default until the reviewer says otherwise.

**The drawing layer needs three small additions**, each pure and tested on
its own: breaking a paragraph into lines; text with an outline; a clipped
area that scrolls. None changes what stage 2 froze.

## 9. What is static first, and what becomes live

| Component | Stage 3 shows | Stage 4 reads | Our game has it today |
|---|---|---|---|
| Score strip | sample figures | clock, kills, own score, creep score | clock no; kills no |
| Ability bar | sample cooldowns | abilities, cooldowns, level, experience | yes |
| Health and resource | sample | health, resource | yes |
| Minimap | sample dots | visible units, camera | yes |
| Objectives | S2's list | towers by lane, objective timers | towers yes; objectives no |
| Team frames | four sample allies | allies | no: one player |
| Target panel | a sample structure | the selected unit | no selection yet |
| Unit plates | sample positions | visible units | yes |
| Inventory | S2's contents | items, gold | six slots, not eighteen |
| Chat | sample lines | chat and events | no |
| Kill feed | two samples | kill events | no |
| Announcements | one sample | announcements | no |
| Tooltip | S5's | what is under the cursor | no |
| Shop | S5's layout | items, gold | six items, no categories |
| Death screen | sample | killer, damage, respawn | respawn yes; damage record no |
| Hero select | three champions | the lobby | no lobby |

Most of the interface is ahead of the game. That is expected, and it is why
the design marks a placeholder on screen in development builds: the static
interface must not be mistaken for working systems.

## 10. Changes this plan asks for

Each is a change to the frozen `docs/INTERFACE.md` and needs review.

1. **The reference** becomes the recording and the five screenshots, with
   the note that the screenshots are the recording enlarged.
2. **The smallest window at which everything is shown in full** is 1600 by
   900, not 1280 by 720; 1280 by 720 stays supported, with collapse.
3. **Health and resource bars** are part of the ability bar, not a
   component of their own.
4. **The kill feed** is added as a component, and to the collapse order.
5. **The chat narrowing** is added in front of the collapse order.
6. **The tooltip** is a dark panel, not parchment, as S5 shows it.
7. **The native size table** in the design is filled from section 5 once
   each component is drawn.

## 11. Questions for the reviewer

1. **Scale steps:** multiples of 1280 by 720, or of 1600 by 900 (section 6)?
2. **Our own objectives.** The reference lists a dragon, TzKal-Zuk, a mole,
   chickens, a star and wards. We have towers, inhibitors, a nexus and four
   camps. Does stage 3 show the reference's list as sample text, or only
   what our game has? The plan assumes the reference's list, marked as
   sample.
3. **Eighteen inventory cells.** The reference has ten equipment cells and
   eight item cells; we have six slots. Stage 3 can draw eighteen; what they
   mean is an item-system decision, not a layout one.
4. **The stone frame.** The reference's stone is close to the cache's but I
   cannot tell from the blur whether it is the same sprites. Step A chooses
   by eye and shows the result; the reviewer should expect to say "closer"
   or "that will do" there.
5. **Hero select and the lobby.** S1 is a whole screen for a system we do
   not have. It is last in the order; it could be left out of stage 3
   altogether.

---

## Appendix: measurements, screenshot by screenshot

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
