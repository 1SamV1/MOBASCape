# Tower destruction

Status: **design for review. Nothing in this document is built**, and no
physics library is added to the project by writing it.

Today a tower whose health reaches zero stops being drawn and the ground it
stood on opens. This document proposes that it comes down instead: the
spire breaks and topples, the stones scatter, and a ruin is left that stays
for the rest of the match.

## What the models are

Measured from the models themselves, at the size they are drawn.

**Tower (model 1761).** Twelve separate pieces:

| Piece | Size | |
|---|---|---|
| Mound | about 320 by 270 across, 138 tall | a broad, nearly level top with steep flanks |
| Spire | about 130 across, from 98 to 432 | stands in the middle of the mound |
| Ten loose stones | 35 to 95 across, 26 to 46 tall | lie round the mound's foot |

**Crystal (model 1747).** Eleven pieces: a gem shell of 33 faces, about 350
across and 526 tall, and inside it a small statue made of ten pieces (body,
head, arms, feet) about 100 across and 320 tall.

Neither needs new art for a first version: a collapse is these pieces moved
separately.

## One thing to decide first: the mound is the tower

At the last review it was agreed that the mound should remain as a small
obstacle. Measured, the mound is not small. It is the tower's whole
footprint: about nine map cells across, the same ground the standing tower
blocks today, because the spire stands inside it. A ruin that keeps the mound
as it is blocks exactly what the tower blocked, and bringing a tower down
would open nothing.

Three ways to have a ruin, with what each does to the lane:

| Ruin | What blocks | Effect |
|---|---|---|
| A. Mound as it stands | The same nine cells across | The lane is no wider after the tower falls |
| B. Mound sunk until its top is at step height | Nothing; all of it can be walked over | The tower's ground opens fully; the ruin is scenery |
| **C. Mound sunk part-way, top at about 80** | **A core about five cells across** | **The lane opens by half; a real obstacle remains; a climber can stand on it** |

**C is proposed.** It keeps the agreed idea (a fallen tower still shapes the
ground) at a size that is fair, and it gives the monkey a perch the others
do not have. The figure of 80 is a starting point; what matters is that the
ruin's size is chosen, not inherited from a model built for something else.

## The five states, and which of them the match knows

| | State | Who decides | What it is |
|---|---|---|---|
| 1 | Standing | the match | The tower as now |
| 2 | Damaged | the match (it is only health) | The same tower, drawn worn |
| 3 | The destruction event | the match | One tick. The tower becomes a ruin. |
| 4 | Falling | each client | Pieces coming down. Drawing only. |
| 5 | Ruin | the match | A fixed shape with fixed collision |

**The match goes from standing to ruin in one tick.** For everything it
measures there is no falling tower: on the tick of the event the tower's
collision is replaced by the ruin's, the tower stops attacking and being
attackable, and that is all. States 2 and 4 exist only on screen.

So nothing that happens while pieces are falling can affect a match. No
piece blocks a lane, pushes a unit, or is waited for. If a client drew no
collapse at all the match would be the same.

### What the event carries

- which structure
- the tick it fell
- **which ruin it became**: the name and version of a ruin shape (below)
- **which collapse to show**: a number picking one of the prepared collapses

Every player and spectator is told the same ruin and the same collapse.
Nobody works out the ruin from their own falling pieces, so it cannot depend
on whose machine finished first. A spectator who joins after the tower fell
is told only that the structure is that ruin, and sees it settled.

## The ruin is prepared in advance, not worked out during a match

A ruin is a small data file made once, offline, for each structure on a map:

- the mound, and how far it is sunk
- where each loose piece comes to rest (position and turn)
- a version, which changes whenever any of that changes

Its collision is read from that shape by the code that already reads a
tower's: the same rules, the same tests. The file is part of the map's data
and is the same on every machine by construction.

Two rules the preparing tool enforces, and refuses to write a file that
breaks:

1. **The ruin's blocked ground lies inside the tower's.** Then on the tick a
   tower becomes a ruin, no unit can be standing inside the ruin: wherever
   it is, it was outside the tower. Nothing is ever trapped or has to be
   moved.
2. **No piece that comes to rest outside that ground is taller than step
   height.** Scattered stones are walked over by everyone, as the stones
   round a standing tower are now.

And one it tests: every champion walks from every side of the ruin to every
other, by the walk-round test that already exists, before the file is
accepted.

Each tower stands on different ground, so each has its own ruin file:
resting places are worked out on the slope it actually stands on.

## How the pieces fall: three ways to do it

What has to be true of the falling, from the requirements set at review:

| | Requirement |
|---|---|
| R1 | Twelve tower pieces fall and settle |
| R2 | A crystal's shell breaks into shards (33 faces, grouped) |
| R3 | Pieces meet the ground and the structure's own base, and do not pass through either |
| R4 | A handful of ragdolls, later |
| R5 | Runs well on the target machine (Intel HD 620) |
| R6 | Every player and spectator sees the same event and ends with the same ruin |

The three ways:

| | Small solver of our own, run in the game | Rapier, run in the game | **Collapse prepared offline, played in the game** |
|---|---|---|---|
| What the game runs | A rigid-body solver we write | A full physics engine | A list of positions over time |
| R1, R2 | Possible; contacts and stacking are where home-made solvers jitter | Yes | Yes |
| R3 | We must write collision against ground and shapes ourselves | Yes, with triangle meshes | Checked when prepared; nothing to go wrong in a match |
| R4 ragdolls | No. Joints are a second large piece of work | Yes: hinge and ball joints exist | No. A ragdoll depends on the moment and cannot be prepared |
| R5 | Unknown until written | Unknown until measured | Certain: moving twelve pieces along stored paths costs nothing |
| R6 same ruin | Only if our arithmetic is made exact | Only with its determinism feature, and that must be proved across machines | By construction: the ruin and the collapse are data |
| Added to the game | A few hundred lines to maintain for ever | About 80,000 lines and its maths and collision libraries | A small player for stored paths |
| Main risk | Jitter, tunnelling, a maintenance burden for one effect | Size, build time, a second maths library version, unproven sameness | Less variety: only as many collapses as were prepared |

