# Interface

Status: **design approved with clarifications, and frozen. Stages 1 and 2
have passed and are frozen with it. Stage 3's step A theme, step B placing
pass, step 1 score strip, step 2 ability bar, step 3 minimap, step 4
objectives, step 5 team frames, step 6 target panel, step 7 unit plates,
step 8 inventory, step 9 chat, step 10 kill feed, step 11 announcements,
step 12 tooltip and step 13 shop are built and tested headlessly; the other
components are not built, and `hud.rs` is still the default.** The stage 3
plan was frozen separately in
`docs/INTERFACE_STAGE3_PLAN.md`; each later stage and component still needs
its own go-ahead after the stage before it has passed its check. A change to
this document is a design change and is reviewed like one.

It was written before any code because the work replaces most of
`crates/client/src/hud.rs` and adds a drawing layer the client does not have.

The interface copies the *composition and style* of the reference game: where
things sit, how big they are against the screen, what is loud and what is
quiet. The code is ours. The panels, fonts and icons come from the game cache
through data files, so every one of them can be replaced without touching the
code that lays them out.

## The reference

The reference is the full-match recording of the RuneScape-asset MOBA
(`videoplayback (3).mp4`, 17 minutes 28 seconds, 640 by 360). It is the only
source for layout decisions. Where this document and the recording disagree,
the recording is right and this document is to be corrected.

### What was measured, and how well

One frame, at 6:20 of match time, was measured by hand. The recording is 640
by 360, so no figure below is better than about one percent of the screen, and
small things (icon sizes, text heights, gaps) cannot be measured from it at
all. **Nothing here supports a claim of pixel accuracy.** The figures fix
composition and proportion only.

Positions are fractions of the screen, from the top left.

| Component | Anchor | Left to right | Top to bottom | Size (w by h) |
|---|---|---|---|---|
| Score strip | top centre | 0.36 to 0.64 | 0.01 to 0.05 | 0.28 by 0.04 |
| Target panel | top left | 0.01 to 0.18 | 0.08 to 0.22 | 0.17 by 0.14 |
| Objectives | left edge | 0.01 to 0.15 | 0.38 to 0.72 | 0.14 by 0.34 |
| Chat and event log | bottom left | 0.00 to 0.23 | 0.86 to 0.99 | 0.23 by 0.13 |
| Ability bar | bottom centre | 0.38 to 0.62 | 0.90 to 0.99 | 0.24 by 0.09 |
| Inventory | bottom right, left of the minimap | 0.73 to 0.85 | 0.78 to 0.99 | 0.12 by 0.21 |
| Minimap | bottom right corner | 0.86 to 1.00 | 0.74 to 0.99 | 0.14 by 0.25 |
| Team frames | right edge | 0.90 to 0.99 | 0.16 to 0.33 | 0.09 by 0.17 |
| Announcements | centre, upper third | centred | about 0.22 | text only |
| Tooltip | beside the cursor | n/a | n/a | sized to its text |
| Unit plates | above each unit | n/a | n/a | name, level, health |

### What must match the reference

- **Composition.** Each component sits at the edge or corner the table gives.
- **Proportion.** Each component takes about the share of the screen the table
  gives, within what whole-number scaling allows (see Scaling).
- **Hierarchy.** The battlefield is the loudest thing on screen. The ability
  bar and minimap are the strongest pieces of interface; objectives, team
  frames and chat are dark and quiet until something in them changes.
  Announcements are the brightest text on screen and are short-lived.
- **Style.** Stone and parchment panels, the RuneScape fonts, yellow
  announcements with a dark outline, a parchment tooltip.

### The inventory panel, from five more frames

The panel was cropped from five moments of the match (2:00, 5:00, 8:40, 11:40
and 15:00 of the recording) and is the same in all five. It is **not** six
slots. Top to bottom:

| Row | Slots | What is seen |
|---|---|---|
| 1 | 5 | equipment slots, each with a faint outline of what belongs there; the first holds a staff from the start |
| 2 | 5 | equipment slots; one fills with boots, another with a book, as the match goes on |
| 3 | 5 | item slots; four are filled from the start (two dark items, a blue orb, a dark pouch) |
| 4 | 3, then gold | three item slots, then a gold-pile icon and the gold count |
| 5 | 2 buttons | Shop and Teleport; Teleport turns red at one point |

