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

Status: **design for review. Nothing in this section is built**, and none of
it is to be built until it has been reviewed.

Four things, in the order they depend on each other: a record of how tall
things are, climbing, hopping, leaping, and slopes. The aim is that a
champion's body decides how it crosses the map: what stops Jad should not
stop a monkey, and a hill should mean something different to each.

### What is being replaced: W today

Every champion's W is the same thing. It moves the champion instantly, in the
direction of the cursor, up to `w_distance` (768 units, six tiles) or to the
furthest open point short of that. It costs 30 mana, has an 8 second cooldown,
and passes through anything in between. It has no wind-up, no travel time, and
gives an opponent nothing to see or answer. It lives in the client, not the
simulation.

It stays as it is for every champion until that champion's own way of moving
is designed. This section proposes the monkey's.

### One number per cell: how tall the thing there is

Today the map records only whether a cell is blocked. It should record **how
tall the solid thing over each cell is**: the height of the highest point of
any drawn surface over that cell, counting only what is at or below head
height for scenery (so leaves do not count) and everything for a structure
(so nobody stands under a crystal's overhang). Open ground is 0.

The code that marks obstacles already samples those heights and throws them
away. Kept, they answer every question below with one rule each:

| Question | Rule |
|---|---|
| May a champion stand here? | The thing here is no taller than the champion's **step height** |
| How high is it standing? | As high as the thing here (drawing only) |
| May a leap pass over here? | The thing here is no taller than the leap's **clearance** |

Today's fixed step height of 48 becomes the default. Because obstacles are
widened by a body's width, there is one map per distinct step height in a
match, built when the match loads.

### Climbing

A champion file gains `step`: the tallest thing it can get on top of. Default
48. Proposed: monkey 96, Jad 48.

With 96 the monkey gets onto the lower two thirds of a tower's mound (138
tall), onto every loose stone, and onto the ruin of a fallen tower. It does
not get onto the spire, a trunk, or a crystal.

- **It is a rule of the match.** A climber really does stand where others
  cannot: closer to a tower, and with ground to retreat onto. That is worth
  something in a fight and has to be balanced as such.
- **Standing height is drawing only.** The simulation stays flat. A champion
  on a rock is hit, targeted and measured from the point on the ground under
  it. It is drawn as high as the thing it stands on, eased so it does not
  snap.
- **No climbing animation yet.** It walks up. That waits for the animation
  system.

### Hops

Nothing new in the simulation. A champion already walks over anything under
its step height. Drawn at the height of what it is on, eased with the same
spring as its footfalls, it visibly hops onto and off a stone as it crosses
it. Light champions with a large `bounce` make more of it; heavy ones tread
over it. It never changes speed or path, so it cannot be an advantage or a
penalty.

### The leap

The monkey's W. Pressed and aimed, a short committed arc that crosses what
others must walk round.

**What happens, in order**

1. **Crouch, 0.10 s.** The monkey stops and gathers. The landing point is
   fixed at this moment and shown on the ground to everyone who can see him.
   He cannot move, attack or cast.
2. **In the air.** He travels in a straight line over the ground from where
   he crouched to the landing point, at a steady rate. Time in the air is
   0.30 s plus 0.25 s for every 400 units, so the full 640 takes 0.70 s. He
   cannot steer, attack or cast.
3. **Landing.** He is on the ground at the landing point, facing the way he
   travelled, moving at walking speed in that direction. A drawn squash and a
   little dust; no pause.

**Where he lands**

- Range: at most 640 units (five tiles), at least 160. An aim nearer than the
  least does nothing and costs nothing.
- The line from crouch to aim is checked cell by cell. If any cell on it has
  something taller than the leap's **clearance of 150**, the landing point is
  pulled back to the last open cell before it.
- If the landing cell is not somewhere he may stand, it is pulled back along
  the line to the nearest that is.
- If that leaves less than the least range, the leap does not happen and
  costs nothing.

So he clears loose stones, the skirt of a tower's mound, a ruin, low walls,
and any gap between trees. He does not clear a tree's trunk, a tower's
spire, or a crystal; aimed across one, he lands short of it. A tower remains
a wall to leap round, not over.

**What others can do about it**

- He can be seen crouching and the landing point can be seen. The whole thing
  is fixed from the crouch: same range, same time, same line, every time.
- He can be targeted and hit throughout. Being in the air protects him from
  nothing. For anything that asks where he is, he is at the point on the
  ground under him.
