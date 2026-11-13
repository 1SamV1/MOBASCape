# Movement

Status: **design proposed; locomotion implemented for the player's champion.**

Built so far (`crates/sim/src/motion.rs`, `crates/client/src/movement.rs`):
turn time, acceleration time, stopping time, the slow-while-turning rule, the
30 Hz integer tick with drawing between ticks, and a walk cycle driven by
distance. An attack now waits until the champion has turned to face its
target.

Also built: a hidden `weight` per champion that sets the timings, and a
drawing-only spring in the step of light champions.

Not built yet: attack wind-up, live reload of champion files, the `--lab`
mode, recording and replay, and any of this for minions.

Moving a champion is what a player does every second of a match. If it does
not feel good with nothing else on screen, no later system will fix it. This
document says what "feels good" is made of, where the values live, and what
has to be true before movement counts as done.

The target is **weight**: champions are large creatures that take time to
turn, to get going and to swing. Each champion should feel different to move.

## Where movement stands today

- A unit moves at `speed * dt` along its path (`follow_path` in
  `crates/client/src/animated.rs`). It is at full speed on the first frame and
  stops dead on the last.
- Facing snaps to the direction of travel in the same frame
  (`unit.yaw = dir.x.atan2(dir.z)`). There is no turning.
- The walk animation advances with wall-clock time, not with distance
  covered, so feet slide whenever speed changes.
- A champion file (`data/champions/*.json`) carries one movement value,
  `speed`. Jad at 380, the monkey at 420 and Kree at 450 all move the same
  way, at slightly different speeds.
- Movement runs on the render loop in floating point, so it is not the same
  at 30 fps and 144 fps, and cannot be replayed.

## What weight is made of

Six settings. Each is a property of the champion, not of the engine.

| Setting | What it does | What the player feels |
|---|---|---|
| Turn time | Seconds to turn 180 degrees | Mass. The single biggest lever. |
| Acceleration time | Seconds from standing to full speed | Effort to get going |
| Stopping time | Seconds from full speed to standing | Momentum |
| Move speed | World units per second at full speed | Pace |
| Attack wind-up | Seconds from attack start to the hit | Commitment |
| Stride length | World units covered per walk cycle | Feet that grip the ground |

Two rules tie them together:

- **A champion moves while it turns, but slower the further it is from facing
  its direction of travel.** Speed is scaled by how well facing and direction
  agree, with a floor so a click always produces motion at once. A heavy
  champion asked to reverse therefore swings round in an arc; a light one
  pivots.
- **The walk animation is driven by distance, not time.** One cycle plays per
  stride length covered. Accelerating, slowed and hasted champions all keep
  their feet planted.

## Per-champion data

New fields in `data/champions/<name>.json`, beside the existing `speed` and
`attack_interval`:

```json
{
  "speed": 350,
  "turn_time": 0.45,
  "accel_time": 0.30,
  "stop_time": 0.20,
  "attack_windup": 0.50,
  "stride": 240
}
```

Starting values for the three champions that exist. These are places to begin
tuning, not measurements, and they are meant to be changed by playing.

| | monkey | jad | kree |
|---|---|---|---|
| Role in the set | light | heavy | fast flyer |
| `speed` | 420 (as now) | 350 | 450 (as now) |
| `turn_time` | 0.15 | 0.45 | 0.25 |
| `accel_time` | 0.10 | 0.30 | 0.20 |
| `stop_time` | 0.05 | 0.20 | 0.25 |
| `attack_windup` | 0.25 | 0.50 | 0.30 |
| `stride` | default | default | default |

Left out, `stride` defaults to top speed times the length of the walk cycle,
which plays the walk at its authored rate at top speed. Set it only where a
champion's feet visibly slide.

The point of three is contrast. If two of them feel alike, one is wrong.

## Weight

One number a player never sees: `weight`, from 0 (feather) to 100 (colossus).
It sets turn, acceleration and stopping time wherever a champion's file leaves
them out:

| `weight` | Turn | Acceleration | Stop |
|---|---|---|---|
| 0 | 0.06 s | 0.04 s | 0.005 s |
| 20 (monkey) | 0.15 s | 0.10 s | 0.05 s |
| 42 (kree) | 0.25 s | 0.17 s | 0.25 s, set by hand |
| 85 (jad) | 0.45 s | 0.30 s | 0.20 s |
| 100 | 0.52 s | 0.35 s | 0.235 s |

