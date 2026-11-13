# Interface

Status: **design proposed. Nothing in this document is built.** It is written
to be reviewed before any code, because the work replaces most of
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

### What is not known yet

- The exact contents of the inventory panel. The measured frame shows a grid of
  equipment-shaped slots above a row of item slots, a gold count, and Shop and
  Teleport buttons. Slot counts are to be read from clearer frames, not
  assumed.
- Every size smaller than about six pixels in the recording.
- Which cache sprites the reference uses for each panel. This is found by
  looking at the sprite dump (stage 1), not by guessing ids.
- How the reference behaves at other window sizes. Only one size was recorded.

## Rules

### 1. The interface shows the match; it does not run it

A component reads a snapshot of match state and draws it. It never holds a
timer, a cooldown, a gold count or a spawn rule of its own. The objectives
panel shows the time until a boss appears because the match says so; it cannot
make one appear.

Concretely there is one plain struct, `HudState`, filled once a frame from the
match, and each component takes only the part it needs. What the player does to
the interface (a click on an item slot, a key for an ability) leaves as a
*command* to the match, the same command a network client would send. No
component changes match state directly.

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

### Stage 2: interface renderer

The drawing layer of rule 3: its own pass after the scene, an atlas built from
the theme's images, nearest-pixel sampling, see-through drawing, clipping,
layer order, and the scaling rule.

**Check:** a test screen showing one sprite at scales 1 to 4, a nine-slice
panel at three sizes, each font's test line, a see-through rectangle over the
scene, and a clipped panel. Resizing the window keeps every edge hard and
every piece pinned where it should be, and steps the scale at the right sizes.

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