**The third is proposed for towers and crystals.** Their collapse does not
depend on anything that happens in the match except which tower and, at
most, a direction. So it can be worked out once, looked at, corrected by
hand if a stone lands badly, and then played identically for everyone. It
needs no physics in the game at all, which is the smallest system that meets
R1 to R3, R5 and R6.

It does not meet R4, and should not be made to. That is the honest dividing
line: **a tower's fall can be prepared; a body's cannot.** A ragdoll's fall
depends on where the champion was, how it was moving and what hit it, and
has to be worked out when it happens.

### Where that leaves the shared foundation

The tool that prepares collapses needs real physics to produce
convincing ones. That tool is where Rapier is proposed: as something the
tool uses on a developer's machine, not something the game contains.

- It is the first practical use of the library, on exactly the bodies and
  shapes in question, with nothing in the game depending on it.
- It answers by measurement the questions the audit could not: whether
  pieces rest without jitter, whether anything passes through the ground,
  how it behaves on our triangle shapes.
- When ragdolls are designed, the decision to put it in the game is then
  made on experience and not on a feature list. If it disappoints, the tool
  can be changed without touching the game or a single match.
- If it proves poor even for the tool, a small solver of our own is enough
  there, where a bad result is thrown away instead of shipped.

So there is one physics foundation, adopted in two steps: first where it
cannot hurt, then, if it earns it, where it can.

### Variety without losing sameness

Several collapses are prepared for each tower (proposed: four, the spire
going over toward each side) and the event says which. Identical for
everyone watching; different from the last tower. Which one is picked from
the match's own state at the tick of the event, so it is the same on every
machine and in every replay.

Small things added on top by each client (dust, chips, a shake of the
camera) may differ from machine to machine. They change nothing.

## What is seen

**Damaged.** Drawn from health alone, so it needs no event of its own.
Below two thirds, the stones round the foot are shown disturbed; below one
third the spire leans a few degrees. Proper cracks and falling chips need
particles and a way to mark the surface, which the client does not have; the
first version does without them and says so.

**Collapse, about two and a half seconds.** The spire breaks from the mound
and goes over. The loose stones are thrown outward and settle. The mound
sinks to its ruin height. Pieces play their prepared paths; at the end each
is exactly at its ruin position, so what is seen and what blocks agree.

**Ruin.** The sunk mound, the spire lying where it fell, the stones round
it. It stays for the rest of the match.

**Crystal.** The shell breaks into shards (its 33 faces grouped into about
eight by which touch), which fall away and lie round the base. The statue
inside is left standing, about 100 across, as the ruin: a small obstacle and
a landmark. Whether the statue then means anything in the match is a
question for the game's design, not this document.

## Splitting a model into pieces

A model is split into its separate pieces when it is loaded: corners at the
same position are treated as one, and triangles that share a corner belong
to the same piece. Pieces are numbered by the lowest-numbered corner in
each, so the numbering is the same every time and on every machine, and a
ruin file can refer to "piece 3" safely. A test fixes the count for the two
models measured: twelve and eleven.

## Stages of building, once approved

Each is tested before the next is begun.

1. **The ruin as a state of the match.** Standing or ruin per structure; the
   event; collision switched on its tick. The ruin here is simply the sunk
   mound with the other pieces removed. No falling yet. Tests: the ruin's
   ground lies inside the tower's; nothing is trapped; every champion walks
   round it; a late spectator is told the ruin.
2. **Pieces.** Models split into pieces and drawn piece by piece, each able
   to be moved on its own. Nothing looks different yet.
3. **The preparing tool.** Works out resting places and falling paths on each
   tower's real ground, enforces the two rules, writes the ruin files. This
   is where the physics library is first used and judged.
4. **Playing a collapse.** The client plays the prepared paths on the event.
5. **Damaged look, variety, the crystal.**
6. **Dust and chips**, when the client has particles.

## How it will be judged

By code:

- The match's state after a tower falls is identical whether or not any
  collapse is drawn.
- Two runs of the same match pick the same collapse for the same tower.
- Every ruin file passes the two rules and the walk-round test.
- At the end of a collapse every piece is at its ruin position exactly.

By the preparing tool, reported for each collapse it makes:

- every piece comes to rest, and stays at rest
- no piece ends below the ground or inside the mound
- no piece passes through the ground on the way

By hand, which decides it:

- A tower coming down looks like stone falling, on the target machine, at
  the frame rate the game runs at.
- The ruin reads at a glance as a fallen tower and not as a small tower.
- Nobody is ever stopped by something they cannot see, or walks through
  something they can.

## Open questions for review

- **The ruin's size:** A, B or C above, and if C, how tall.
- **Does a ruin block sight** when vision is built? Proposed: no.
- **The statue in a fallen crystal:** scenery, or something in the match?
- **Direction of fall:** one of four prepared, picked by the match. Should it
  instead follow where the last blow came from? That needs more prepared
  collapses for the same variety.
- **How long the collapse takes.** Two and a half seconds is proposed; it is
  only for the eye, so it can be whatever looks right.
- **Inhibitors that come back.** If a structure stands again, the ruin is
  replaced by the structure on a tick, and a unit may be standing where it
  rises. That needs a rule (push out to the nearest open ground, in a fixed
  order) before any structure is allowed to return.