A new champion needs one value to feel right. Any of the three timings can
still be written into the file to break the pattern, as Kree's long glide to a
stop does. A file with neither a weight nor timings moves instantly, as every
champion did before.

Weight is also the number later systems should read: how far a knockback
throws a champion, who gives way when two collide, how much dust a stop kicks
up.

## Two layers

1. **The simulation decides how a champion moves.** Position, facing, speed.
2. **Drawing gives it a personality.** Nothing in this layer can change where
   a champion is, what it hits or what hits it.

The second layer today is spring. A light champion hops on each footfall,
takes a small dying bounce as it stops, and floats slowly at rest; a heavy one
stays planted. The amount follows from weight (lightness squared), so the
monkey rises about 8 world units in a step and Jad about a third of one. Its
selection ring and bars stay on the ground, which is what makes the body read
as lifted. `"bounce"` in the champion file scales it: 0 turns it off.

The agile and the heavy should be opposites on both layers: the monkey quick
to pivot and barely touching the ground, Jad deliberate and rooted.

## Rules for the implementation

1. **Movement runs in the simulation tick,** at 30 Hz, in integer or
   fixed-point units, from commands. This is step 1 of the path in
   SPECTATING.md; it is the same piece of work.
2. **One movement function serves tap-to-move and hold-to-steer** (ADR-005).
   They differ in where the desired direction comes from, not in how the
   champion responds to it.
3. **The first response is immediate.** On the tick a command arrives, facing
   starts to turn and the champion starts to move. Weight comes from how long
   the response takes to complete, never from a delay before it begins.
4. **The client draws between ticks.** Position and facing are interpolated,
   so 30 Hz simulation looks smooth at any frame rate.
5. **Modifiers multiply, they do not replace.** Buffs, slows and items scale
   `speed`; they never write over a champion's own values.
6. **No value is typed into code.** Every number above lives in the champion
   file and reloads while the game runs.

## The tick rate is not baked in

The simulation runs at 30 ticks a second today (ADR-009). That number is one
constant, `TICK_HZ` in `crates/sim/src/motion.rs`. Every value a champion
carries is in seconds or in units per second and is converted there, so the
rate can be changed without touching the movement model or any champion file.

Measured, the same heavy champion (speed 350, turn 0.45 s, acceleration
0.30 s, stop 0.20 s) at four rates:

| Rate | To top speed | Half turn | Covered in 2 s | Carried through a reversal |
|---|---|---|---|---|
| 20 Hz | 0.35 s | 0.50 s | 656 | 21.8 |
| 30 Hz | 0.33 s | 0.47 s | 653 | 24.5 |
| 60 Hz | 0.32 s | 0.47 s | 650 | 27.4 |
| 120 Hz | 0.32 s | 0.46 s | 646 | 28.7 |

Timings agree to within a tick and distances to within 2%. The arc of a
reversal shifts by a few world units, because at a lower rate the champion
comes round in bigger steps. So a change of rate needs a light retune of turn
times, not a rewrite. `a_champion_moves_the_same_whatever_rate_the_simulation_runs_at`
in `crates/sim/tests/motion.rs` holds these bounds.

The rate itself is still an open decision. Thirty is the working value.

## The movement lab

A way to tune movement with nothing else in the way.

- `--lab`: a flat, empty area, one champion, no minions, towers, fog or HUD.
- The champion file reloads when it is saved.
- A readout shows current speed, facing error and the six values.
- Keys switch between the champions, so contrast can be judged in seconds.

Five drills, each done with every champion:

1. **Reverse.** Run, then click directly behind. Watch the arc.
2. **Zig-zag.** Click left and right of the path in quick succession.
3. **Stop and go.** Short clicks with pauses. Watch the start and the stop.
4. **Orbit.** Hold-to-steer in a circle round a point.
5. **Strike.** Walk up to a target dummy and attack; walk away mid-swing.

## Done means

- [ ] All six values are read from the champion file and reload live.
- [ ] Movement runs in the simulation tick and is identical at any frame rate.
- [ ] A recorded movement session replays to the same state hash.
- [ ] Feet do not slide: distance per walk cycle is within 10% of `stride` at
      full speed, half speed and while accelerating.
- [ ] The three champions are told apart by movement alone, with models
      hidden, by someone who was not told which is which.
- [ ] The five drills have been played with each champion and the values
      written back to the champion files.
- [ ] The person who owns game feel has played it and signed off.