So: ten equipment slots in two rows of five, eight item slots (five and
three), a gold count and two buttons. What each equipment outline stands for,
what the four starting items are, and what a red Teleport button means cannot
be read at this size; they are open questions for the item system, not for the
layout.

### More of the reference to measure before stage 3 is accepted

One frame fixes a starting layout, not a specification. Before the static HUD
is accepted it is compared against frames chosen to catch each component
changing:

- an announcement on screen ("Giant Mole has appeared!", "An enemy has been
  slain")
- a tooltip open, and a second tooltip of a different length
- the objectives panel early (few entries) and late (every timer running)
- the team frames with an ally dead, and with all five alive
- the target panel open and closed
- the shop open, the scoreboard open, the death screen, hero select
- the chat box with one line and with a full box

### What is not known yet

- Every size smaller than about six pixels in the recording.
- Which cache sprites the reference uses for each panel. This is found by
  looking at the sprite dump (stage 1), not by guessing ids.
- How the reference behaves at other window sizes. Only one size was recorded.

## Rules

### 1. The interface shows the match; it does not run it, and cannot overrule it

A component reads a snapshot of match state and draws it. It never holds a
timer, a cooldown, a gold count or a spawn rule of its own. The objectives
panel shows the time until a boss appears because the match says so; it cannot
make one appear.

Concretely there is one plain struct, `HudState`, filled once a frame from the
match, and each component takes only the part it needs. What the player does to
the interface (a click on an item slot, a key for an ability) leaves as a
*command* to the match. No component changes match state directly.

There is one command path, not two. A click on the shop's Buy button and a
buy order arriving over the network are the same command, enter the match at
the same place, and are checked by the same code: is this player alive, in
range of a shop, able to pay, holding a free slot. The interface may grey out
a button it expects to be refused, as a courtesy, but that is only a guess at
what the match will say. The match's answer is the only one that counts, and a
command the interface should never have sent is still refused. Nothing a
component draws or allows is ever taken as proof that an action is legal.

This is the same line `docs/SPECTATING.md` draws: a spectator's interface is
the same components fed a snapshot from someone else's match.

### 2. Components are separate

Each component is its own file with three things and nothing else: the slice of
`HudState` it reads, its layout in its own pixel grid, and its drawing. A
component does not know where on screen it is, what scale it is drawn at, or
which other components exist. Replacing one does not touch another.

| Component | Reads | Sends |
|---|---|---|
| Ability bar | abilities and cooldowns, battle spells, portrait, level | cast, level up |
| Health and resource bars | health, shield, resource, regeneration | none |
| Inventory | items, gold | buy, sell, use, open shop |
| Minimap | map picture, units the viewer may see, camera box | move camera, ping, move order |
| Chat and event log | chat lines, match events | chat message |
| Objectives | structure counts, objective timers and states | none |
| Team frames | each ally's portrait, health, resource, spells, respawn | none (later: select ally) |
| Score strip | match clock, team kills, own score, creep score | none |
| Announcements | a queue of match announcements | none |
| Target panel | the selected unit's name, health and figures | none |
| Tooltip | what is under the cursor | none |
| Unit plates | each visible unit's name, level, health, screen position | none |
| Full-screen interfaces | hero select, shop, scoreboard, death screen, tier list | their own commands |

Full-screen interfaces are components like any other; they take the whole
screen and dim the match behind them.

### 3. The renderer does not know about RuneScape

The drawing layer offers four things: a sprite, a nine-slice panel (corners
kept, edges and middle repeated), a line of text in a bitmap font, and a solid
or see-through rectangle. All of them clip to a rectangle and draw in layer
order.

Which sprite is "the panel background", which font is "body text", what colour
"announcement" is: that is a **theme**, a data file mapping roles to assets.
Components ask for roles, never for cache ids. Replacing every panel, font and
icon with original art is then a new theme file and a new set of images, and no
change to any component.

The cache decoders (sprites, fonts) live in `crates/cache`. The drawing layer
takes plain images and glyph tables and does not depend on the cache at all.

## Scaling

RuneScape's interface is pixel art and looks wrong the moment it is stretched.

- **Native pixels are kept.** A sprite is drawn at a whole multiple of its own
  size: 1, 2, 3 or 4 times. Never 1.5.
- **Never stretched unevenly.** A sprite's width and height always share one
  scale. Panels grow by nine-slice repetition, not by stretching.
- **One scale for the whole interface**, chosen from the window: the largest
  whole number at which the bottom row (chat, ability bar, inventory, minimap)
  fits side by side across the window's width and the tallest column fits its
  height. The player may override it in settings.
- **Sampling is nearest-pixel**, so edges stay hard.
- **Extra space goes to the battlefield.** Components stay pinned to their
  edges and corners. Centred components (score strip, ability bar,
  announcements) stay centred. Nothing is redesigned per monitor; a larger
  window shows more battlefield or steps the scale up.
- **Positions are whole pixels** after scaling, so nothing shimmers.

### How small the interface may be, and how much of the screen it may take

In the measured frame the interface covers about a fifth of the screen and
leaves the rest to the battlefield. That is the budget:

- **The interface never covers more than a quarter of the window.** The scale
  steps down, and then components give way in the order below, to keep that.
- **The smallest supported window is 1280 by 720.** At that size, at scale 1,
  every component is shown in full. This is a target until stage 1 gives the
  real sizes of the sprites; it is then checked by adding up the native widths
  of the bottom row and the native heights of the tallest column, and the
  figure is corrected here if they do not fit.
- **The native size of every component is written into this table** when it is
  first laid out in stage 3, so the smallest window and the scale steps are
  measured facts and not estimates:

| Component | Native width | Native height | Measured in |
|---|---|---|---|
| (filled in at stage 3) | | | |

### When the window is too small

At scale 1, if the components still do not fit, they give way in a fixed
order, least important first, and each one comes back as soon as there is room:

1. Target panel
2. Objectives collapses to its header line
3. Team frames collapse to portraits and health bars
4. Chat and event log collapses to its last line
5. Score strip
6. Inventory

The ability bar and the minimap are never hidden. Below the size at which those
two fit side by side, the window refuses to shrink further.

Proportions in the reference table are the target at the scale the window
picks; whole-number scaling means a component's share of the screen will move a
little between window sizes. That is accepted. Stretching to hold the share
exactly is not.

## Stages

Each stage has a check that can fail. A stage is not done until its check has
been run on the real cache, on the user's machine, and the result looked at.

### Stage 1: cache decoding

Decoders for sprites and bitmap fonts in `crates/cache`, and a dump tool in
`crates/tools`.

Hand-built samples test that the decoder does what its author thinks the
format is. They do not show that the author is right about the format. Only
the real cache does that, so the dump tool is the check:

- **Sprites.** For a given range of ids, write every frame of every sprite as
  a PNG, and a report with, per sprite: the frame count, each frame's width,
  height and offsets, the sheet's full size, the palette size, and how many
  pixels are transparent.
- **Transparency.** PNGs are written with real transparency, and a second
  copy over a magenta ground, so a wrongly opaque or wrongly clear pixel shows.
- **Palette.** The report lists each palette as colour swatches in a PNG, so a
  channel-order mistake (red and blue swapped) is visible at a glance.
- **Frames and variants.** A sprite with several frames is written as separate
  files and as one strip, so their alignment against each other can be seen.
- **Fonts.** For each font: a sheet of every glyph in a grid; a report of each
  glyph's width, height, offsets and advance; and a test line ("The quick
  brown fox 0123456789 Oh dear, you are dead!") drawn with the decoder's own
  spacing over a drawn baseline, so wrong advances or a wrong baseline show as
  crooked or cramped text.

**Check:** the user runs the tool; the panels, buttons, icons and letters are
recognisable and clean; the test lines sit on their baselines. Any sprite that
is torn, sheared, wrongly coloured or wrongly transparent fails the stage.

Until that check passes, the decoders are **untested against real data**,
whatever their unit tests say.

How to run it (the output folder is kept out of the repository, under
`.cache/`, because what it writes is the game's own artwork):

```
cargo run -p rift-tools -- dump-fonts .cache/interface_dump
cargo run -p rift-tools -- dump-sprites .cache/interface_dump --overview-only
cargo run -p rift-tools -- dump-sprites .cache/interface_dump --from 0 --to 400
```

`dump-fonts` writes, for each font, its letters in a grid, a test line at
its own size and three times larger, and two reports. `dump-sprites` writes
an overview sheet for every hundred ids, with each sprite's id, size and
frame count over magenta, and a report; without `--overview-only` it also
writes every frame clear and over magenta, a strip of every sheet with
several frames, and every palette as swatches. The report's "unread" column
counts bytes the decoder found between the last frame and the palette: a
right reading leaves none.

**Passed.** Run on the real cache (revision 235) on the user's machine:

- All 7,318 sprites read, 13,304 frames, none failed and none left a byte
  unread. Both ways of storing a frame occur (8,523 by row, 4,781 by
  column), and 37 frames carry their own opacity.
- All 21 fonts read, each with 257 bytes of measurements and 256 letters.
- Looked at, as a sample and not all of it: 9 of the 21 test lines, 3 of the
  91 overview sheets, all 6 strips written for sprites 420 to 445 and their
  palettes, 19 frames over magenta, and 14 of the 37 frames with their own
  opacity. Nothing was torn, sheared, wrongly coloured or wrongly
  transparent; every line stood on its baseline.
- Running it found a fault that was not in these decoders: the cache reader
  refused every group stored uncompressed, which is how the 848 smallest
  sprites are stored. That is fixed and tested, and the game's scene came
  out the same after it as before.

Left as an uncertainty, not a failure: 21 of the 37 frames with their own
opacity are wholly clear. They are taken to be sprites the game's makers
blanked. The other 16 are read by the same code and look right, and none of
the 21 is anything the interface needs.

Found in the dump, for the stages after: fonts 494 (small), 495 (regular),
496 (bold), 497 (the one "Oh dear, you are dead!" is written in); a stone
frame in pieces at 913 to 936 and 952 to 997; stone buttons at 170 to 191;
spell icons at 15 to 64, prayer icons at 115 to 134, equipment slots at 156
to 166.

### Stage 2: interface renderer

The drawing layer of rule 3: its own pass after the scene, an atlas built from
the theme's images, nearest-pixel sampling, see-through drawing, clipping,
layer order, and the scaling rule.

**Check:** a test screen showing one sprite at scales 1 to 4, a nine-slice
panel at three sizes, each font's test line, a see-through rectangle over the
scene, and a clipped panel. Resizing the window keeps every edge hard and
every piece pinned where it should be, and steps the scale at the right sizes.

**As built.** Three parts, each knowing less than the one after it:

- `crates/ui` is the drawing layer. It has no dependencies: not the cache,
  not the graphics card. It is given a **theme** (plain images, tables of
  letters and colours, each under a role) and turns requests for a sprite, a
  panel, a line of text or a rectangle into quads over one atlas, clipped on
  whole pixels and sorted by layer. It also holds the scale rule and the
  test screen.
- `crates/client/src/ui_pass.rs` is the pass: one pipeline, drawn after the
  scene with no depth, reading the atlas by whole pixel and laying each quad
  over the scene by its opacity. It is given the atlas and the quads and
  knows nothing of what they show.
- `crates/client/src/theme_file.rs` reads `data/ui/theme.json`, which names
  each role and says which cache sprite or font it is, and builds the theme
  from the cache. It is the only code that sees both a role and an id.
  Replacing the art is another theme file and, for art that is not in the
  cache, another loader; nothing in the other two parts changes.

Clipping is done on the quads themselves and not by the graphics card, so
it is exact, costs no extra draws, and can be tested without one. Corners
of a panel are kept and its edges and middle are repeated; where an area is
too small for both corners, each keeps its outer half.

**How it is tested without a graphics card.** A reference rasteriser in
`crates/ui` draws the quads the way the pass is asked to. Against plain
arithmetic for every pixel: a sprite at scales 1 to 4; clipping at
rectangles and at every edge of the window; clips inside clips; layer
order; opacity; a panel at six sizes and four scales, including sizes that
are not whole multiples of its middle and one barely bigger than its
corners; text on its baseline, in its colour, at its advances. The test
screen is built at ten window sizes with every corner on a whole pixel
inside the window, its pins are shown to move with the window's edges, and
two themes with the same roles are shown to lay out identically.

The rasteriser mixes colours as stored bytes and the graphics card mixes
light, so a see-through edge will be a shade different on screen. That, and
everything in `ui_pass.rs`, is what only the running game can show.

**How to run the check:**

```
cargo test -p rift-ui
cargo run -p rift-client --profile quick -- --champion monkey --ui-test
```

The test screen is drawn over the game, which goes on as usual underneath.
Its first line says the window's size and the scale chosen, and the same is
written to the log each time either changes. The scale steps at whole
multiples of 640 by 360: 2 from 1280 by 720, 3 from 1920 by 1080, 4 from
2560 by 1440. `--ui-scale 1` to `4` forces a scale. To pass, with the
window resized through those sizes and dragged smaller than 640 by 360:

- the four sprites are 1, 2, 3 and 4 times one size, with hard edges;
- the three panels have identical corners and unbroken edges;
- every font's line stands on its red baseline;
- the scene shows through the dark, blue and red rectangles, and red is
  over blue;
- the panel inside the yellow frame stops at the frame on both sides;
- the pieces marked TR, BL, BR, T and B stay the same distance from their
  edges, and the pieces hanging off the right, bottom and top left are cut
  at the window's edge;
- nothing shimmers, blurs or moves by a part of a pixel while resizing.

The old interface in `hud.rs` is untouched and still drawn.

**Passed.** Run on the user's machine (Intel HD 620, DirectX 12, a 1920 by
1080 screen at 175% scaling), with the test screen over a live match:

- The client compiled and the pass ran at the first attempt. The theme was
  made from the real cache: one atlas of 256 by 256, four fonts, a panel
  and a sprite. The game's scene and play were unchanged under it.
- Full-size screenshots at scales 1, 2, 3 and 4. At 2, 3 and 4 the inside of
  the largest panel was measured: every block of 2 by 2, 3 by 3 and 4 by 4
  screen pixels (11,050, 11,220 and 11,050 blocks) is one colour, so each
  pixel of the atlas is exactly one square of the scale's size. Scale 1 was
  judged by eye, enlarged.
- At every scale: the three panels have the same corners and unbroken
  edges; every font's line stands on its baseline; the clipped panel and
  its writing stop at the frame; the scene, including a tower, a crystal
  and units, shows through the see-through rectangles, with red over blue.
- The window was dragged through about 170 sizes at a forced scale of 2,
  from 2240 by 1260 down to 593 by 368 and back, with pieces clipped away
  and returning and nothing lost. The pinned pieces were at their corners
  and edges in every screenshot, at seven window sizes.
- Left to itself the scale followed the window: 3 at 2240 by 1260, 2 at 1811
  by 1040, and, dragged smaller, 2 at 1531 by 729 and 1 at 1506 by 709, the
  step falling exactly where the height goes under 720.

Not done: scale 4 chosen by the window itself, which needs 2560 by 1440 and
more screen than the machine has; it was seen forced. The see-through
rectangles are a shade lighter on screen than in the reference rasteriser,
as expected, since the graphics card mixes light.

Known and left: the test screen's own layout crowds at scales 1 and 4 (the
row of sprites is drawn at fixed sizes and runs into the font lines), and
its first line is hard to read over the old health bars. Neither is the
renderer, and the test screen is not kept by stage 3.

### Stage 3: static HUD

Every component in its place with made-up values, laid out against the
reference table, including the collapse order for small windows.

**Check:** a screenshot beside the reference frame at the same size. Each
component's anchor matches; each component's share of the screen is within the
tolerance whole-number scaling allows; the hierarchy reads the same (what the
eye goes to first, second, third). Differences are listed, not waved through.

### Stage 4: live integration

Each component is connected to the match through `HudState`, one at a time,
and only when the system it shows exists. A component whose system is not built
stays on made-up values and is marked as such on screen in development builds,
so a placeholder is never mistaken for a working feature.

**Check, per component:** the value on screen changes when, and only when, the
match changes it; and with the component removed, the match plays out
identically.

## What this replaces

`crates/client/src/hud.rs` today draws coloured rectangles and a built-in
blocky font, all in one file, reading match values directly. It stays in place
and working until stage 3 is accepted, then components replace it one at a
time. At no point is the game left with no interface.

## Kept apart from this work

Ground shading, vision and collision are a separate track and are not waiting
on this document. Interface work does not start a stage while a rendering
check the user has asked for is still open.