- A stun or root that lands during the crouch cancels the leap, and the
  cooldown is spent.
- A stun or root that lands while he is in the air does not stop the arc. It
  takes effect when he lands, for its full length.
- A knockback in the air: open question, below.

**Cost.** 30 mana and an 8 second cooldown, as W has now, until it has been
played.

**The arc is drawing only.** How high he rises (a quarter of the distance,
no more than 180) and how he tumbles are for the eye. The simulation knows
only the line, the time, and that he is in the air.

### Slopes

Top speed changes with the steepness of the ground under a champion, along
the way it is heading. Steepness is rise over run of the drawn ground,
limited to half (a climb of one in two).

| | Uphill, per unit of steepness | Downhill, per unit of steepness |
|---|---|---|
| Lightest (weight 0) | 15% slower | 10% faster |
| Heaviest (weight 100) | 60% slower | 40% faster |

In between it runs evenly with weight. On a hill of three in ten, which the
map has plenty of, the monkey (weight 20) climbs 7% slower and descends 5%
faster; Jad (85) climbs 16% slower and descends 11% faster. Across a slope,
neither.

- **Only top speed changes** in the first version. Time to speed, time to
  stop and time to turn are as on the flat. Whether a heavy champion should
  also take longer to stop going downhill is an open question; it would feel
  right and it would make stopping distance depend on the ground, which is
  harder to read.
- **It is the same for everyone on the same ground**, and the ground does not
  change. A player can learn where the fast and slow lines are. Nothing is
  random.
- **It applies to walking only.** A leap covers its distance in its fixed
  time whatever is under it.

### Keeping it the same on every machine

- Everything above that decides where a champion is lives in the simulation
  and uses whole numbers: the heights of things, the line of a leap, its
  time, the steepness of the ground.
- The ground's heights, which today only the client holds, are given to the
  simulation as the whole numbers they come from in the map.
- Times are held in milliseconds and counted in ticks. At 30 Hz a leap's
  phases land on tick boundaries to within one tick (33 ms); at 60 Hz, 17 ms.
  A leap covers the same distance to the same point at any tick rate; its
  duration can differ by at most one tick.
- The drawn height, arc, squash and dust are in the client and can differ
  between machines without changing anything that matters.

### What has to exist first

1. The height of the thing over each cell, kept in the walkable map.
2. The ground's heights in the simulation.
3. A champion's state in the simulation able to say "crouching" and "in the
   air", with where from, where to and how far along. The leap is the first
   ability to move out of the client, and it sets the pattern for the rest.
4. W read from the champion file, so the monkey leaps and the others keep
   what they have.

### How it will be tested

In the simulation, by code:

- A champion stands where the thing is no taller than its step and nowhere
  taller; two champions with different steps disagree about the same rock.
- A leap aimed at open ground lands on the aim, in the stated time, from any
  direction.
- A leap aimed across a trunk, a spire and a crystal lands short of each; one
  across a stone, a mound's skirt and a ruin does not.
- A leap never ends where the champion may not stand, for every aim round a
  ring, at every range.
- The same leap at 20, 30, 60 and 120 Hz ends at the same point, within one
  tick of the same time.
- Uphill, downhill and across the same slope give the stated speeds for a
  light and a heavy champion; on the flat nothing changes.
- The walk-round tests still pass with climbing on.

By hand, which is what decides it:

- The leap feels like a jump and not a teleport with a delay.
- An opponent watching can tell where he will land.
- The numbers above (step 96, clearance 150, range 640, the times, the slope
  figures) are starting points. Each is tuned in play and the tuned value
  written back here.

### Open questions for review

- **Knockback in the air.** Does a knockback move the landing point, or wait
  until he lands? Moving it is more interesting and breaks "the landing point
  is fixed". Proposed: it waits.
- **Leaping onto things.** May he land on a rock he could stand on? Proposed:
  yes, since landing only needs somewhere he may stand.
- **Leaping over units.** Units do not collide yet. When they do, proposed:
  he passes over them and cannot land inside one.
- **Vision.** When vision is built, does being in the air reveal him over
  brush? Proposed: no change in the first version.
- **Slopes and stopping**, as above.
- **Minions and slopes.** If slopes slow champions and not minions, waves
  arrive differently from champions on hilly lanes. Proposed: the same rule
  for every unit once minions are in the simulation.
- **Who else gets a step above the default.** Only the monkey is proposed.
  Each champion's is a design decision of its own.

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