Until every box is ticked, attacks, abilities and items are not built on top
of the new movement.

## Collision with the world

Status: **built, and checked by hand on the player's champion.**

What stands in a unit's way is read from the shape that is drawn. There is no
radius to guess.

- The walkable map has four cells to a game tile each way (32 world units a
  cell).
- Every solid thing (a tree, a tower, a crystal) is marked on it from its own
  triangles, turned and scaled by the function that draws it. Only the part of
  its surface between **step height** (48) and **head height** (160) blocks.
  A tree blocks at its trunk and not under its leaves; a leaning rock at its
  foot and not under its overhang; loose stones not at all.
- Every obstacle is marked wider by a body's half-width (28), so a unit can be
  treated as a point.
- What a thing's own walls close off counts as part of it. An arch stays open.
- Marking a shape is two steps: finding the cells its surface covers
  (`block_solid`), and finishing them, which closes what they enclose and
  adds the room for a body (`block_cells`). The height data finds its cells
  its own way and is finished by the same second step, so the two can be
  compared cell for cell.
- Each structure's ground is kept apart from the rest and given back when the
  structure falls.
- The ground itself blocks only where there is none. Hills and dips of any
  steepness are walked. A unit's height is read off the triangle that is
  drawn.
- A walker turns for its next waypoint early only if the way is clear, slides
  round what it cannot pass through, and slows for a goal too close to turn
  to. A test walks three champions from every side of an obstacle to every
  other: several thousand walks, all arriving.

Measured from the models themselves, for reference: the tower (model 1761 at
its drawn scale) is a mound about 320 by 270 across and 138 tall, with a
spire to 432 and ten loose stones 26 to 46 tall. The crystal (1747) is a gem
shell about 350 across and 526 tall with a statue inside.

Not built: units do not yet collide with each other, and minions do not use
the walkable map at all.

## Proposed: getting about on rough ground

Status: **approved and frozen. Stage 1 (height data) is built, has passed
and is frozen with it: what it blocks is not changed again unless a real
fault is found. Nothing else in this section is built.** No later stage is
authorised. Each needs its own go-ahead after the one before it has passed
its tests. A change to this section is a design change and is reviewed like
one.

It went through three drafts. The direction was approved at the first
review, the separated height data at the second, and at the third the three
points then left open: more than one surface or solid stretch in a cell, the
arc of a leap over uneven ground, and the standing angle measured on the real
tower. At the fourth review, after stage 1 was built against the real map,
five things that building it had shown were approved and written in: the
kerb, levels measured above each cell's own ground, an underside read on its
own, the crystal in place of the tower as the example of a cell with more
than one surface, and the split of the function that marks the walkable map.
They are marked *(fourth review)* where they appear. A fifth review approved
what the stronger comparison then showed: where the kerb is measured from,
that 48 itself is not stepped over, and that structures stay cut off at head
height for now. The collision system
that exists is not changed by this document.

What stage 1 may and may not do:

- It adds height data, and nothing else.
- What blocks a unit must stay exactly what blocks it today.
- No climbing, hop, leap or slope enters with it.
- The real tower is kept as a test for every later stage.

Five things, in the order they depend on each other: what the map knows
about height, climbing, hops, the leap, and slopes. The aim is that a
champion's body decides how it crosses the map: what stops Jad should not
stop a monkey, and a hill should mean something different to each.

### What is being replaced: W today

Every champion's W is the same thing. It moves the champion instantly, in the
direction of the cursor, up to `w_distance` (768 units, six tiles) or to the
furthest open point short of that. It costs 30 mana, has an 8 second cooldown,
and passes through anything in between. It has no wind-up, no travel time, and
gives an opponent nothing to see or answer. It lives in the client, not the
simulation.

### What the map knows about height

The first draft kept one number per cell, the height of the tallest thing
there, and asked it three questions. They are three different questions. The
tallest point over a cell cannot tell the top of a rock from the side of a
trunk, cannot hold a ledge with a roof over it, and says nothing about
whether a body gets from one cell to the next.

The map is still made from the triangles that are drawn, by the code that
exists. What that code keeps for each cell becomes separate things:

| Kept per cell | What it is | Where it comes from |
|---|---|---|
| **Ground** | The height of the ground itself. One number. | The map's own tile heights: whole numbers, the same on every machine |
| **Standing surfaces** | The heights above the ground of every surface a champion could stand on here. None, one, or several. | Triangles over the cell that face upward and are no steeper than the standing angle |
| **Solid stretches** | Every stretch of height in which a body is in the way here. None, one, or several, each with a bottom and a top. | Everything else, as set out below |

