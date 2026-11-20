# Interface stage 3: the whole, and the final comparison

Status: **built and rendered headlessly; waiting for the reviewer's eye.**
The old `crates/client/src/hud.rs` is still the default and still runs the
match. This file lists what the assembled interface matches and where it
differs from the reference. It is evidence for the review, not a new design
and not a sign-off.

## The pictures

The ignored test
`the_whole_stage_three_can_be_written_for_the_eye` in
`crates/client/src/theme_file.rs` writes:

| Picture | State |
|---|---|
| `target/stage3-whole-1920.ppm` | the strained whole at 1920 by 1080, scale 1 |
| `target/stage3-whole-1366.ppm` | the same at 1366 by 768, scale 1, with collapse |
| `target/stage3-death-1920.ppm` | the sample death screen |
| `target/stage3-shop-1920.ppm` | the sample shop |

Run it with:

```sh
cargo test -p rift-client whole_stage_three -- --ignored --nocapture
```

The five reference screenshots S1 to S5 are not in the repository. The
comparison below uses the reference coordinates measured in
`docs/INTERFACE_STAGE3_PLAN.md`, which are good to about four pixels.

## Anchors at 1920 by 1080

| Piece | Built | Reference coordinate | Difference |
|---|---|---|---|
| Score strip | 693 to 1227, 10 to 48 | 692 to 1226, 10 to 48 | one pixel right |
| Kill feed | 1622 to 1908, 18 to 136 | one entry 1622 to 1908, 18 to 46 | four entries stacked at the plan's 30-pixel pitch |
| Team frames | 1728 to 1908, 168 to 356 | the same | none |
| Target panel | 8 to 340, 82 to 240 | the same | none |
| Objectives | 8 to 280, 410 to 712 (usual) or 808 (longest) | 8 to 280, 410 to 760 or 810 | our own lines; see below |
| Chat | 5 to 445, 930 to 1070 | the same | none |
| Ability bar | 731 to 1189, 968 to 1062 | 730 to 1188, 968 to 1062 | one pixel right |
| Inventory | 1494 to 1638, 902 to 1062 | 1408 to 1640, 838 to 1062 | six slots, planned |
| Minimap | 1646 to 1912, 800 to 1064 | the same | none |

At 1366 by 768 the placing pass takes the two steps the plan names:
the objectives to their title and the team frames to portraits. The usual
state covers 23.3%, no two pieces are within 8 pixels, and neither of those
two steps was forced by the other.

## What the eye goes to

1. The battlefield and its unit plates.
2. The ability bar and minimap.
3. The target panel, objectives and team frames.
4. The chat, score strip, kill feed and announcement.
5. The tooltip, when it is up.

This is the hierarchy `docs/INTERFACE.md` asks for.

## Differences, listed

1. **The inventory is six slots, not eighteen.** The reference has five
   equipment rows and eighteen cells; our item system has six items. The plan
   chose six in two rows of three and a smaller 144 by 160 panel. The
   reference's extra cells are not drawn, so nothing promises an item system
   we do not have.
2. **The objectives are our game's, not the reference's.** The panel has
   three lanes with two towers and an inhibitor a side, two nexuses and four
   camps. It has no dragon, TzKal-Zuk or mole. It also has no green team-state
   line under the team names, which cannot be read in the reference and would
   have to be invented. Its usual height is therefore 302 where the reference
   is 350; its longest is 398, against the reference's tallest state of about
   400.
3. **The tooltip is dark, not parchment.** This is the plan's change from
   `docs/INTERFACE.md`, made because S5 shows a dark tooltip. It is 310 wide;
   its height follows the wrapped passive text.
4. **The target panel and unit plates are samples.** The target is a
   structure, and the plates are placed at sample window fractions. The live
   selection, projection, fog and brush hiding are stage 4.
5. **The announcement's letter height is unconfirmed.** It is drawn in
   `font.title` at its own size. The reference's letters measure about 22
   pixels; the font's cell is 16 by 20. The plan records this as unsettled.
6. **The shop is the frame with our six items.** It has one category, no
   upgrade tree, and a Buy button that does nothing yet. The reference's long
   category list is its item system, not ours.
7. **The death scene is dimmed, not grey.** The reference draws the whole
   scene in grey and dark. Stage 3 uses the `dim` rectangle only; the grey is
   a later rendering task.
8. **No per-component "sample" marks are drawn.** Section 3 of the plan asks
   for a small mark on every component drawing made-up values. The assembled
   screen does not carry them; the `--ui-layout` header names the layout and
   the stage, and every sample is documented here. This is a difference for
   the reviewer to accept or reject.
9. **The old interface is still the default.** `--ui-layout` draws the new
   one over the game; `hud.rs` still runs the match. That stays so until the
   reviewer accepts the new interface.

## Not in this stage, by design

- Reading the match, buying or selling, sending chat, moving the camera,
  selecting a unit, or any command from the interface.
- A scoreboard or hero selection.
- Grey death rendering, a live cursor, or fading kill-feed entries.

## What acceptance needs

A reviewer's pass over the four pictures above beside S2, S3, S4 and S5, and
a decision on the differences in this file. No geometry clash is left at the
two supported sizes, and every figure in the plan's step B tests is produced
by the code rather than carried over from the Python script.