**A cell holds as many surfaces and solid stretches as the shape has there.**
Most cells have none: open ground is a ground height and nothing more. A cell
under a tree has one solid stretch for the trunk. A cell can have a ledge to
stand on, clear air above it, and a branch above that: one surface and two
solid stretches with a gap between, and the gap stays open. The real crystal
needs this *(fourth review)*: read whole, 2 of its cells have more than one
standing surface and 48 have more than one solid stretch, the gem's shell
above and the statue inside it below. The real tower, which an earlier
draft gave as the example, has no cell with two standing surfaces: its spire
is too steep to stand on anywhere. Stretches that touch or nearly touch are
joined; nothing else is.

**How a shape becomes solid stretches.** Each triangle over a cell is one of
three kinds, by which way it faces:

- *A top*: it faces upward. If it is no steeper than the standing angle it is
  also a standing surface.
- *An underside*: it faces downward.
- *A side*: it is steeper than the standing angle either way.

Under a top it is solid down to the first underside beneath it in that cell,
or to the ground if there is none: a rock is solid all the way down, a ledge
only as thick as it is. A side is solid over the heights it spans. That is
what gives a cell in the middle of a rock, which has no sides over it at all,
a solid stretch from the ground to the rock's top.

**An underside read on its own** *(fourth review)*, with no top above it in
that cell, is solid only over the heights the underside itself spans. Nothing
is guessed above it. A palm frond is modelled as a single sheet, and from
below only its underside is there to read: it is a sheet in the air, not the
foot of a column. The first build guessed that such an underside went on up
to head height, and walled off the ground under every low frond on the map.

**Which way a triangle faces** is worked out from its own three corners and
nothing else, by one rule for every model, so it cannot depend on the order
a model is read in. The models are drawn mirrored (their own up is down), and
the rule takes that into account once, in one place; the lighting already
relies on the same rule, and being lit from the right side is the evidence
that it is right. On the real tower it puts 15% of the surface facing
downward (the undersides of the stones and the mound's base) and the rest
upward or sideways, which is what a thing sitting on the ground should give;
reversed, it would be the other way about. A test fixes it: a closed box must
come out with its lid, and only its lid, as somewhere to stand.

Leaves and anything else above head height are left out for scenery, as now,
so a champion walks under a tree. For a structure nothing is left out, so
nobody stands under a crystal's overhang. (Not yet so: stage 1 had to block
exactly what is blocked today, and today a structure is cut off at head
height like everything else. Reading structures whole changes what blocks,
and is a change for a later stage to make and test.)

**Levels are heights above each cell's own ground** *(fourth review)*, not
heights in the world. The ground carries a champion up and down a hill
freely, as it does today, and a hill is never a step however steep it is.
Only what stands on the ground is stepped onto or stopped by.

**Where a champion is** therefore has a third part. As well as its position
on the flat it has a **level**: the height it is standing at, which is the
ground or one of the standing surfaces of its cell. Two champions can be in
the same cell at different levels, one under a ledge and one on it.

**Getting from one cell to the next** is not stored: it is a rule applied to
a champion at its level in cell A and a neighbouring cell B. Among B's places
to stand (its ground, unless that is inside something solid, and each of its
standing surfaces), the champion may step to one only if all of these hold:

1. The difference between its present level and that one is no more than the
   champion's **step height**.
2. The champion's body, standing there, from its feet to its head, meets none
   of B's solid stretches, nor those of any cell within a body's half-width
   of B.

**The kerb** *(fourth review)*. Anything solid that ends lower than the kerb
(48, today's step-over height) is stepped onto by every champion, whatever
its slope and whether or not it has a surface level enough to stand on: its
top counts as a place to stand, and it is never in the way. This is what the
walkable map does today with loose stones, and without it the height data
could not block the same cells. A champion's own step height reaches higher
than the kerb only by way of real standing surfaces.

The kerb is measured from the level the champion is walking at, the lower of
where it stands and where it would stand, and not from wherever its feet
happen to be put. Two kerbs are not one: a stone 34 high with another
leaning over it to 75 is closed to a walker on the ground, although each is
less than a kerb above the one before. And the walkable map stops a walker
at 48 exactly and lets it over anything less, so a step that reproduces it
reaches everything lower than 48 and nothing at 48.

If more than one place in B passes, it takes the one nearest its present
level, and the higher of two equally near. If none passes, it cannot go
there. The choice is the same on every machine.

Rule 1 is what the first draft lacked: two surfaces each within reach of the
ground are not thereby within reach of each other. A stone 40 high next to a
ledge 130 high is a step of 90, not two steps of under 96.

The walker and the pathfinder already ask "may I go from this cell to that
one"; today the answer ignores height. They would ask this rule instead.

A body's own height is, for now, the one head height the map already uses
(160). A height per champion, so that Jad cannot go where only something
short fits, is the same rule with a different number and is left for when
there is a champion who needs it.

### Climbing

A champion file gains `step`: the greatest difference in level it can take
in one stride. Default 48, today's fixed value. Proposed: monkey 96, Jad 48.

**Measured on the real tower.** The rules above were run on the tower's
model (1761, at its drawn scale) for three standing angles, with no solid
stretches counted, so these are the most a champion could reach:

| Standing angle | Step 48 (Jad) reaches | Step 96 (monkey) reaches |
|---|---|---|
| 30 degrees | The lowest stones only (26 high) | Most of the mound's top (128) |
| **45 degrees** | **Every loose stone (46); none of the mound** | **The whole top of the mound (138); not the spire** |
| 60 degrees | The whole top of the mound (138) | The same |

What the model turned out to be changes what climbing means here. The mound
is not a slope to get part-way up. It is a broad, nearly level top between
100 and 138 high, with steep flanks, a rim at 90 to 100 on its south and west
sides, and the spire standing in the middle. At 45 degrees:

- The flanks on the east side are sides, not surfaces: nobody walks up them.
- The top is one unbroken patch of standing surface, about nine cells
  across, not a scatter of odd cells.
- The monkey gets up where the rim is within 96 of the ground, or by way of a
  stone (a stone's top at about 50, then the rim at about 100). Where the rim
  is over 96 with no stone by it, he does not. So there are particular ways
  up, which can be learned.
- Jad gets onto the stones and no further.
- The spire's own faces count as surfaces at over 400, in the same cells as
  the mound's top at 130. Nobody reaches them: it is a step of nearly 300.

At 60 degrees the east flank becomes a ramp and Jad walks to the top, which
removes the difference the whole design is for. At 30 the monkey still gets
up but the top breaks into pieces. **45 degrees is proposed on that
evidence**, and is confirmed or changed when stage 2 puts the monkey on the
real thing: whether he can get on and off from every side he should, and
whether he is ever left on a cell he cannot leave.

A step of 96 sits close to the rim's height, so a few units either way
decides how many ways up there are. That is a number to tune by hand.

He would not get up a trunk or the spire, whose faces are sides, nor onto a
crystal.

The work is in three separate parts, built and judged separately:

1. **Simulation.** Whether a stride is allowed, and the level it ends at.
   This is a rule of the match: a climber really does stand where others
   cannot, and that is worth something in a fight.
2. **Drawing.** The champion is drawn at its level, eased so that it rises
   and falls over the length of the stride and never snaps. Nothing the match
   measures uses the drawn height.
3. **Animation.** A clamber where the rise is large. Not in the first
   version; it waits for the animation system, and until then a stride of 90
   upward will look like a glide. That is accepted for a first test and not
   for a finished champion.

Everything the match measures (range, targeting, hits) uses the champion's
position on the flat. Being on a rock does not put it out of reach.

### Hops

Nothing new in the simulation: a hop is the drawing of a small stride up and
down again. A light champion with a large `bounce` makes more of it; a heavy
one treads over. It never changes speed or path, so it is neither an
advantage nor a penalty.

### The leap

The monkey's W. Pressed and aimed, a short committed arc that crosses what
others must walk round.

**What happens, in order**

1. **Crouch, 0.10 s.** The monkey stops and gathers. The landing point is
   fixed at this moment and shown on the ground to everyone who can see him.
   He cannot move, attack or cast.
2. **In the air.** He travels in a straight line over the map from where he
   crouched to the landing point, at a steady rate. Time in the air is 0.30 s
   plus 0.25 s for every 400 units, so the full 640 takes 0.70 s. He cannot
   steer, attack or cast.
3. **Landing.** He stands at the landing point, at its level, facing the way
   he travelled and moving at walking speed in that direction.

**The arc is part of the match, and it is the arc that is drawn.** The first
draft gave the simulation a clearance of 150 and the drawing an arc up to 180
high, which could show him passing through something the match said he
cleared, or over something it said he did not. There is now one arc. The
simulation holds it in whole numbers; the client draws exactly it and may add
a tumble, a squash and dust, which change nothing.

**The arc is in true height, not height above the ground under him.** It is
fixed by two points and a peak:

- Where his feet leave: the true height of the place he crouched (its ground
  plus the level he stood at).
- Where his feet land: the true height of the landing place, likewise.
- Between them, a straight line in true height, with a smooth rise above it
  that is greatest half-way: a quarter of the distance over the flat, no less
  than 60 and no more than 180.

So a leap uphill climbs to its landing and a leap downhill drops to it; a
leap onto a rock ends at the rock's top; and the arc does not ripple with the
ground it passes over. What the ground does in between matters only as
something to clear.

**What he clears** follows from the arc and his body, with no separate
number. His body in the air is an upright shape as wide as a body and as tall
as a body, with its foot on the arc. At every point of the arc, in every cell
that shape stands over, in true height:

- it must not meet any solid stretch of that cell, and
- its foot must not be below that cell's ground.

The second matters as much as the first: a rise in the ground between him
and his aim is something to clear like any rock, and a leap from one side of
a hill to the other lands short if the hill is taller than the arc. So:

- Near the middle of a long leap he passes over a stone, a mound's shoulder,
  a ruin, a low wall.
- Near either end he is low, and will not clear what he would at the middle.
- A trunk, a spire and a crystal reach higher than any arc. Aimed across one,
  he lands short of it. A tower remains something to leap round.
- A short leap has a low arc and clears little. It is a hop, not a way over a
  wall.

**Where he lands**

- Range: at most 640 units (five tiles), at least 160. An aim nearer than
  the least does nothing and costs nothing.
- The landing point must be somewhere he may stand: open ground, or a
  standing surface, with room for his body there. He may land on a rock. He
  need not be able to walk up to where he lands: the arc is the approach, and
  the arc being clear is what makes it legal.
- If the aim fails either test, the landing point is drawn back along the
  line to the furthest point that passes both. If that is nearer than the
  least range, the leap does not happen and costs nothing.

**Once he has left the ground the leap is settled.** The line and the landing
point were tested when he crouched and are not tested again in the air.

- *Something comes to stand on the line while he is in the air.* He passes it.
- *The landing point stops being somewhere he may stand* (a structure rises
  there; later, another unit stands there). On the tick he lands he is put at
  the nearest cell where he may stand, searched outward in a fixed order so
  that every machine picks the same one. A leap never ends inside anything
  and never fails to land.
- *He dies in the air.* The leap ends on that tick. For everything the match
  measures he died at the point on the flat under him, and that is where his
  body lies and where anything that depends on where he died takes place.
  The body falling from the arc is drawing only.

**What others can do about it**

- He is seen crouching, and the landing point is seen. Range, time, line and
  arc are the same every time.
- He can be targeted and hit throughout. Being in the air protects him from
  nothing. For anything that asks where he is, he is at the point on the flat
  under him.
- A stun or root that lands during the crouch cancels the leap, and the
  cooldown is spent.
- A stun, root or knockback that lands while he is in the air does not change
  the arc. It takes effect when he lands, in full. The landing point shown to
  everyone stays true.

**Cost.** 30 mana and an 8 second cooldown, as W has now, until it has been
played.

### Where abilities live

The leap runs in the simulation. Every other champion's W would still run in
the client. That is a state to pass through, not the design. In a match
between players, **every ability that moves a champion is decided by the
simulation**, because a client that moves its own champion can move it
anywhere.

So the leap is followed, before any other champion's W is redesigned, by
moving today's instant W into the simulation exactly as it behaves now. The
other champions keep the ability they have; what changes is who decides it.
New designs for their movement come later, one champion at a time.

### Slopes

Top speed changes with the steepness of the ground under a champion, along
the way it is heading.

| | Uphill, per unit of steepness | Downhill, per unit of steepness |
|---|---|---|
| Lightest (weight 0) | 15% slower | 10% faster |
| Heaviest (weight 100) | 60% slower | 40% faster |

In between it runs evenly with weight. On a hill of three in ten the monkey
(weight 20) would climb 7% slower and descend 5% faster; Jad (85) 16% slower
and 11% faster. Across a slope, neither. **These figures are a place to start
and are not approved**; they are settled by playing the same routes with a
light and a heavy champion.

Two constraints on how it is built:

- **Steepness is measured so that it cannot flicker.** Not from the one
  triangle underfoot, which changes at every tile edge, but from the ground's
  height a fixed distance ahead and the same distance behind along the
  heading (proposed: 96 units each way, three quarters of a tile), limited to
  one in two. And a change in top speed is not applied at once: it is a new
  top speed, which the champion reaches through its own acceleration and
  stopping times like any other. Speed never steps.
- **The heights are the map's own.** The simulation works steepness out from
  the tile heights in the map data, as whole numbers. It never asks a
  client's renderer how high the ground is, and no two machines can disagree.

Only top speed changes in the first version; time to speed, to stop and to
turn are as on the flat. It applies to walking only: a leap covers its
distance in its fixed time whatever is under it. It is the same for everyone
on the same ground, and the ground does not change, so the fast and slow
lines can be learned.

### Keeping it the same on every machine

- Everything that decides where a champion is lives in the simulation and
  uses whole numbers: ground, standing surfaces, solid spans, levels, the
  line and arc of a leap, its time, the steepness of the ground.
- Times are held in milliseconds and counted in ticks. A leap ends at the
  same point at any tick rate; its phases can differ in length by at most one
  tick (33 ms at 30 Hz, 17 ms at 60 Hz).
- Easing, tumbling, squash and dust are in the client and may differ between
  machines without changing anything the match measures.

### The order of building, once approved

One step at a time, each tested before the next is begun. Climbing, hops, the
leap and slopes are not built together.

1. **Height data. Built and passed.** Ground, standing surface and solid span
   per cell, and the rule for stepping between cells, in the simulation. No
   champion behaves differently yet: with every step height at the kerb the
   map must block exactly what it blocks today, and a test says so. Nothing
   that moves uses the height data yet; the walkable map is still what
   blocks.
2. **Climbing.** `step` read from the champion file. The monkey and Jad
   tried by hand on the real tower, stones, trees and crystal.
3. **Drawn height and hops.** Checked for snapping, for floating, and for any
   advantage it was not meant to give.
4. **The leap**, in the simulation. Every direction, every kind of obstacle,
   every landing case, every interruption above, and every tick rate.
5. **Today's W for the other champions**, moved into the simulation unchanged.
6. **Slopes**, on their own. A light and a heavy champion on the same routes.
7. **The movement drills again**, with all of it on. Only then are the
   numbers judged.

### How it will be tested

In the simulation, by code:

- With every step height at the kerb, the new map blocks the same cells as
  the map that exists. The cells themselves are compared, one by one, never
  how many there are: the same number of cells in different places is a
  failure, and a test says so. Each shape is compared lying across the cells
  as it lies across the real map, and the test shapes are tried at several
  positions within a cell. At start-up the game makes the same comparison
  for every blocking shape on the map and says so in one line of the log.

  What each of these is evidence for is kept apart. The start-up line on the
  real map (319 shapes, 7,691 cells the same, none different) covers every
  shape where it really stands, but every structure on that map stands a
  whole number of cells from the map's corner, so it does not try a shape
  lying off the grid. That case is covered by the test shapes at six
  positions within a cell, and by the real tower and crystal models read at
  256 positions within a cell each, which is done from the exported model
  files and is not part of the tests that run with the game.
- A trunk has no standing surface however low it is sampled; a stone's top
  and a mound's shoulder have one.
- A closed box has its lid, and only its lid, as somewhere to stand, whatever
  order its triangles are given in.
- A cell with a ledge, clear air and a branch above keeps the air between
  them open: a body that fits stands on the ledge, one that does not is
  stopped, and a leap whose arc passes through the gap is clear.
- A cell in the middle of a rock is solid from the ground to the rock's top;
  a cell under a thin ledge is solid only for the ledge's thickness.
- A leap uphill, downhill and onto a rock leaves from and arrives at the
  right true heights; one aimed over a rise in the ground taller than its arc
  lands short.
- Two surfaces each within a step of the ground and more than a step from
  each other cannot be walked between.
- Two champions with different steps disagree about the same rock, and each
  agrees with itself from every direction.
- A leap over open ground lands on the aim, in the stated time, from any
  direction.
- A leap aimed across a trunk, a spire and a crystal lands short of each; one
  across a stone, a mound's shoulder and a ruin does not, when they are near
  the middle of the arc, and does when they are at its ends.
- A leap never ends where he may not stand, for every aim round a ring at
  every range, including when the landing point is taken away mid-air.
- Death in the air leaves him at the point under him on that tick.
- A stun in the crouch cancels; a stun or knockback in the air waits.
- The same leap at 20, 30, 60 and 120 Hz ends at the same point, within one
  tick of the same time.
- Walking a straight line over rolling ground, top speed never changes by
  more than a set small amount from one tick to the next.
- Uphill, downhill and across the same slope give the stated speeds for a
  light and a heavy champion; on the flat nothing changes.
- The walk-round tests still pass with climbing on.

By hand, which is what decides it:

- Climbing reads as climbing, or at least not as a fault, before there is an
  animation for it.
- The leap feels like a jump and not a teleport with a delay.
- An opponent watching can tell where he will land.
- Every number here (step 96, the arc's peak, range 640, the times, the slope
  figures, the angle a surface may slope and still be stood on) is tuned in
  play, and the tuned value written back into this document.

### Decided at the first review

| | Decision |
|---|---|
| Collision read from the drawn shape | Keep |
| A step height per champion; the monkey climbs | Keep |
| Hops as drawing only | Keep |
| A committed leap with a fixed, shown landing point | Keep |
| Knockback waits until he lands | Keep |
| He may land on a rock he could stand on | Keep, with the landing tests above |
| One height number for every purpose | Replaced: ground, standing surfaces and solid stretches, several to a cell where the shape needs them |
| Stepping between neighbouring cells checked by difference in level | Keep |
| Simulation, drawing and animation as separate parts | Keep |
| A leap clearance separate from the drawn arc | Replaced: one arc in true height, tested against his whole body and the ground |
| Fixed landing; death, stun, root and knockback rules in the air | Keep |
| The seven stages of building | Keep |
| The standing angle | 45 degrees proposed on the real tower's measurements; confirmed at stage 2 |
| The other champions' W in the client | Temporary; moved into the simulation at step 5 |
| The slope figures | To be tested and tuned, not approved |

### Decided at the fourth review, after stage 1

| | Decision |
|---|---|
| Stage 1, height data | Passed |
| The kerb: anything solid lower than 48 is stepped onto by anyone | Added |
| Levels measured above each cell's own ground | Added |
| An underside with no top above it is only itself | Added |
| The example of a cell with several surfaces | The crystal, not the tower |
| Marking the walkable map split in two: finding a shape's cells, and finishing them | Keep |
| The comparison with the walkable map | Of the cells themselves, at each shape's real position |
| Stage 2 | Not authorised |

### Decided at the fifth review

| | Decision |
|---|---|
| The kerb is measured from the level walked at, the lower of where the champion stands and where it would stand | Approved |
| What is lower than 48 is stepped over; what is 48 is not | Approved |
| Structures stay cut off at head height, as today | Approved as a limit of stage 1; reading them whole is a later change, tested on its own |
| Stage 1 and this account of it | Frozen |
| Stage 2 | Not authorised; to be proposed and reviewed on its own |

### Still open

- **The standing angle.** 45 degrees, on the measurements above; confirmed or
  changed at stage 2 with the monkey on the real tower.
- **The monkey's step.** 96 is close to the height of the mound's rim. Tuned
  by hand at stage 2.
- **Slopes and stopping.** Whether a heavy champion should also take longer
  to stop going downhill. Proposed: not in the first version.
- **Leaping over units.** Units do not collide yet. When they do, proposed:
  he passes over them and is never put down inside one.
- **Vision.** When vision is built, whether being in the air or on a rock
  lets him see or be seen over brush. Proposed: no change at first.
- **Minions and slopes.** Proposed: the same rule for every unit once minions
  are in the simulation, so waves and champions keep pace with each other.
- **A body height per champion.** Left until a champion needs it.
- **Who else gets a step above the default.** Only the monkey is proposed.

## Not in this document

Unit-to-unit collision, knockbacks, other champions' ways of moving, and
camera feel. Each builds on this and gets its own design.

## Open questions

- Whether a champion should stop completely to make a very sharp turn. The
  speed-scaling rule above says no; playing the heavy champion will show
  whether that is right.
- Whether attack wind-up can be cancelled by a move command, and until when.
- Flyers: whether `kree` should ignore stopping time in a way that reads as
  gliding.
